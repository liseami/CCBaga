/**
 * [INPUT]: 用户主动确认身份暂存/恢复、Security 与默认 Claude Code/Desktop 凭据服务。
 * [OUTPUT]: 仅原生内部的凭据快照与恢复；正文不传 UI、不经命令列或网络。
 * [POS]: 身份保险库的 macOS 凭据适配，独立于清理器的只读属性扫描；保留系统授权。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
#import <Foundation/Foundation.h>
#import <Security/Security.h>
// ----- 测试只替换默认钥匙串获取；生产仍调用系统默认钥匙串 -----
#ifndef BAGA_IDENTITY_KEYCHAIN
#define BAGA_IDENTITY_KEYCHAIN SecKeychainCopyDefault
#endif
static char *reply(NSDictionary *value){NSData *data=[NSJSONSerialization dataWithJSONObject:value options:0 error:nil];return strdup([[NSString alloc]initWithData:data encoding:NSUTF8StringEncoding].UTF8String);}
static BOOL managed(NSString *service){return [service isEqualToString:@"Claude Code-credentials"]||[service isEqualToString:@"Claude Code"]||[service isEqualToString:@"Claude Safe Storage"];}
static char *failure(OSStatus status){return reply(@{@"error":[NSString stringWithFormat:@"身份钥匙串操作未完成（OSStatus %d）。",(int)status]});}
char *rf_identity_snapshot(void){
    SecKeychainRef keychain=NULL;
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
    OSStatus opened=BAGA_IDENTITY_KEYCHAIN(&keychain);
#pragma clang diagnostic pop
    if(opened!=errSecSuccess)return failure(opened);
    id search=CFBridgingRelease(keychain);
    NSMutableArray *saved=[NSMutableArray array];
    for(NSString *service in @[@"Claude Code-credentials",@"Claude Code",@"Claude Safe Storage"]){
        CFTypeRef raw=NULL;
        OSStatus status=SecItemCopyMatching((__bridge CFDictionaryRef)@{(__bridge id)kSecClass:(__bridge id)kSecClassGenericPassword,(__bridge id)kSecAttrService:service,(__bridge id)kSecMatchLimit:(__bridge id)kSecMatchLimitAll,(__bridge id)kSecReturnAttributes:@YES,(__bridge id)kSecReturnPersistentRef:@YES,(__bridge id)kSecMatchSearchList:@[search]},&raw);
        if(status==errSecItemNotFound)continue;if(status!=errSecSuccess)return failure(status);
        for(NSDictionary *row in CFBridgingRelease(raw)){
            NSData *reference=row[(__bridge id)kSecValuePersistentRef];CFTypeRef secret=NULL;
            status=SecItemCopyMatching((__bridge CFDictionaryRef)@{(__bridge id)kSecValuePersistentRef:reference,(__bridge id)kSecReturnData:@YES},&secret);
            if(status!=errSecSuccess)return failure(status);
            NSData *value=CFBridgingRelease(secret);
            [saved addObject:@{@"service":service,@"account":row[(__bridge id)kSecAttrAccount]?:@"",@"data":[value base64EncodedStringWithOptions:0],@"reference":[reference base64EncodedStringWithOptions:0]}];
        }
    }
    return reply(@{@"items":saved});
}
char *rf_identity_remove(const char *json){
    NSArray *items=[NSJSONSerialization JSONObjectWithData:[[NSString stringWithUTF8String:json] dataUsingEncoding:NSUTF8StringEncoding] options:0 error:nil];
    if(![items isKindOfClass:NSArray.class])return failure(errSecParam);
    for(NSDictionary *saved in items){
        if(!managed(saved[@"service"]))return failure(errSecParam);
        NSData *ref=[[NSData alloc]initWithBase64EncodedString:saved[@"reference"] options:0];CFTypeRef raw=NULL;
        OSStatus status=SecItemCopyMatching((__bridge CFDictionaryRef)@{(__bridge id)kSecValuePersistentRef:ref,(__bridge id)kSecReturnAttributes:@YES,(__bridge id)kSecReturnRef:@YES,(__bridge id)kSecReturnData:@YES},&raw);
        if(status==errSecItemNotFound)continue;if(status!=errSecSuccess)return failure(status);
        NSDictionary *row=CFBridgingRelease(raw);NSData *expected=[[NSData alloc]initWithBase64EncodedString:saved[@"data"] options:0];
        if(![row[(__bridge id)kSecAttrService] isEqual:saved[@"service"]]||![row[(__bridge id)kSecAttrAccount] isEqual:saved[@"account"]]||![row[(__bridge id)kSecValueData] isEqual:expected])return failure(errSecParam);
        if(saved[@"remaining"]){
            NSData *remaining=[[NSData alloc]initWithBase64EncodedString:saved[@"remaining"] options:0];if(!remaining)return failure(errSecParam);
            status=SecItemUpdate((__bridge CFDictionaryRef)@{(__bridge id)kSecValuePersistentRef:ref},(__bridge CFDictionaryRef)@{(__bridge id)kSecValueData:remaining});
            if(status!=errSecSuccess)return failure(status);continue;
        }
        id item=row[(__bridge id)kSecValueRef];if(!item)return failure(errSecParam);
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
        if(CFGetTypeID((__bridge CFTypeRef)item)==SecKeychainItemGetTypeID())status=SecKeychainItemDelete((__bridge SecKeychainItemRef)item);
        else status=SecItemDelete((__bridge CFDictionaryRef)@{(__bridge id)kSecClass:(__bridge id)kSecClassGenericPassword,(__bridge id)kSecMatchItemList:@[item]});
#pragma clang diagnostic pop
        if(status!=errSecSuccess)return failure(status);
    }
    return reply(@{@"ok":@YES});
}
char *rf_identity_restore(const char *json){
    SecKeychainRef keychain=NULL;
#pragma clang diagnostic push
#pragma clang diagnostic ignored "-Wdeprecated-declarations"
    OSStatus opened=BAGA_IDENTITY_KEYCHAIN(&keychain);
#pragma clang diagnostic pop
    if(opened!=errSecSuccess)return failure(opened);
    id search=CFBridgingRelease(keychain);
    NSArray *items=[NSJSONSerialization JSONObjectWithData:[[NSString stringWithUTF8String:json] dataUsingEncoding:NSUTF8StringEncoding] options:0 error:nil];
    if(![items isKindOfClass:NSArray.class])return failure(errSecParam);
    for(NSDictionary *saved in items){
        if(!managed(saved[@"service"]))return failure(errSecParam);
        NSData *data=[[NSData alloc]initWithBase64EncodedString:saved[@"data"] options:0];if(!data)return failure(errSecParam);
        NSDictionary *attrs=@{(__bridge id)kSecClass:(__bridge id)kSecClassGenericPassword,(__bridge id)kSecAttrService:saved[@"service"],(__bridge id)kSecAttrAccount:saved[@"account"],(__bridge id)kSecValueData:data,(__bridge id)kSecUseKeychain:search};
        OSStatus status=SecItemAdd((__bridge CFDictionaryRef)attrs,NULL);
        if(status==errSecDuplicateItem){
            CFTypeRef existing=NULL;NSDictionary *query=@{(__bridge id)kSecClass:(__bridge id)kSecClassGenericPassword,(__bridge id)kSecAttrService:saved[@"service"],(__bridge id)kSecAttrAccount:saved[@"account"],(__bridge id)kSecReturnData:@YES,(__bridge id)kSecMatchSearchList:@[search]};
            status=SecItemCopyMatching((__bridge CFDictionaryRef)query,&existing);NSData *value=CFBridgingRelease(existing);
            if(status==errSecSuccess&&![value isEqual:data]){
                NSData *expected=[[NSData alloc]initWithBase64EncodedString:saved[@"expected"]?:@"" options:0];
                if(expected&&[value isEqual:expected]){NSDictionary *match=@{(__bridge id)kSecClass:(__bridge id)kSecClassGenericPassword,(__bridge id)kSecAttrService:saved[@"service"],(__bridge id)kSecAttrAccount:saved[@"account"],(__bridge id)kSecMatchSearchList:@[search]};status=SecItemUpdate((__bridge CFDictionaryRef)match,(__bridge CFDictionaryRef)@{(__bridge id)kSecValueData:data});}
                else status=errSecDuplicateItem;
            }
        }
        if(status!=errSecSuccess)return failure(status);
    }
    return reply(@{@"ok":@YES});
}
