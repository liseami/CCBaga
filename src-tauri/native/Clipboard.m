/*
 * [INPUT]: AppKit 剪贴板和辅助功能设置入口。
 * [OUTPUT]: 文本复制、授权页面入口和共享响应内存释放。
 * [POS]: 宿主的小型系统桥；不捕获选区、不模拟键盘、不注册快捷键。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
#import <AppKit/AppKit.h>
void rf_copy(const char *text) { [NSPasteboard.generalPasteboard clearContents]; [NSPasteboard.generalPasteboard setString:[NSString stringWithUTF8String:text] forType:NSPasteboardTypeString]; }
void rf_permissions(void) { [NSWorkspace.sharedWorkspace openURL:[NSURL URLWithString:@"x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"]]; }
void rf_free(char *value) { free(value); }
