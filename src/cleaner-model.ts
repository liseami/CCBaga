/**
 * [INPUT]: 原生层返回的脱敏清理快照。
 * [OUTPUT]: 分类、预设选择、跨平台路径父子去重、子项排除、重新扫描选择映射与规模格式化。
 * [POS]: 身份页复用规模格式化；旧清理快照选择仅保留纯逻辑回归。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
export type Item = {id:string;path:string;title:string;group:string;risk:string;impact:string;bytes:number;files:number;exact:boolean;keys:string[]};
export type Scan = {folder:string;items:Item[];running:boolean;processes:{pid:number;name:string}[];message:string;warnings:string[];diagnostics:{title:string;detail:string;status:string}[];org_id:string};
export type CleanResult = {cleaned:number;errors:string[];completed:boolean};
export type Preset = 'cache'|'reset'|'full';
export const groups = [
  {id:'all',label:'全部项目'}, {id:'cache',label:'缓存与日志'}, {id:'account',label:'登录与本地标识'},
  {id:'history',label:'会话与历史'}, {id:'desktop',label:'桌面端数据'},
  {id:'config',label:'代理与认证配置'}, {id:'backups',label:'既有备份'}, {id:'organization',label:'旧组织文件'},
];
export const riskLabels:Record<string,string> = {safe:'可再生成',login:'需重新登录',history:'包含历史',config:'修改配置',backup:'永久删除'};
export function presetIds(items:Item[], preset:Preset):string[] {
  return items.filter(i=>preset==='full'||i.risk==='safe'||(preset==='reset'&&i.risk==='login')).map(i=>i.id);
}
export function isDescendantPath(parent:string, child:string):boolean {
  // ----- POSIX 的反斜杠和空格是文件名；只折叠明确的 Windows 路径 -----
  const windowsPath=/^(?:[A-Za-z]:[\\/]|\\\\)/.test(parent);
  const normalize=(path:string)=>windowsPath?path.replaceAll('\\','/').replace(/\/+$/,'').toLowerCase():path.replace(/\/+$/,'');
  const a=normalize(parent),b=normalize(child);
  if(a===b)return false;
  return (a||parent==='/')&&b.startsWith(a+'/')?true:false;
}
export function effectiveItems(items:Item[], ids:string[]):Item[] {
  const selected=items.filter(i=>ids.includes(i.id));
  return selected.filter(i=>!selected.some(p=>p.id!==i.id&&isDescendantPath(p.path,i.path)));
}
export function deselectItems(items:Item[], ids:string[], removed:string[]):string[] {
  const paths=items.filter(i=>removed.includes(i.id)).map(i=>i.path);
  return ids.filter(id=>!removed.includes(id)&&!paths.some(path=>{
    const parent=items.find(i=>i.id===id);
    return !!parent&&isDescendantPath(parent.path,path);
  }));
}
// ----- 扫描编号变化后按原路径恢复选择，避免静默扩大删除范围 -----
export function reselectIds(items:Item[], ids:string[], next:Item[]):string[] {
  const paths=new Set(items.filter(i=>ids.includes(i.id)).map(i=>i.path));
  return next.filter(i=>paths.has(i.path)).map(i=>i.id);
}
export function formatBytes(bytes:number):string {
  if(!bytes)return '0 B';
  const units=['B','KB','MB','GB','TB'];
  const index=Math.min(Math.floor(Math.log(bytes)/Math.log(1024)),units.length-1);
  return `${(bytes/1024**index).toLocaleString('zh-CN',{maximumFractionDigits:index<2?0:1})} ${units[index]}`;
}
