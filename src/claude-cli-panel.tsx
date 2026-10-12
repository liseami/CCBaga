/**
 * [INPUT]: 原生 CLI 状态/管理命令、精确进程结束、共享确认窗和任务日志。
 * [OUTPUT]: 紧凑 CLI 安装状态、卸载/重装与确认后执行。
 * [POS]: 身份管理页的程序维护区域；程序管理保留身份保险库与用户资料。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {useEffect,useState} from 'react';
import {Button} from 'cromma-ui';
import {invoke} from '@tauri-apps/api/core';
import {useI18n} from './i18n';
import {AppDialog} from './dialog';
import {CleanupActivity,useCleanupActivity} from './cleaner-activity';
import type {Scan} from './cleaner-model';
type Status={method:string;path:string;version:string};
type Action='uninstall'|'reinstall';
export function ClaudeCliPanel({blocked,onBusy}:{blocked:boolean;onBusy:(busy:boolean)=>void}){
 const {t}=useI18n();const [status,setStatus]=useState<Status|null>(null),[loading,setLoading]=useState(true),[action,setAction]=useState<Action|null>(null),[review,setReview]=useState<Action|null>(null),[error,setError]=useState('');const activity=useCleanupActivity();
 useEffect(()=>{void read();},[]);
 useEffect(()=>{onBusy(!!action||!!review);return()=>onBusy(false);},[action,review,onBusy]);
 async function read(){setLoading(true);try{setStatus(await invoke<Status>('claude_cli_read'));}catch(e){setError(String(e));}finally{setLoading(false);}}
 async function run(){if(!review)return;const selected=review;setReview(null);setAction(selected);setError('');const onProgress=activity.start('cli');try{
  const stopped=await invoke<Scan>('cleaner_stop');if(stopped.running)throw new Error('仍有 Claude 进程在运行。');
  setStatus(await invoke<Status>('claude_cli_manage',{action:selected,onProgress}));activity.finish(true,selected==='uninstall'?'Claude Code CLI 已卸载。':'Claude Code CLI 已重装，版本核验通过。');
 }catch(e){setError(String(e));activity.finish(false,String(e));await read();}finally{setAction(null);}}
 return <section className="identity-cli"><div className="identity-section-head"><div><h2>{t('Claude Code CLI')}</h2><span className="identity-caption">{loading?t('正在检查安装状态…'):status?.method?status.version||t('已安装'):t('未安装')}</span></div><div className="cleanup-title-actions"><Button size="small" variant="secondary" disabled={blocked||loading||!!action||!status?.method} onClick={()=>setReview('uninstall')}>{t('一键卸载')}</Button><Button size="small" variant="secondary" disabled={blocked||loading||!!action} onClick={()=>setReview('reinstall')}>{t('一键重装')}</Button></div></div>{error&&<p role="alert" className="cleanup-feedback error">{t(error)}</p>}<CleanupActivity activity={activity.state}/><AppDialog open={!!review} onOpenChange={open=>{if(!open)setReview(null);}} title={review==='uninstall'?t('卸载 Claude Code CLI'):t('重装 Claude Code CLI')} description={t('将结束已识别的 Claude 进程；程序维护保留身份暂存与个人资料。')} footer={<><Button variant="secondary" onClick={()=>setReview(null)}>{t('取消')}</Button><Button variant="ink" onClick={()=>void run()}>{t('确认')}</Button></>}><div/></AppDialog></section>;
}
