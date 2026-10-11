/*
 * [INPUT]: 独立临时钥匙串、虚构凭据与 Cleanup 原生桥。
 * [OUTPUT]: 白名单、外部应用所有者 -25244 复现、同服务账号隔离与非 Claude 拒绝检查。
 * [POS]: 原生清理回归边界；不使用登录钥匙串或真实账号。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
#import <Foundation/Foundation.h>
#import <Security/Security.h>
// 与当前 Claude 文件钥匙串保持相同实现，临时钥匙串只供隔离验证。
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
extern char *rf_credentials_remove(const char *value);
static NSDictionary *removeFixture(NSString *service,SecKeychainRef keychain,BOOL shared) {
    SecAccessRef access=NULL;SecTrustedApplicationRef system=NULL;
    NSMutableDictionary *extra=[NSMutableDictionary dictionary];
    if(shared){
        NSCAssert(SecTrustedApplicationCreateFromPath("/usr/bin/security",&system)==errSecSuccess,@"system identity failed");
        // 当前测试程序必须不在受信任列表，才能复现真实的非所有者错误。
        NSCAssert(SecAccessCreate(CFSTR("foreign-owned disposable fixture"),(__bridge CFArrayRef)@[(__bridge id)system],&access)==errSecSuccess,@"foreign access create failed");
        extra[(__bridge id)kSecAttrAccess]=(__bridge id)access;
    }
    NSDictionary *item=@{(__bridge id)kSecClass:(__bridge id)kSecClassGenericPassword,
        (__bridge id)kSecAttrService:service,(__bridge id)kSecAttrAccount:@"disposable",
        (__bridge id)kSecUseKeychain:(__bridge id)keychain,(__bridge id)kSecValueData:[@"fixture" dataUsingEncoding:NSUTF8StringEncoding],
        (__bridge id)kSecReturnPersistentRef:@YES};
    [extra addEntriesFromDictionary:item];
    CFTypeRef raw=NULL;NSCAssert(SecItemAdd((__bridge CFDictionaryRef)extra,&raw)==errSecSuccess,@"fixture create failed");
    if(access)CFRelease(access);if(system)CFRelease(system);
    NSData *ref=CFBridgingRelease(raw);NSData *sibling=nil;
    if(shared){
        NSMutableDictionary *other=[extra mutableCopy];other[(__bridge id)kSecAttrAccount]=@"preserved sibling";
        CFTypeRef siblingRaw=NULL;NSCAssert(SecItemAdd((__bridge CFDictionaryRef)other,&siblingRaw)==errSecSuccess,@"sibling create failed");
        sibling=CFBridgingRelease(siblingRaw);
    }
    if(shared)NSCAssert(SecItemDelete((__bridge CFDictionaryRef)@{(__bridge id)kSecValuePersistentRef:ref})==errSecInvalidOwnerEdit,@"shared fixture must reproduce -25244");
    char *answer=rf_credentials_remove([ref base64EncodedStringWithOptions:0].UTF8String);
    NSDictionary *result=[NSJSONSerialization JSONObjectWithData:[[NSString stringWithUTF8String:answer] dataUsingEncoding:NSUTF8StringEncoding] options:0 error:NULL];free(answer);
    NSDictionary *query=@{(__bridge id)kSecValuePersistentRef:ref,(__bridge id)kSecReturnAttributes:@YES};CFTypeRef attributes=NULL;
    OSStatus status=SecItemCopyMatching((__bridge CFDictionaryRef)query,&attributes);
    if(attributes)CFRelease(attributes);
    if([service hasPrefix:@"Claude Code-credentials-"])NSCAssert(status==errSecItemNotFound,@"fixture was not deleted: %@",result);
    else NSCAssert(status==errSecSuccess,@"unrelated fixture changed");
    if(sibling){
        CFTypeRef preserved=NULL;
        NSCAssert(SecItemCopyMatching((__bridge CFDictionaryRef)@{(__bridge id)kSecValuePersistentRef:sibling,(__bridge id)kSecReturnAttributes:@YES},&preserved)==errSecSuccess,@"same-service sibling was deleted");
        CFRelease(preserved);
    }
    return result;
}
int main(void){@autoreleasepool{
    NSString *path=[NSTemporaryDirectory() stringByAppendingPathComponent:[NSUUID.UUID.UUIDString stringByAppendingString:@".keychain-db"]];
    SecKeychainRef keychain=NULL;const char *password="disposable-only";
    NSCAssert(SecKeychainCreate(path.UTF8String,(UInt32)strlen(password),password,FALSE,NULL,&keychain)==errSecSuccess,@"isolated keychain create failed");
    @try {
        NSCAssert([removeFixture(@"Claude Code-credentials-rf-fixture",keychain,NO)[@"ok"] boolValue],@"allowed fixture delete failed");
        NSCAssert([removeFixture(@"Claude Code-credentials-rf-shared-fixture",keychain,YES)[@"ok"] boolValue],@"shared fixture delete failed");
        NSCAssert(removeFixture(@"unrelated-rf-fixture",keychain,NO)[@"error"]!=nil,@"unrelated credential accepted");
    } @finally {SecKeychainDelete(keychain);CFRelease(keychain);}
    puts("4 isolated keychain checks passed: own, foreign owner, same-service isolation, unrelated refusal");
}return 0;}
