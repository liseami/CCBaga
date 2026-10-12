/**
 * [INPUT]: 原生 SQLite 身份目录与用户的数据保留选择。
 * [OUTPUT]: 身份/策略/账号显示信息类型、默认保留选项与界面分类。
 * [POS]: 身份管理的脱敏契约；只有明确允许的账号显示字段，不含凭据、原始文件正文或任意路径。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
export type IdentityPolicy={skills:boolean;plugins:boolean;mcp:boolean;settings:boolean;projects:boolean;history:boolean;cache:boolean};
export type Identity={id:string;name:string;saved_at:number;bytes:number;policy:IdentityPolicy};
export type CurrentIdentity={present:boolean;display_name:string|null;email:string|null;organization:string|null;unavailable:boolean};
export type IdentityState={records:Identity[];active:string;recovery:string|null;running:boolean;current:CurrentIdentity};
export const defaultPolicy:IdentityPolicy={skills:true,plugins:true,mcp:true,settings:true,projects:true,history:false,cache:false};
export const policyRows:readonly [keyof IdentityPolicy,string][]=[['skills','Skills 与命令'],['plugins','插件'],['mcp','MCP 配置'],['settings','个人设置'],['projects','项目记录'],['history','会话历史'],['cache','缓存与日志']];
