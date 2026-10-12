/*
 * [INPUT]: 临时钥匙串、虚构默认/自定义/无关服务和原生 Identity 桥。
 * [OUTPUT]: 快照范围、MCP 保留、精确删除/恢复、并发变更拒绝的真实 Security API 回归。
 * [POS]: 替换默认钥匙串获取的离线验证；不改系统默认钥匙串，不使用真实账号。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
#import <Foundation/Foundation.h>
#import <Security/Security.h>
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
static SecKeychainRef fixture=NULL;
static OSStatus fixtureDefault(SecKeychainRef *out){*out=fixture;CFRetain(fixture);return errSecSuccess;}
#define BAGA_IDENTITY_KEYCHAIN fixtureDefault
#include "Identity.m"
static NSDictionary *json(char *value){NSData *bytes=[[NSString stringWithUTF8String:value] dataUsingEncoding:NSUTF8StringEncoding];free(value);return [NSJSONSerialization JSONObjectWithData:bytes options:0 error:nil];}
static NSString *payload;
static const char *input(id value){payload=[[NSString alloc]initWithData:[NSJSONSerialization dataWithJSONObject:value options:0 error:nil] encoding:NSUTF8StringEncoding];return payload.UTF8String;}
static void add(NSString *service,NSString *account,NSString *value){
    NSCAssert(SecItemAdd((__bridge CFDictionaryRef)@{(__bridge id)kSecClass:(__bridge id)kSecClassGenericPassword,(__bridge id)kSecAttrService:service,(__bridge id)kSecAttrAccount:account,(__bridge id)kSecUseKeychain:(__bridge id)fixture,(__bridge id)kSecValueData:[value dataUsingEncoding:NSUTF8StringEncoding]},NULL)==errSecSuccess,@"fixture create failed");
}
int main(void){@autoreleasepool{
    NSString *path=[NSTemporaryDirectory() stringByAppendingPathComponent:[NSUUID.UUID.UUIDString stringByAppendingString:@".keychain-db"]];
    const char *password="disposable-only";
    NSCAssert(SecKeychainCreate(path.UTF8String,(UInt32)strlen(password),password,FALSE,NULL,&fixture)==errSecSuccess,@"isolated keychain create failed");
    @try{
        add(@"Claude Code-credentials",@"first",@"{\"claudeAiOauth\":{\"accessToken\":\"fixture\"},\"mcpOAuth\":\"keep\"}");
        add(@"Claude Code-credentials",@"sibling",@"{\"claudeAiOauth\":{\"accessToken\":\"sibling\"}}");
        add(@"Claude Code",@"first",@"fixture-api");add(@"Claude Safe Storage",@"first",@"fixture-storage");
        add(@"Claude Code-credentials-custom",@"first",@"custom-kept");add(@"unrelated-fixture",@"first",@"unrelated-kept");
        NSArray *saved=json(rf_identity_snapshot())[@"items"];NSCAssert(saved.count==4,@"snapshot escaped default services");
        NSDictionary *first=nil;for(NSDictionary *row in saved)if([row[@"service"] isEqual:@"Claude Code-credentials"]&&[row[@"account"] isEqual:@"first"])first=row;
        NSCAssert(first!=nil,@"Code fixture missing");
        NSMutableDictionary *partial=[first mutableCopy];partial[@"remaining"]=[[@"{\"mcpOAuth\":\"keep\"}" dataUsingEncoding:NSUTF8StringEncoding] base64EncodedStringWithOptions:0];
        NSCAssert([json(rf_identity_remove(input(@[partial])))[@"ok"] boolValue],@"partial clear failed");
        NSCAssert(json(rf_identity_remove(input(@[first])))[@"error"]!=nil,@"changed credential was removed");
        NSMutableDictionary *restore=[first mutableCopy];restore[@"expected"]=partial[@"remaining"];
        NSCAssert([json(rf_identity_restore(input(@[restore])))[@"ok"] boolValue],@"conditional restore failed");
        NSCAssert([json(rf_identity_remove(input(@[first])))[@"ok"] boolValue],@"exact clear failed");
        NSCAssert([json(rf_identity_snapshot())[@"items"] count]==3,@"sibling was removed");
        NSCAssert([json(rf_identity_restore(input(@[first])))[@"ok"] boolValue],@"restore absent credential failed");
        NSCAssert([json(rf_identity_snapshot())[@"items"] count]==4,@"restore missing");
        NSCAssert([json(rf_identity_remove(input(saved)))[@"ok"] boolValue],@"default entries clear failed");
        NSCAssert([json(rf_identity_snapshot())[@"items"] count]==0,@"default entries remain");
        for(NSString *service in @[@"Claude Code-credentials-custom",@"unrelated-fixture"]){
            CFTypeRef raw=NULL;NSCAssert(SecItemCopyMatching((__bridge CFDictionaryRef)@{(__bridge id)kSecClass:(__bridge id)kSecClassGenericPassword,(__bridge id)kSecAttrService:service,(__bridge id)kSecMatchSearchList:@[(__bridge id)fixture],(__bridge id)kSecReturnAttributes:@YES},&raw)==errSecSuccess,@"unrelated/custom entry changed");CFRelease(raw);
        }
    }@finally{SecKeychainDelete(fixture);CFRelease(fixture);}
    puts("Isolated identity Keychain checks passed: scope, shared fields, stale refusal, exact delete, restore and unrelated preservation.");
}return 0;}
