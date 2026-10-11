/*
 * [INPUT]: 当前用户进程的可执行路径/Node 启动脚本、Claude 应用标识与 Security 钥匙串接口。
 * [OUTPUT]: Claude Desktop 全部辅助进程与 Claude Code Node 入口编号、脱敏凭据发现及原生单条直接删除。
 * [POS]: 清理的 macOS 边界；只读取凭据属性，扫描逐查询禁止交互，删除允许系统授权，不读取密码、不保留恢复副本。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
#import <AppKit/AppKit.h>
#import <Security/Security.h>
#import <LocalAuthentication/LocalAuthentication.h>
#include <libproc.h>
#include <sys/sysctl.h>
#include <unistd.h>

static char *cleanResponse(NSDictionary *data) {
    NSData *json = [NSJSONSerialization dataWithJSONObject:data options:0 error:NULL];
    return strdup([[NSString alloc] initWithData:json encoding:NSUTF8StringEncoding].UTF8String);
}
static char *credentialError(OSStatus status, NSString *stage);
// ----- 辅助进程按所属应用标识识别；Node 只识别官方 CLI 脚本，不匹配用户参数 -----
static BOOL claudeNode(pid_t pid) {
    int mib[] = {CTL_KERN, KERN_PROCARGS2, pid};
    size_t size = 0;
    if (sysctl(mib, 3, NULL, &size, NULL, 0) != 0 || size > 1048576 || size <= sizeof(int)) return NO;
    size_t allocated = size;
    char *raw = calloc(1, allocated);
    if (!raw) return NO;
    BOOL found = NO;
    if (sysctl(mib, 3, raw, &size, NULL, 0) == 0) {
        int argc = 0; memcpy(&argc, raw, sizeof(argc));
        char *cursor = raw + sizeof(argc), *end = raw + size;
        cursor += strnlen(cursor, end - cursor);
        while (cursor < end && *cursor == 0) cursor++;
        for (int i = 0; i < argc && cursor < end; i++) {
            size_t length = strnlen(cursor, end - cursor);
            if (length == (size_t)(end - cursor)) break;
            NSString *arg = [[NSString alloc] initWithBytes:cursor length:length encoding:NSUTF8StringEncoding];
            cursor += length + 1;
            if (i == 0) continue;
            if ([arg hasPrefix:@"-e"] || [arg hasPrefix:@"-p"] || [arg isEqualToString:@"--eval"] || [arg hasPrefix:@"--eval="] || [arg isEqualToString:@"--print"] || [arg hasPrefix:@"--print="]) break;
            if ([arg hasPrefix:@"-"]) continue;
            found = [arg hasSuffix:@"/node_modules/@anthropic-ai/claude-code/cli.js"];
            break;
        }
    }
    memset(raw, 0, allocated); free(raw);
    return found;
}
char *rf_claude_pids(void) {
    NSMutableArray *pids = [NSMutableArray array];
    NSMutableDictionary *bundles = [NSMutableDictionary dictionary];
    int capacity = proc_listallpids(NULL, 0) + 64;
    if (capacity <= 64) return NULL;
    pid_t *all = calloc(capacity, sizeof(pid_t));
    if (!all) return NULL;
    int count = proc_listallpids(all, capacity * sizeof(pid_t));
    if (count <= 0) { free(all); return NULL; }
    if (count > capacity) count = capacity;
    for (int i = 0; i < count; i++) {
        struct proc_bsdinfo info;
        char path[PROC_PIDPATHINFO_MAXSIZE] = {0};
        if (all[i] <= 1 || proc_pidinfo(all[i], PROC_PIDTBSDINFO, 0, &info, sizeof(info)) != sizeof(info) || info.pbi_uid != getuid()) continue;
        if (proc_pidpath(all[i], path, sizeof(path)) <= 0) continue;
        NSString *file = @(path);
        NSString *name = file.lastPathComponent;
        BOOL match = ([name isEqualToString:@"node"] || [name isEqualToString:@"nodejs"] || [name isEqualToString:@"bun"]) && claudeNode(all[i]);
        NSRange marker = [file rangeOfString:@".app/Contents/"];
        if (marker.location != NSNotFound) {
            NSString *root = [file substringToIndex:marker.location + 4];
            NSNumber *known = bundles[root];
            if (!known) {
                NSString *identifier = [NSBundle bundleWithPath:root].bundleIdentifier ?: @"";
                known = @([identifier isEqualToString:@"com.anthropic.claudefordesktop"] || [identifier hasPrefix:@"com.anthropic.claudefordesktop."]);
                bundles[root] = known;
            }
            match = match || known.boolValue;
        }
        if (match) [pids addObject:@(all[i])];
    }
    free(all);
    NSData *json = [NSJSONSerialization dataWithJSONObject:pids options:0 error:NULL];
    return strdup([[NSString alloc] initWithData:json encoding:NSUTF8StringEncoding].UTF8String);
}
// ----- 只识别 Claude 的已知服务，发现阶段不读取密码 -----
static BOOL credentialService(NSString *service) {
    return [service isEqualToString:@"Claude Code-credentials"] ||
        [service hasPrefix:@"Claude Code-credentials-"] ||
        [service isEqualToString:@"Claude Safe Storage"];
}
char *rf_credentials_list(void) {
    LAContext *context = [LAContext new];
    context.interactionNotAllowed = YES;
    NSDictionary *query = @{(__bridge id)kSecClass:(__bridge id)kSecClassGenericPassword,
        (__bridge id)kSecMatchLimit:(__bridge id)kSecMatchLimitAll,
        (__bridge id)kSecReturnAttributes:@YES, (__bridge id)kSecReturnPersistentRef:@YES,
        (__bridge id)kSecUseAuthenticationContext:context};
    CFTypeRef raw = NULL;
    OSStatus status = SecItemCopyMatching((__bridge CFDictionaryRef)query, &raw);
    if (status == errSecItemNotFound) return cleanResponse(@{@"items":@[]});
    if (status != errSecSuccess) return credentialError(status, @"无法检查钥匙串属性，未读取凭据内容");
    NSArray *rows = CFBridgingRelease(raw);
    NSMutableArray *items = [NSMutableArray array];
    for (NSDictionary *row in rows) {
        NSString *service = row[(__bridge id)kSecAttrService];
        NSData *reference = row[(__bridge id)kSecValuePersistentRef];
        if (credentialService(service) && reference) {
            [items addObject:@{@"service":service,@"reference":[reference base64EncodedStringWithOptions:0]}];
        }
    }
    return cleanResponse(@{@"items":items});
}
static char *credentialError(OSStatus status, NSString *stage) {
    NSString *reason;
    if(status==errSecUserCanceled)reason=@"已取消系统授权；余下凭据未处理。";
    else if(status==errSecInteractionNotAllowed)reason=@"系统未允许授权交互，请在设置中解锁钥匙串后重试。";
    else if(status==errSecInvalidOwnerEdit)reason=@"该凭据的所有者控制拒绝删除，请在钥匙串访问中确认这一条项目；未修改所有者或访问控制。";
    else if(status==errSecAuthFailed)reason=@"钥匙串拒绝授权，请在钥匙串访问中检查该项访问控制。";
    else if(status==errSecItemNotFound)reason=@"凭据已变化，请重新扫描。";
    else reason=CFBridgingRelease(SecCopyErrorMessageString(status,NULL))?:@"系统操作失败。";
    return cleanResponse(@{@"error":[NSString stringWithFormat:@"%@：%@（OSStatus %d）",stage,reason,(int)status]});
}
char *rf_credentials_remove(const char *value) {
    NSData *reference = [[NSData alloc] initWithBase64EncodedString:[NSString stringWithUTF8String:value] options:0];
    if (!reference.length) return cleanResponse(@{@"error":@"凭据编号无效，请重新扫描。"});
    LAContext *context=[LAContext new];context.interactionNotAllowed=NO;
    NSDictionary *query = @{(__bridge id)kSecValuePersistentRef:reference,
        (__bridge id)kSecReturnAttributes:@YES,(__bridge id)kSecReturnRef:@YES,
        (__bridge id)kSecUseAuthenticationContext:context};
    CFTypeRef raw = NULL;
    OSStatus status = SecItemCopyMatching((__bridge CFDictionaryRef)query, &raw);
    if (status != errSecSuccess) return credentialError(status, @"无法核验凭据属性");
    NSDictionary *row = CFBridgingRelease(raw);
    if (!credentialService(row[(__bridge id)kSecAttrService])) return cleanResponse(@{@"error":@"凭据不属于 Claude，拒绝删除。"});
    id item = row[(__bridge id)kSecValueRef];
    if (!item) return cleanResponse(@{@"error":@"无法取得这一条凭据的系统引用，请重新扫描。"});
    // ----- 文件钥匙串使用原生单项删除；SecItemDelete 的共享密码保护会返回 -25244 -----
    // 这与 Apple security 工具相同，仍由 Security 校验实际删除权限，不修改所有者或 ACL。
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
    if(CFGetTypeID((__bridge CFTypeRef)item)==SecKeychainItemGetTypeID())
        status=SecKeychainItemDelete((__bridge SecKeychainItemRef)item);
    else status=SecItemDelete((__bridge CFDictionaryRef)@{(__bridge id)kSecClass:(__bridge id)kSecClassGenericPassword,
        (__bridge id)kSecMatchItemList:@[item]});
#pragma clang diagnostic pop
    if (status != errSecSuccess) return credentialError(status, @"钥匙串删除失败");
    return cleanResponse(@{@"ok":@YES});
}
