/**
 * [INPUT]: 共享界面语言状态和宿主读写命令。
 * [OUTPUT]: 侧栏底部的中文 / EN 切换，成功写入后刷新界面，失败保留原语言并展示错误。
 * [POS]: 唯一语言写入入口；偏好由宿主持久化，并由实测页心跳读取。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {useEffect,useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {useI18n,setLocale,type Locale} from './i18n';
export function LanguageSwitch(){
  const {locale,t}=useI18n();
  const [busy,setBusy]=useState(true),[error,setError]=useState('');
  useEffect(()=>{let live=true;void invoke<Locale>('ui_language_read').then(next=>{if(live)setLocale(next);}).catch(e=>{if(live)setError(String(e));}).finally(()=>{if(live)setBusy(false);});return()=>{live=false;};},[]);
  async function change(next:Locale){if(busy||next===locale)return;setBusy(true);setError('');try{await invoke('ui_language_write',{language:next});setLocale(next);}catch(e){setError(String(e));}finally{setBusy(false);}}
  return <><div className="language-switch" role="group" aria-label={t('界面语言')}>{(['zh','en'] as const).map(value=><button key={value} type="button" aria-pressed={value===locale} disabled={busy} onClick={()=>void change(value)}>{value==='zh'?'中文':'EN'}</button>)}</div>{error&&<p className="system-feedback error" role="alert">{t(error)}</p>}</>;
}
