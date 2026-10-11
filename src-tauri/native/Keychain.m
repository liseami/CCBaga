/*
 * [INPUT]: Security 的登录钥匙串授权接口。
 * [OUTPUT]: 用户触发的登录钥匙串解锁。
 * [POS]: 授权边界；解锁不读取任何凭据，不改变系统的删除授权策略。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
#import <Foundation/Foundation.h>
#import <Security/Security.h>
static char *keyResponse(NSDictionary *value) {
    NSData *data=[NSJSONSerialization dataWithJSONObject:value options:0 error:NULL];
    return strdup([[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding].UTF8String);
}
// 现存 Claude 凭据使用 macOS 文件钥匙串，解锁沿用对应公开 API。
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
char *rf_keychain_unlock(void) {
    SecKeychainRef keychain=NULL;OSStatus status=SecKeychainCopyDefault(&keychain);
    if(status!=errSecSuccess)return keyResponse(@{@"error":[NSString stringWithFormat:@"登录钥匙串不可用（OSStatus %d）。",(int)status]});
    SecKeychainStatus state=0;status=SecKeychainGetStatus(keychain,&state);
    if(status==errSecSuccess&&!(state&kSecUnlockStateStatus))status=SecKeychainUnlock(keychain,0,NULL,TRUE);
    CFRelease(keychain);
    if(status==errSecUserCanceled)return keyResponse(@{@"error":@"已取消系统钥匙串解锁。"});
    if(status!=errSecSuccess)return keyResponse(@{@"error":[NSString stringWithFormat:@"钥匙串解锁失败（OSStatus %d），请在钥匙串访问中解锁登录钥匙串。",(int)status]});
    return keyResponse(@{@"message":@"登录钥匙串已解锁。具体凭据的删除许可由 macOS 在清理时决定；系统要求时请完成授权。"});
}

#pragma clang diagnostic pop
