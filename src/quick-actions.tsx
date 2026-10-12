/**
 * [INPUT]: 共用地区、浏览器/系统命令与身份管理页入口、共享 YAML 与真实进度。
 * [OUTPUT]: 傻瓜设置的四个快捷动作；Claude 动作进入可恢复身份暂存确认。
 * [POS]: 首页操作边界；身份确认与保留选择由身份页统一管理，不调用永久删除。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {Button,Select} from 'cromma-ui';
import {WindowBrowser,Refresh,ClockDefault,ShieldCheck} from 'cromma-ui/pika';
import {useEffect,useRef,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {useI18n} from './i18n';
import type {BrowserState} from './browser-model';
import {CleanupActivity,useCleanupActivity} from './cleaner-activity';
export type Region={id:string;label:string;language:string;locale:string;timezone:string};
export function useQuickActions(onStart:()=>void,onChanged:()=>void,onIdentity:()=>void){
 const {t}=useI18n();const [regions,setRegions]=useState<Region[]>([]),[region,setRegion]=useState('us-west'),[busy,setBusy]=useState(''),[error,setError]=useState('');const gate=useRef(false);const activity=useCleanupActivity();
 useEffect(()=>{void invoke<Region[]>('get_regions').then(setRegions).catch(e=>setError(String(e)));},[]);
 async function act(kind:'cache'|'fingerprint'|'system'|'claude'){
  if(gate.current)return;if(kind==='claude'){onIdentity();return;}
  gate.current=true;setBusy(kind);setError('');onStart();const onProgress=activity.start(kind==='system'?'system':'browser');
  try{if(kind==='cache'){
   const before=await invoke<BrowserState>('browser_read');await invoke('browser_quit');await invoke('browser_clear',{includeSiteData:false,onProgress});if(before.running)await invoke('browser_launch',{onProgress});
   activity.finish(true,before.running?'缓存已清理，登录、书签和密码保留；浏览器已重新启动。':'缓存已清理，登录、书签和密码保留；浏览器保持关闭。');
  }else if(kind==='fingerprint'){await invoke('browser_rotate',{region});await invoke('browser_launch',{onProgress});activity.finish(true,'CCBaga-B已使用新指纹、地区语言与时区启动，网络跟随 TUN。');}
  else{const result=await invoke<{message:string}>('system_set_region',{region});activity.finish(true,`时区已核验。${result.message}`);}
  }catch(e){setError(String(e));activity.finish(false,String(e));}finally{gate.current=false;setBusy('');onChanged();}
 }
 const selected=regions.find(r=>r.id===region);const icons={cache:WindowBrowser,fingerprint:Refresh,system:ClockDefault,claude:ShieldCheck};
 const content=<section className="system-page quick-page"><div className="cleanup-title"><h1>{t('傻瓜設定')}</h1></div><div className="quick-controls"><div className="quick-region"><Select aria-label={t('国家地区')} size="small" value={region} disabled={!!busy||!regions.length} onValueChange={v=>{if(v)setRegion(v);}} items={regions.map(r=>({value:r.id,label:t(r.label)}))}/></div><div className="quick-buttons">{([['cache','清理浏览器缓存'],['fingerprint','切换浏览器指纹'],['system','重置时区、语言'],['claude','暂存 Claude 身份']] as const).map(([kind,label])=>{const Icon=icons[kind];return <Button key={kind} leftIcon={<Icon size={19}/>} variant="secondary" size="small" loading={busy===kind} disabled={!!busy||(kind!=='claude'&&!selected)} onClick={()=>void act(kind)}>{t(label)}</Button>;})}</div></div>{error&&<p role="alert" className="system-feedback error">{t(error)}</p>}<CleanupActivity activity={activity.state}/></section>;
 return {content,busy:!!busy};
}
