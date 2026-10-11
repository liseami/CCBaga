/**
 * [INPUT]: 编译期加载的中英文 YAML 目录，React 外部状态订阅与宿主提供的界面语言。
 * [OUTPUT]: translate、useI18n、setLocale；支持参数化文案和原生层消息，切换立即刷新已有结果。
 * [POS]: 两个前端入口的文案边界；源文案作稳定键，界面语言不改变机器语言、地区或网络配置。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {useSyncExternalStore} from 'react';
import zh from './locales/zh.yaml';
import en from './locales/en.yaml';
export type Locale='zh'|'en';
type Values=Record<string,string|number>;
const catalogs:Record<Locale,Record<string,string>>={zh,en};
const listeners=new Set<()=>void>();
let locale:Locale='zh';
const subscribe=(listener:()=>void)=>{listeners.add(listener);return()=>{listeners.delete(listener);};};
const escape=(text:string)=>text.replace(/[.*+?^${}()|[\]\\]/g,'\\$&');
// ----- 原生层保留原始诊断；仅在呈现边界翻译，不改路径、配置或错误契约 -----
const templates=Object.keys(catalogs.zh).filter(key=>/\{\w+\}/.test(key)).map(key=>{
  const slots=[...key.matchAll(/\{(\w+)\}/g)].map(match=>match[1]);
  const parts=key.split(/\{\w+\}/g);
  return {key,slots,specificity:parts.join('').length,pattern:new RegExp('^'+parts.map(escape).join('([\\s\\S]*?)')+'$')};
}).sort((a,b)=>b.specificity-a.specificity);
export function translate(key:string,values:Values={},language:Locale=locale):string {
  if(!key)return key;
  const entry=Object.prototype.hasOwnProperty.call(catalogs[language],key)?catalogs[language][key]:undefined;
  if(entry!==undefined)return entry.replace(/\{(\w+)\}/g,(slot,name)=>values[name]===undefined?slot:String(typeof values[name]==='string'?translate(values[name] as string,{},language):values[name]));
  if(language==='en'){
    for(const template of templates){
      const match=key.match(template.pattern);
      if(match)return translate(template.key,Object.fromEntries(template.slots.map((slot,i)=>[slot,match[i+1]])),language);
    }
    if(key.startsWith('Error: '))return 'Error: '+translate(key.slice(7),{},language);
    if(key.includes('；'))return key.split('；').map(part=>translate(part,{},language)).join('; ');
    if(key.includes('：'))return key.split('：').map(part=>translate(part,{},language)).join(': ');
    if(key.includes('\n'))return key.split('\n').map(line=>translate(line,{},language)).join('\n');
  }
  return key;
}
export function setLocale(next:unknown){
  if(next!=='zh'&&next!=='en')return;
  if(typeof document!=='undefined'){
    document.documentElement.lang=next==='zh'?'zh-CN':'en';
    document.title=translate(document.body.classList.contains('browser-surface')?'CC浏览器 · 环境核验':'CC清理器',{},next);
  }
  if(locale===next)return;
  locale=next;listeners.forEach(listener=>listener());
}
export function useI18n(){
  const language=useSyncExternalStore(subscribe,()=>locale,()=>locale);
  return {locale:language,t:(key:string,values?:Values)=>translate(key,values,language)};
}
