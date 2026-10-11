/*
 * [INPUT]: 主线程 AppKit 文件选择器、系统 UTType与 pem/ppk/pub 扩展名白名单。
 * [OUTPUT]: 用户明确选择的本机金钥文件路径；取消返回空数组，不读取文件正文。
 * [POS]: VPS 文件导入桥；文件内容与配对核验留在 Rust，界面只接收路径。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
#import <AppKit/AppKit.h>
#import <UniformTypeIdentifiers/UniformTypeIdentifiers.h>
char *rf_choose_ssh_keys(const char *extension) {
    @autoreleasepool {
        NSString *type = [NSString stringWithUTF8String:extension];
        NSArray *types = @[@"pem", @"ppk", @"pub"];
        if (![type isEqualToString:@"all"] && ![types containsObject:type]) return NULL;
        NSOpenPanel *panel = [NSOpenPanel openPanel];
        panel.canChooseFiles = YES;
        panel.canChooseDirectories = NO;
        panel.allowsMultipleSelection = [type isEqualToString:@"all"];
        NSMutableArray<UTType *> *contentTypes = [NSMutableArray array];
        for (NSString *ext in (panel.allowsMultipleSelection ? types : @[type])) {
            UTType *contentType = [UTType typeWithFilenameExtension:ext];
            if (contentType) [contentTypes addObject:contentType];
        }
        panel.allowedContentTypes = contentTypes;
        panel.allowsOtherFileTypes = NO;
        NSMutableArray *paths = [NSMutableArray array];
        if ([panel runModal] == NSModalResponseOK) {
            for (NSURL *url in panel.URLs) [paths addObject:url.path];
        }
        NSData *data = [NSJSONSerialization dataWithJSONObject:paths options:0 error:nil];
        return data ? strdup([[NSString alloc] initWithData:data encoding:NSUTF8StringEncoding].UTF8String) : NULL;
    }
}
