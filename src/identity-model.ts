/**
 * [INPUT]: 原生 SQLite 身份目录与用户的数据保留选择。
 * [OUTPUT]: 身份/策略/账号摘要/有界进程预览显示信息类型、默认保留选项、浏览器绑定和独立重开警告与界面分类；身份删除结果可含保留浏览器资料的应用目录相对路径。
 * [POS]: 身份管理的脱敏契约；只有明确允许的账号显示字段，不含凭据、原始文件正文或任意绝对路径。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
export type IdentityPolicy={skills:boolean;plugins:boolean;mcp:boolean;settings:boolean;projects:boolean;history:boolean;cache:boolean};
export type Identity={id:string;name:string;saved_at:number;bytes:number;policy:IdentityPolicy};
export type IdentitySummary={display_name?:string|null;email:string|null;organization:string|null;plan:string|null;auth_status:string|null;expires_at?:number|null};
export type ProcessPreview={pid:number;entrypoint:string;workspace:string|null};
export type CurrentIdentity={present:boolean;display_name:string|null;email:string|null;organization:string|null;plan:string|null;auth_status:string|null;expires_at?:number|null;unavailable:boolean};
export type BrowserLink={enabled:boolean;bound_ids:string[]};
export type IdentityState={browser:BrowserLink;browser_warning:string|null;browser_retained_path:string|null;records:Identity[];summaries:Record<string,IdentitySummary>;active:string;recovery:string|null;running:boolean;processes:ProcessPreview[]|null;processes_truncated:boolean;current:CurrentIdentity};
export const defaultPolicy:IdentityPolicy={skills:true,plugins:true,mcp:true,settings:true,projects:true,history:false,cache:false};
export const policyRows:readonly [keyof IdentityPolicy,string][]=[['skills','Skills 与命令'],['plugins','插件'],['mcp','MCP 配置'],['settings','个人设置'],['projects','项目记录'],['history','会话历史'],['cache','缓存与日志']];

// ----- 本地时间元数据不等于服务端认证；旧摘要或无期限数据保持未知 -----
export type CredentialStatus='local_unexpired'|'refresh_available'|'expired'|'api_key'|'unknown';
export function credentialStatus(metadata:Pick<IdentitySummary,'auth_status'|'expires_at'>|null|undefined,now=Date.now()):CredentialStatus{
 if(metadata?.auth_status==='api_key')return 'api_key';
 const expiry=metadata?.expires_at;
 if(typeof expiry!=='number'||!Number.isSafeInteger(expiry)||expiry<=0)return 'unknown';
 if(metadata?.auth_status==='local_unexpired')return expiry>now?'local_unexpired':'expired';
 if(metadata?.auth_status==='refresh_available')return 'refresh_available';
 if(metadata?.auth_status==='expired')return 'expired';
 return 'unknown';
}
