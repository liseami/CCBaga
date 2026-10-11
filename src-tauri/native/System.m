/*
 * [INPUT]: Foundation 时区与用户偏好、CoreLocation 状态、系统设置辅助功能接口。
 * [OUTPUT]: 电脑环境快照、首选语言与地区写入、管理员授权的时区修改与可核验的定位关闭。
 * [POS]: 电脑设置的 macOS 边界；定位通过系统开关关闭，不写 locationd 私有数据库。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
#import <AppKit/AppKit.h>
#import <ApplicationServices/ApplicationServices.h>
#import <CoreLocation/CoreLocation.h>

static char *systemResponse(NSDictionary *data) {
    NSData *json = [NSJSONSerialization dataWithJSONObject:data options:0 error:NULL];
    return strdup([[NSString alloc] initWithData:json encoding:NSUTF8StringEncoding].UTF8String);
}
static NSArray *savedLanguages(void) {
    id value = CFBridgingRelease(CFPreferencesCopyValue(CFSTR("AppleLanguages"), kCFPreferencesAnyApplication,
        kCFPreferencesCurrentUser, kCFPreferencesAnyHost));
    return [value isKindOfClass:NSArray.class] ? value : NSLocale.preferredLanguages;
}
static NSDictionary *snapshot(void) {
    [NSTimeZone resetSystemTimeZone];
    NSDictionary *automatic = [NSDictionary dictionaryWithContentsOfFile:@"/Library/Preferences/com.apple.timezone.auto.plist"];
    return @{@"timezone": NSTimeZone.systemTimeZone.name,
        @"timezones": NSTimeZone.knownTimeZoneNames,
        @"automatic_timezone": automatic[@"Active"] ?: NSNull.null,
        @"languages": savedLanguages(),
        @"location_enabled": @([CLLocationManager locationServicesEnabled]),
        @"accessibility": @(AXIsProcessTrusted())};
}
char *rf_system_read(void) { return systemResponse(snapshot()); }
static BOOL openSettings(NSString *pane) {
    __block BOOL opened = NO;
    void (^open)(void) = ^{
        opened = [NSWorkspace.sharedWorkspace openURL:[NSURL URLWithString:pane]];
    };
    if (NSThread.isMainThread) open(); else dispatch_sync(dispatch_get_main_queue(), open);
    return opened;
}
char *rf_system_open(const char *value) {
    NSDictionary *panes = @{
        @"timezone": @"x-apple.systempreferences:com.apple.Date-Time-Settings.extension",
        @"language": @"x-apple.systempreferences:com.apple.Localization-Settings.extension",
        @"location": @"x-apple.systempreferences:com.apple.preference.security?Privacy_LocationServices"};
    NSString *pane = panes[[NSString stringWithUTF8String:value]];
    if (!pane || !openSettings(pane)) return systemResponse(@{@"error": @"无法打开对应系统设置页面。"});
    return systemResponse(@{@"ok": @YES});
}
char *rf_system_language(const char *value) {
    NSString *language = [NSString stringWithUTF8String:value];
    NSArray *allowed = @[@"zh-Hans", @"zh-Hant", @"en", @"fr", @"de", @"es", @"it", @"pt-BR", @"ja", @"ko", @"ru", @"ar"];
    if (![allowed containsObject:language]) return systemResponse(@{@"error": @"请选择支持的系统语言。"});
    NSMutableArray *languages = [savedLanguages() mutableCopy];
    [languages removeObject:language];
    [languages insertObject:language atIndex:0];
    CFPreferencesSetValue(CFSTR("AppleLanguages"), (__bridge CFArrayRef)languages,
        kCFPreferencesAnyApplication, kCFPreferencesCurrentUser, kCFPreferencesAnyHost);
    if (!CFPreferencesSynchronize(kCFPreferencesAnyApplication, kCFPreferencesCurrentUser, kCFPreferencesAnyHost) ||
        ![savedLanguages().firstObject isEqual:language]) return systemResponse(@{@"error": @"语言偏好未能写入，请在系统设置中完成修改。"});
    return systemResponse(@{@"state": snapshot(), @"message": @"首选语言已保存。重新打开应用后生效；系统完整切换可能需要注销或重启。"});
}
char *rf_system_timezone(const char *value) {
    NSString *zone = [NSString stringWithUTF8String:value];
    NSCharacterSet *safe = [NSCharacterSet characterSetWithCharactersInString:@"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789/_+-"];
    if (![NSTimeZone.knownTimeZoneNames containsObject:zone] || [zone rangeOfCharacterFromSet:safe.invertedSet].location != NSNotFound)
        return systemResponse(@{@"error": @"请选择系统支持的时区。"});
    // ----- 系统授权只在明确点击应用时触发；参数已通过本机时区白名单 -----
    NSString *script = [NSString stringWithFormat:@"do shell script \"/usr/bin/defaults write /Library/Preferences/com.apple.timezone.auto Active -bool false && /usr/sbin/systemsetup -settimezone %@\" with administrator privileges", zone];
    NSTask *task = [NSTask new];
    task.executableURL = [NSURL fileURLWithPath:@"/usr/bin/osascript"];
    task.arguments = @[@"-e", script];
    task.standardOutput = [NSFileHandle fileHandleWithNullDevice];
    task.standardError = [NSFileHandle fileHandleWithNullDevice];
    if (![task launchAndReturnError:NULL]) return systemResponse(@{@"error": @"无法启动系统授权。"});
    [task waitUntilExit];
    if (task.terminationStatus != 0) return systemResponse(@{@"error": @"时区修改未完成或管理员授权已取消，请刷新当前状态。"});
    NSDictionary *state = snapshot();
    if (![state[@"timezone"] isEqual:zone] || ![state[@"automatic_timezone"] isEqual:@NO])
        return systemResponse(@{@"error": @"系统未确认手动时区设置，请在日期与时间页面检查自动时区开关。"});
    return systemResponse(@{@"state": state, @"message": @"时区已应用，按位置自动设置时区已关闭。"});
}

// ----- 只按稳定标识或语义标签匹配定位总开关，绝不按列表位置操作 -----
static id axValue(AXUIElementRef element, CFStringRef attribute) {
    CFTypeRef value = NULL;
    if (AXUIElementCopyAttributeValue(element, attribute, &value) != kAXErrorSuccess) return nil;
    return CFBridgingRelease(value);
}
static AXUIElementRef findControl(AXUIElementRef element, NSSet *labels, BOOL toggle, NSUInteger depth, NSUInteger *count) {
    if (depth > 20 || ++(*count) > 4000) return NULL;
    NSString *role = axValue(element, kAXRoleAttribute);
    BOOL eligible = toggle ? ([role isEqual:@"AXCheckBox"] || [role isEqual:@"AXSwitch"] || [role isEqual:@"AXButton"]) : [role isEqual:@"AXButton"];
    if (eligible) {
        if (toggle && [axValue(element, kAXIdentifierAttribute) isEqual:@"Location_Services_Toggle"])
            return (AXUIElementRef)CFRetain(element);
        for (id attribute in @[(__bridge id)kAXTitleAttribute, (__bridge id)kAXDescriptionAttribute]) {
            id label = axValue(element, (__bridge CFStringRef)attribute);
            if ([label isKindOfClass:NSString.class] && [labels containsObject:label]) return (AXUIElementRef)CFRetain(element);
        }
    }
    for (id child in axValue(element, kAXChildrenAttribute)) {
        AXUIElementRef found = findControl((__bridge AXUIElementRef)child, labels, toggle, depth + 1, count);
        if (found) return found;
    }
    return NULL;
}
char *rf_system_location(void) {
    if (![CLLocationManager locationServicesEnabled]) return systemResponse(@{@"state": snapshot(), @"message": @"定位服务已关闭。"});
    if (!openSettings(@"x-apple.systempreferences:com.apple.preference.security?Privacy_LocationServices"))
        return systemResponse(@{@"error": @"无法打开定位服务设置。"});
    BOOL pressed = NO;
    NSSet *labels = [NSSet setWithArray:@[@"Location Services", @"定位服务", @"定位服務", @"Services de localisation", @"Service de localisation"]];
    NSSet *confirm = [NSSet setWithArray:@[@"Turn Off", @"关闭", @"關閉", @"停用", @"Désactiver"]];
    if (AXIsProcessTrusted()) {
        for (NSUInteger attempt = 0; attempt < 32; attempt++) {
            if (![CLLocationManager locationServicesEnabled]) break;
            NSRunningApplication *app = [NSRunningApplication runningApplicationsWithBundleIdentifier:@"com.apple.systempreferences"].firstObject;
            if (app) {
                AXUIElementRef root = AXUIElementCreateApplication(app.processIdentifier);
                AXUIElementSetMessagingTimeout(root, 0.5);
                NSUInteger count = 0;
                AXUIElementRef control = findControl(root, pressed ? confirm : labels, !pressed, 0, &count);
                if (control) {
                    id current = axValue(control, kAXValueAttribute);
                    // 无法识别开关状态时留给用户，避免误点或反向开启。
                    if (pressed || ([current respondsToSelector:@selector(integerValue)] && [current integerValue] == 1)) {
                        if (AXUIElementPerformAction(control, kAXPressAction) == kAXErrorSuccess) pressed = YES;
                    }
                    CFRelease(control);
                }
                CFRelease(root);
            }
            [NSThread sleepForTimeInterval:0.25];
        }
    }
    BOOL enabled = [CLLocationManager locationServicesEnabled];
    return systemResponse(@{@"state": snapshot(), @"message": enabled ?
        @"系统设置已打开。请完成定位总开关关闭或系统授权，返回后点击刷新；当前仍检测到定位开启。" : @"已核验：定位服务已关闭。"});
}

// ----- 地区操作由 Rust 目录限定；写入后读取偏好核验，保留其他语言 -----
char *rf_system_region(const char *languageValue, const char *localeValue) {
    NSString *language = [NSString stringWithUTF8String:languageValue];
    NSString *locale = [NSString stringWithUTF8String:localeValue];
    NSArray *allowed = @[@"en-US", @"en-CA", @"en-GB", @"en-SG", @"en-AU", @"fr", @"de", @"ja", @"ko", @"zh-Hans", @"zh-Hant"];
    if (![allowed containsObject:language] || ![NSLocale.availableLocaleIdentifiers containsObject:locale])
        return systemResponse(@{@"error": @"语言或地区格式无效。"});
    NSMutableArray *languages = [savedLanguages() mutableCopy];
    [languages removeObject:language]; [languages insertObject:language atIndex:0];
    CFPreferencesSetValue(CFSTR("AppleLanguages"), (__bridge CFArrayRef)languages, kCFPreferencesAnyApplication, kCFPreferencesCurrentUser, kCFPreferencesAnyHost);
    CFPreferencesSetValue(CFSTR("AppleLocale"), (__bridge CFStringRef)locale, kCFPreferencesAnyApplication, kCFPreferencesCurrentUser, kCFPreferencesAnyHost);
    if (!CFPreferencesSynchronize(kCFPreferencesAnyApplication, kCFPreferencesCurrentUser, kCFPreferencesAnyHost))
        return systemResponse(@{@"error": @"语言与地区写入失败，时区可能已修改。"});
    id savedLocale = CFBridgingRelease(CFPreferencesCopyValue(CFSTR("AppleLocale"), kCFPreferencesAnyApplication, kCFPreferencesCurrentUser, kCFPreferencesAnyHost));
    if (![savedLanguages().firstObject isEqual:language] || ![savedLocale isEqual:locale])
        return systemResponse(@{@"error": @"语言与地区核验失败，请打开系统设置检查。"});
    return systemResponse(@{@"state": snapshot(), @"message": @"首选语言与地区格式已保存，部分应用需重启或注销。"});
}
