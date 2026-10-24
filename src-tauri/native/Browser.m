/*
 * [INPUT]: Foundation/CoreLocation 状态、SystemConfiguration 代理、LaunchServices 注册/默认处理程序与 proc 进程身份。
 * [OUTPUT]: 含有效区域国家（保留原始 locale）的真实机器快照、浏览器应用注册、按包标识查询运行、默认浏览器读取与系统确认式设置、美国区域设置；资料进程以用户/包标识、唯一绝对 --user-data-dir 与目标目录的设备/inode 一致及启动时间复核；仅明确不同才视为未运行，参数/元数据/进程信息不可读或参数歧义时报告未知，退出只作用于确认属于该资料的进程。
 * [POS]: 浏览器的只读机器来源和进程边界；不伪造硬件，不读取浏览器凭据。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
#import <AppKit/AppKit.h>
#import <CoreServices/CoreServices.h>
#import <CoreLocation/CoreLocation.h>
#import <SystemConfiguration/SystemConfiguration.h>
#import <libproc.h>
#import <sys/stat.h>
#import <sys/sysctl.h>
#import <unistd.h>
static char *browserResponse(NSDictionary *value) {
    NSData *data = [NSJSONSerialization dataWithJSONObject:value options:0 error:NULL];
    return strdup([[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding].UTF8String);
}
char *rf_browser_register(const char *path) {
    NSURL *url=[NSURL fileURLWithPath:[NSString stringWithUTF8String:path] isDirectory:YES];
    OSStatus status=LSRegisterURL((__bridge CFURLRef)url, true);
    if(status!=noErr)return browserResponse(@{@"error":[NSString stringWithFormat:@"浏览器应用注册失败（OSStatus %d）。",(int)status]});
    return browserResponse(@{@"ok":@YES});
}
char *rf_browser_app_running(const char *bundle) {
    NSString *identifier=[NSString stringWithUTF8String:bundle];
    BOOL running=NO;
    for(NSRunningApplication *app in [NSRunningApplication runningApplicationsWithBundleIdentifier:identifier])
        if(!app.terminated){running=YES;break;}
    return browserResponse(@{@"running":@(running)});
}
static NSString *handlerFor(NSString *scheme) {
    NSURL *app=[NSWorkspace.sharedWorkspace URLForApplicationToOpenURL:[NSURL URLWithString:[scheme stringByAppendingString:@"://example.com"]]];
    NSString *identifier=app?[NSBundle bundleWithURL:app].bundleIdentifier:nil;
    return identifier?:@"";
}
char *rf_browser_default_read(const char *bundle) {
    NSString *identifier=[NSString stringWithUTF8String:bundle];
    NSString *http=handlerFor(@"http"), *https=handlerFor(@"https");
    BOOL isDefault=[http caseInsensitiveCompare:identifier]==NSOrderedSame && [https caseInsensitiveCompare:identifier]==NSOrderedSame;
    return browserResponse(@{@"is_default":@(isDefault),@"http":http,@"https":https});
}
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
/* 系统会弹出“是否更改默认网页浏览器”确认框；用户选择前返回的是请求结果，不代表已切换。 */
char *rf_browser_set_default(const char *bundle) {
    CFStringRef identifier=CFStringCreateWithCString(NULL,bundle,kCFStringEncodingUTF8);
    OSStatus http=LSSetDefaultHandlerForURLScheme(CFSTR("http"),identifier);
    OSStatus https=LSSetDefaultHandlerForURLScheme(CFSTR("https"),identifier);
    OSStatus html=LSSetDefaultRoleHandlerForContentType(CFSTR("public.html"),kLSRolesViewer,identifier);
    OSStatus xhtml=LSSetDefaultRoleHandlerForContentType(CFSTR("public.xhtml"),kLSRolesViewer,identifier);
    CFRelease(identifier);
    if(http!=noErr||https!=noErr)
        return browserResponse(@{@"error":[NSString stringWithFormat:@"系统拒绝设置默认浏览器（http %d / https %d）。",(int)http,(int)https]});
    return browserResponse(@{@"ok":@YES,@"http":@(http),@"https":@(https),@"html":@(html),@"xhtml":@(xhtml)});
}
#pragma clang diagnostic pop
char *rf_browser_machine(void) {
    NSOperatingSystemVersion v = NSProcessInfo.processInfo.operatingSystemVersion;
    NSDictionary *proxy = CFBridgingRelease(SCDynamicStoreCopyProxies(NULL)) ?: @{};
    NSString *systemProxy = @"";
    for (NSString *kind in @[@"HTTPS", @"HTTP", @"SOCKS"]) {
        if ([proxy[[kind stringByAppendingString:@"Enable"]] boolValue]) {
            NSString *host = proxy[[kind stringByAppendingString:@"Proxy"]];
            NSNumber *port = proxy[[kind stringByAppendingString:@"Port"]];
            if (host && port) { systemProxy = [NSString stringWithFormat:@"%@://%@:%@", [kind isEqual:@"SOCKS"] ? @"socks5h" : @"http", host, port]; break; }
        }
    }
    id languages = CFBridgingRelease(CFPreferencesCopyValue(CFSTR("AppleLanguages"), kCFPreferencesAnyApplication, kCFPreferencesCurrentUser, kCFPreferencesAnyHost));
    id locale = CFBridgingRelease(CFPreferencesCopyValue(CFSTR("AppleLocale"), kCFPreferencesAnyApplication, kCFPreferencesCurrentUser, kCFPreferencesAnyHost));
    NSString *localeIdentifier = [locale isKindOfClass:NSString.class] ? locale : NSLocale.currentLocale.localeIdentifier;
    NSString *localeCountry = [[NSLocale alloc] initWithLocaleIdentifier:localeIdentifier].countryCode ?: @"";
    [NSTimeZone resetSystemTimeZone];
    return browserResponse(@{@"platform": @"macos", @"version": [NSString stringWithFormat:@"%ld.%ld.%ld", (long)v.majorVersion, (long)v.minorVersion, (long)v.patchVersion],
        @"cores": @(NSProcessInfo.processInfo.activeProcessorCount), @"timezone": NSTimeZone.systemTimeZone.name,
        @"languages": [languages isKindOfClass:NSArray.class] ? languages : NSLocale.preferredLanguages,
        @"locale": localeIdentifier, @"locale_country": localeCountry,
        @"location_enabled": @([CLLocationManager locationServicesEnabled]), @"timezones": NSTimeZone.knownTimeZoneNames,
        @"system_proxy": systemProxy});
}
typedef NS_ENUM(int, ProfileOwnership) { ProfileOwnershipNone, ProfileOwnershipOwned, ProfileOwnershipUnknown };
/* The single explicit absolute --user-data-dir before "--"; nil when it is missing, repeated, valueless or relative. */
static NSString *profileArgument(NSArray<NSString *> *arguments) {
    NSString *selected = nil;
    for (NSUInteger i = 1; i < arguments.count; i++) {
        NSString *arg = arguments[i];
        if ([arg isEqualToString:@"--"]) break;
        NSString *value = nil;
        if ([arg hasPrefix:@"--user-data-dir="]) value = [arg substringFromIndex:16];
        else if ([arg isEqualToString:@"--user-data-dir"]) {
            if (++i >= arguments.count) return nil;
            value = arguments[i];
        }
        if (value) {
            if (selected) return nil; // Repeated switches are ambiguous, even when one of them matches.
            selected = value;
        }
    }
    return [selected hasPrefix:@"/"] ? selected : nil;
}
/* Directory metadata only: trailing slashes, ".", "..", symlinks and case aliases resolve to one dev/inode. */
static ProfileOwnership profileDirectoryOwnership(NSString *selected, NSString *profile) {
    struct stat a = {0}, b = {0};
    if (!selected || stat(selected.fileSystemRepresentation, &a) != 0 || stat(profile.fileSystemRepresentation, &b) != 0
        || !S_ISDIR(a.st_mode) || !S_ISDIR(b.st_mode)) return ProfileOwnershipUnknown;
    return a.st_dev == b.st_dev && a.st_ino == b.st_ino ? ProfileOwnershipOwned : ProfileOwnershipNone;
}
/* Same-user argv via KERN_PROCARGS2; nil when it cannot be read completely. The buffer is cleared and never logged. */
static NSArray<NSString *> *profileArguments(pid_t pid) {
    int mib[] = {CTL_KERN, KERN_PROCARGS2, pid};
    size_t size = 0;
    if (sysctl(mib, 3, NULL, &size, NULL, 0) != 0 || size > 1048576 || size <= sizeof(int)) return nil;
    size_t allocated = size;
    char *raw = calloc(1, allocated);
    if (!raw) return nil;
    NSMutableArray<NSString *> *arguments = nil;
    if (sysctl(mib, 3, raw, &size, NULL, 0) == 0 && size <= allocated && size > sizeof(int)) {
        int argc = 0; memcpy(&argc, raw, sizeof(argc));
        if (argc > 0 && argc < 16384) {
            char *cursor = raw + sizeof(argc), *end = raw + size;
            cursor += strnlen(cursor, end - cursor);
            while (cursor < end && *cursor == 0) cursor++;
            arguments = [NSMutableArray array];
            for (int i = 0; i < argc && cursor < end; i++) {
                size_t length = strnlen(cursor, end - cursor);
                if (length == (size_t)(end - cursor)) break;
                NSString *arg = [[NSString alloc] initWithBytes:cursor length:length encoding:NSUTF8StringEncoding];
                if (!arg) break;
                [arguments addObject:arg];
                cursor += length + 1;
            }
            if (arguments.count != (NSUInteger)argc) arguments = nil;
        }
    }
    memset(raw, 0, allocated); free(raw);
    return arguments;
}
/* Only an observed ESRCH means the locked PID is gone; any other process-info failure stays unknown. */
static ProfileOwnership missingProcess(int error) {
    return error == ESRCH ? ProfileOwnershipNone : ProfileOwnershipUnknown;
}
/* The second read must show the same UID/start time; any failure or difference is unknown, never "not running". */
static ProfileOwnership profileOwnership(const struct proc_bsdinfo *before, BOOL reread, const struct proc_bsdinfo *after,
                                         NSArray<NSString *> *arguments, NSString *profile) {
    if (!reread || after->pbi_uid != before->pbi_uid || after->pbi_start_tvsec != before->pbi_start_tvsec
        || after->pbi_start_tvusec != before->pbi_start_tvusec || !arguments) return ProfileOwnershipUnknown;
    return profileDirectoryOwnership(profileArgument(arguments), profile);
}
static NSRunningApplication *profileApplication(NSString *profile, BOOL *unknown) {
    *unknown = NO;
    NSString *lock = [NSFileManager.defaultManager destinationOfSymbolicLinkAtPath:[profile stringByAppendingPathComponent:@"SingletonLock"] error:NULL];
    NSString *number = [[lock componentsSeparatedByString:@"-"] lastObject];
    if (!number.length || [number rangeOfCharacterFromSet:NSCharacterSet.decimalDigitCharacterSet.invertedSet].location != NSNotFound) return nil;
    pid_t pid = number.intValue;
    struct proc_bsdinfo info = {0};
    if (pid <= 1) return nil;
    errno = 0;
    if (proc_pidinfo(pid, PROC_PIDTBSDINFO, 0, &info, sizeof(info)) != sizeof(info)) {
        *unknown = missingProcess(errno) == ProfileOwnershipUnknown;
        return nil;
    }
    if (info.pbi_uid != getuid()) return nil;
    NSRunningApplication *app = [NSRunningApplication runningApplicationWithProcessIdentifier:pid];
    if (![@[@"ai.local.requirement-french.browser", @"local.ccbroswer"] containsObject:app.bundleIdentifier] || ![@[@"Chromium", @"CCBrowser"] containsObject:app.executableURL.lastPathComponent]) return nil;
    NSArray<NSString *> *arguments = profileArguments(pid);
    struct proc_bsdinfo current = {0};
    BOOL reread = proc_pidinfo(pid, PROC_PIDTBSDINFO, 0, &current, sizeof(current)) == sizeof(current);
    ProfileOwnership ownership = profileOwnership(&info, reread, &current, arguments, profile);
    *unknown = ownership == ProfileOwnershipUnknown;
    return ownership == ProfileOwnershipOwned ? app : nil;
}
char *rf_browser_running(const char *profile) {
    BOOL unknown = NO;
    NSRunningApplication *app = profileApplication([NSString stringWithUTF8String:profile], &unknown);
    if (unknown) return browserResponse(@{@"error": @"无法确认浏览器进程的资料归属，未操作资料。"});
    return browserResponse(@{@"running": app ? @YES : @NO});
}
char *rf_browser_quit(const char *profile) {
    BOOL unknown = NO;
    NSRunningApplication *app = profileApplication([NSString stringWithUTF8String:profile], &unknown);
    if (unknown) return browserResponse(@{@"error": @"无法确认浏览器进程的资料归属，未操作资料。"});
    if (app && ![app terminate]) return browserResponse(@{@"error": @"浏览器未接受退出，请在浏览器窗口保存任务后退出。"});
    return browserResponse(@{@"ok": @YES});
}
