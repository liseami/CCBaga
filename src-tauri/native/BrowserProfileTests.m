// [INPUT]: 虚构 argv、合成进程信息、临时自建资料目录夹具、本测试进程自身 argv 与已回收子进程编号。
// [OUTPUT]: 唯一绝对参数解析、按设备/inode 的同目录别名与不同目录判定、歧义/元数据/进程信息读取失败均为未知及真实 sysctl argv 读取回归。
// [POS]: 独立原生测试；只在临时目录创建夹具，不读真实浏览器资料，不启动或结束浏览器，不枚举其他进程，不输出 argv。
// [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
#import "Browser.m"
#include <stdio.h>
#include <stdlib.h>
#include <sys/wait.h>
static int passed = 0;
#define CHECK(...) do { if (!(__VA_ARGS__)) { fprintf(stderr, "FAILED %s:%d %s\n", __FILE__, __LINE__, #__VA_ARGS__); exit(1); } passed++; } while (0)
static NSDictionary *decodeResponse(char *raw) {
    NSDictionary *value = [NSJSONSerialization JSONObjectWithData:[NSData dataWithBytes:raw length:strlen(raw)] options:0 error:NULL];
    free(raw);
    return value;
}
static ProfileOwnership owner(NSString *argument, NSString *profile) {
    return profileDirectoryOwnership(profileArgument(@[@"Chromium", [@"--user-data-dir=" stringByAppendingString:argument]]), profile);
}
int main(int argc, char **argv) {
    @autoreleasepool {
        NSFileManager *files = NSFileManager.defaultManager;
        NSString *base = NSProcessInfo.processInfo.environment[@"CCBAGA_TEST_TMP"] ?: NSTemporaryDirectory();
        char *dir = strdup([[base stringByAppendingPathComponent:@"ccbaga-profile-XXXXXX"] fileSystemRepresentation]);
        CHECK(mkdtemp(dir) != NULL);
        NSString *root = [files stringWithFileSystemRepresentation:dir length:strlen(dir)];
        NSString *profile = [root stringByAppendingPathComponent:@"独立 环境"];
        NSString *other = [root stringByAppendingPathComponent:@"独立 环境-other"];
        NSString *link = [root stringByAppendingPathComponent:@"link"];
        CHECK([files createDirectoryAtPath:[profile stringByAppendingPathComponent:@"Default"] withIntermediateDirectories:YES attributes:nil error:NULL]);
        CHECK([files createDirectoryAtPath:other withIntermediateDirectories:NO attributes:nil error:NULL]);
        CHECK([files createSymbolicLinkAtPath:link withDestinationPath:profile error:NULL]);
        CHECK([@"fixture" writeToFile:[root stringByAppendingPathComponent:@"file"] atomically:NO encoding:NSUTF8StringEncoding error:NULL]);

        // One explicit absolute switch before "--"; everything else is ambiguous (nil → unknown).
        CHECK([profileArgument(@[@"Chromium", @"--user-data-dir=/a", @"about:blank"]) isEqualToString:@"/a"]);
        CHECK([profileArgument(@[@"Chromium Helper", @"--type=renderer", @"--user-data-dir", @"/a"]) isEqualToString:@"/a"]);
        CHECK(profileArgument(@[@"Chromium", @"--user-data-dir=/a", @"--user-data-dir=/a"]) == nil);
        CHECK(profileArgument(@[@"Chromium", @"--user-data-dir=/b", @"--user-data-dir", @"/a"]) == nil);
        CHECK(profileArgument(@[@"Chromium", @"--user-data-dir"]) == nil);
        CHECK(profileArgument(@[@"Chromium", @"--user-data-dir="]) == nil);
        CHECK(profileArgument(@[@"Chromium", @"--user-data-dir=relative/a"]) == nil);
        CHECK(profileArgument(@[@"Chromium", @"--", @"--user-data-dir=/a"]) == nil);
        CHECK(profileArgument(@[@"Chromium", @"https://example.org/?--user-data-dir=/a"]) == nil);
        CHECK(profileArgument(@[@"Chromium", @"-user-data-dir=/a"]) == nil);
        CHECK(profileArgument(@[@"--user-data-dir=/a"]) == nil);
        CHECK(profileArgument(@[]) == nil);

        // Same directory through aliases resolves to one dev/inode.
        CHECK(owner(profile, profile) == ProfileOwnershipOwned);
        CHECK(owner([profile stringByAppendingString:@"/"], profile) == ProfileOwnershipOwned);
        CHECK(owner([profile stringByAppendingString:@"/./Default/.."], profile) == ProfileOwnershipOwned);
        CHECK(owner(link, profile) == ProfileOwnershipOwned);
        CHECK(owner(profile, [link stringByAppendingString:@"/"]) == ProfileOwnershipOwned);
        // Case alias only where the fixture volume itself is case-insensitive.
        struct stat lower = {0}, upper = {0};
        NSString *upperRoot = [[root stringByDeletingLastPathComponent] stringByAppendingPathComponent:root.lastPathComponent.uppercaseString];
        if (stat(root.fileSystemRepresentation, &lower) == 0 && stat(upperRoot.fileSystemRepresentation, &upper) == 0 && lower.st_ino == upper.st_ino)
            CHECK(owner([upperRoot stringByAppendingPathComponent:@"独立 环境"], profile) == ProfileOwnershipOwned);
        // Two existing different directories, including a similar prefix, are clearly not this profile.
        CHECK(owner(other, profile) == ProfileOwnershipNone);
        CHECK(owner(root, profile) == ProfileOwnershipNone);
        // Unreadable or non-directory metadata, and ambiguous arguments, are unknown.
        CHECK(owner([root stringByAppendingPathComponent:@"missing"], profile) == ProfileOwnershipUnknown);
        CHECK(owner(profile, [root stringByAppendingPathComponent:@"missing"]) == ProfileOwnershipUnknown);
        CHECK(owner([root stringByAppendingPathComponent:@"file"], profile) == ProfileOwnershipUnknown);
        CHECK(owner(@"relative/独立 环境", profile) == ProfileOwnershipUnknown);
        CHECK(profileDirectoryOwnership(nil, profile) == ProfileOwnershipUnknown);

        // Synthetic process records: a failed or changed second read is unknown, never "not running".
        struct proc_bsdinfo before = {0}, same = {0}, reused = {0}, user = {0};
        before.pbi_uid = 501; before.pbi_start_tvsec = 100; before.pbi_start_tvusec = 7;
        same = before;
        reused = before; reused.pbi_start_tvusec = 8;
        user = before; user.pbi_uid = 502;
        NSArray *owned = @[@"Chromium", @"--user-data-dir", link], *foreign = @[@"Chromium", @"--user-data-dir", other];
        NSArray *ambiguous = @[@"Chromium", @"--user-data-dir", profile, @"--user-data-dir", profile], *missing = @[@"Chromium"];
        CHECK(profileOwnership(&before, YES, &same, owned, profile) == ProfileOwnershipOwned);
        CHECK(profileOwnership(&before, YES, &same, foreign, profile) == ProfileOwnershipNone);
        CHECK(profileOwnership(&before, YES, &same, ambiguous, profile) == ProfileOwnershipUnknown);
        CHECK(profileOwnership(&before, YES, &same, missing, profile) == ProfileOwnershipUnknown);
        CHECK(profileOwnership(&before, YES, &same, nil, profile) == ProfileOwnershipUnknown);
        CHECK(profileOwnership(&before, NO, &same, owned, profile) == ProfileOwnershipUnknown);
        CHECK(profileOwnership(&before, YES, &reused, owned, profile) == ProfileOwnershipUnknown);
        CHECK(profileOwnership(&before, YES, &user, owned, profile) == ProfileOwnershipUnknown);
        CHECK(missingProcess(ESRCH) == ProfileOwnershipNone);
        CHECK(missingProcess(EPERM) == ProfileOwnershipUnknown);
        CHECK(missingProcess(0) == ProfileOwnershipUnknown);

        // Real reads limited to this test process and its own reaped child; argv is compared in memory, never printed.
        CHECK(argc == 2 && strncmp(argv[1], "--user-data-dir=", 16) == 0);
        NSArray<NSString *> *actual = profileArguments(getpid());
        CHECK(actual.count == (NSUInteger)argc);
        CHECK([profileArgument(actual) isEqualToString:[NSString stringWithUTF8String:argv[1] + 16]]);
        CHECK(profileArguments(-1) == nil);
        pid_t child = fork();
        if (child == 0) _exit(0);
        CHECK(child > 0 && waitpid(child, NULL, 0) == child);
        struct proc_bsdinfo gone = {0};
        errno = 0;
        CHECK(proc_pidinfo(child, PROC_PIDTBSDINFO, 0, &gone, sizeof(gone)) != sizeof(gone) && missingProcess(errno) == ProfileOwnershipNone);

        // Locks naming a reaped PID or this non-browser process are not ownership; quit acts on neither.
        NSString *lock = [profile stringByAppendingPathComponent:@"SingletonLock"];
        CHECK([files createSymbolicLinkAtPath:lock withDestinationPath:[NSString stringWithFormat:@"host-%d", child] error:NULL]);
        CHECK([decodeResponse(rf_browser_running(profile.fileSystemRepresentation))[@"running"] isEqual:@NO]);
        CHECK([files removeItemAtPath:lock error:NULL]);
        NSString *selfLock = [NSString stringWithFormat:@"host-%d", getpid()];
        CHECK([files createSymbolicLinkAtPath:lock withDestinationPath:selfLock error:NULL]);
        CHECK([decodeResponse(rf_browser_running(profile.fileSystemRepresentation))[@"running"] isEqual:@NO]);
        CHECK([decodeResponse(rf_browser_quit(profile.fileSystemRepresentation))[@"ok"] isEqual:@YES]);
        CHECK([files removeItemAtPath:lock error:NULL]);
        CHECK([files createSymbolicLinkAtPath:lock withDestinationPath:@"host-not-a-pid" error:NULL]);
        CHECK([decodeResponse(rf_browser_running(profile.fileSystemRepresentation))[@"running"] isEqual:@NO]);
        CHECK([files removeItemAtPath:root error:NULL]);
        free(dir);
        printf("Browser profile ownership: %d checks passed\n", passed);
    }
    return 0;
}
