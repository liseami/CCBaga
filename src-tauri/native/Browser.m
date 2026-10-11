/*
 * [INPUT]: Foundation/CoreLocation 状态、SystemConfiguration 代理、LaunchServices 注册/默认处理程序与 proc 进程身份。
 * [OUTPUT]: 真实机器快照、浏览器应用注册、按包标识查询运行、默认浏览器读取与系统确认式设置、资料进程查询和正常退出、美国区域设置。
 * [POS]: 浏览器的只读机器来源和进程边界；不伪造硬件，不读取浏览器凭据。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
#import <AppKit/AppKit.h>
#import <CoreServices/CoreServices.h>
#import <CoreLocation/CoreLocation.h>
#import <SystemConfiguration/SystemConfiguration.h>
#import <libproc.h>
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
    [NSTimeZone resetSystemTimeZone];
    return browserResponse(@{@"platform": @"macos", @"version": [NSString stringWithFormat:@"%ld.%ld.%ld", (long)v.majorVersion, (long)v.minorVersion, (long)v.patchVersion],
        @"cores": @(NSProcessInfo.processInfo.activeProcessorCount), @"timezone": NSTimeZone.systemTimeZone.name,
        @"languages": [languages isKindOfClass:NSArray.class] ? languages : NSLocale.preferredLanguages,
        @"locale": [locale isKindOfClass:NSString.class] ? locale : NSLocale.currentLocale.localeIdentifier,
        @"location_enabled": @([CLLocationManager locationServicesEnabled]), @"timezones": NSTimeZone.knownTimeZoneNames,
        @"system_proxy": systemProxy});
}
static NSRunningApplication *profileApplication(NSString *profile) {
    NSString *lock = [NSFileManager.defaultManager destinationOfSymbolicLinkAtPath:[profile stringByAppendingPathComponent:@"SingletonLock"] error:NULL];
    NSString *number = [[lock componentsSeparatedByString:@"-"] lastObject];
    if (!number.length || [number rangeOfCharacterFromSet:NSCharacterSet.decimalDigitCharacterSet.invertedSet].location != NSNotFound) return nil;
    pid_t pid = number.intValue;
    struct proc_bsdinfo info = {0};
    if (pid <= 1 || proc_pidinfo(pid, PROC_PIDTBSDINFO, 0, &info, sizeof(info)) != sizeof(info) || info.pbi_uid != getuid()) return nil;
    NSRunningApplication *app = [NSRunningApplication runningApplicationWithProcessIdentifier:pid];
    if (![@[@"ai.local.requirement-french.browser", @"local.ccbroswer"] containsObject:app.bundleIdentifier] || ![@[@"Chromium", @"CCBrowser"] containsObject:app.executableURL.lastPathComponent]) return nil;
    return app;
}
char *rf_browser_running(const char *profile) {
    NSRunningApplication *app = profileApplication([NSString stringWithUTF8String:profile]);
    return browserResponse(@{@"running": app ? @YES : @NO});
}
char *rf_browser_quit(const char *profile) {
    NSRunningApplication *app = profileApplication([NSString stringWithUTF8String:profile]);
    if (app && ![app terminate]) return browserResponse(@{@"error": @"浏览器未接受退出，请在浏览器窗口保存任务后退出。"});
    return browserResponse(@{@"ok": @YES});
}
