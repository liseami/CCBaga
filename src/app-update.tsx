/**
 * [INPUT]: 原生 runtime_platform、Tauri 官方 updater/process 插件、应用版本、共享 YAML、Cromma UI 与父层事务锁。
 * [OUTPUT]: 侧栏检查更新、静默启动检查、真实下载进度和经签名核验的安装/重启；失败可重试；Intel 社区构建只接受完整 A+B 安装包。
 * [POS]: 原生插件校验与替换；macOS 更新 A、B 使用 DMG，Windows 先退出 B 后以 NSIS 更新两程序；Linux 使用包管理器。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { invoke } from "@tauri-apps/api/core";
import {useEffect,useRef,useState} from 'react';
import {getVersion} from '@tauri-apps/api/app';
import {check,type Update} from '@tauri-apps/plugin-updater';
import {relaunch} from '@tauri-apps/plugin-process';
import {Button,NavButton} from 'cromma-ui';
import {Refresh} from 'cromma-ui/pika';
import {AppDialog} from './dialog';
import {useI18n} from './i18n';
export function AppUpdate({blocked,onBusy}:{blocked:boolean;onBusy:(busy:boolean)=>void}){
  const {t}=useI18n();
  const [open,setOpen]=useState(false),[version,setVersion]=useState(''),[update,setUpdate]=useState<Update|null>(null);
  const [phase,setPhase]=useState<'idle'|'checking'|'current'|'available'|'downloading'|'installing'|'installed'|'restarting'|'error'>('idle');
  const [error,setError]=useState(''),[bytes,setBytes]=useState(0),[total,setTotal]=useState<number>();
  const live=useRef(false),running=useRef(false),resource=useRef<Update|null>(null),installed=useRef(false);
  const busy=phase==='downloading'||phase==='installing'||phase==='restarting';
  useEffect(()=>{
    live.current=true;
    void getVersion().then(v=>{if(live.current)setVersion(v);}).catch(e=>{if(live.current)setError(String(e));});
    void search(false);
    return()=>{live.current=false;void resource.current?.close().catch(()=>{});};
  },[]);
  async function search(show:boolean){
    if(show)setOpen(true);
    if(running.current||installed.current)return;
    running.current=true;setPhase('checking');setError('');
    try{
      const platform=await invoke<string>('runtime_platform');
      if(platform==='macos'&&await invoke<string>('runtime_arch')==='x86_64'){
        if(live.current){setPhase('idle');if(show)setError(t('此 Intel 构建请使用完整 A+B 安装包更新，官方更新源未提供 Intel 版本。'));}
        return;
      }
      if(platform === 'linux'){
        if(live.current){setPhase('idle');if(show)setError(t('Linux 版本请通过安装包或源码更新，当前更新源仅提供 macOS。'));}
        return;
      }
      const next=await check({timeout:20000});
      if(!live.current){await next?.close();return;}
      await resource.current?.close();resource.current=next;setUpdate(next);setPhase(next?'available':'current');
    }catch(e){if(live.current){setError(String(e));setPhase('error');}}
    finally{running.current=false;}
  }
  async function install(){
    if(running.current||blocked||(!update&&!installed.current))return;
    running.current=true;onBusy(true);setError('');
    try{
      if(!installed.current&&update){
        setBytes(0);setTotal(undefined);setPhase('downloading');
        await update.download(event=>{
          if(event.event==='Started')setTotal(event.data.contentLength);
          if(event.event==='Progress')setBytes(value=>value+event.data.chunkLength);
        },{timeout:600000});
        setPhase('installing');if(await invoke<string>('runtime_platform')==='windows')await invoke('browser_quit');await update.install();installed.current=true;
        await update.close().catch(()=>{});resource.current=null;
      }
      setPhase('restarting');await relaunch();
    }catch(e){setError(String(e));setPhase(installed.current?'installed':'error');}
    finally{running.current=false;onBusy(false);}
  }
  const label=installed.current?t('更新已安装，请重启'):phase==='checking'?t('正在检查更新'):update?t('发现更新 {version}',{version:update.version}):t('检查更新');
  return <>
    <NavButton className="workspace-nav update-nav" icon={<Refresh size={17} aria-hidden="true"/>} disabled={blocked||busy} onClick={()=>{if(update||installed.current)setOpen(true);else void search(true);}}>{label}</NavButton>
    {version&&<small className="app-version">v{version}</small>}
    <AppDialog open={open} onOpenChange={value=>{if(!busy)setOpen(value);}} title={t('主程序更新')} width={460} footer={<>
      <Button variant="ghost" disabled={busy} onClick={()=>setOpen(false)}>{t('关闭弹窗')}</Button>
      {update||installed.current?<Button variant="ink" disabled={blocked||busy||phase==='checking'} onClick={()=>void install()}>{installed.current?t('重启主程序'):t('更新并重启主程序')}</Button>:<Button variant="ink" disabled={phase==='checking'||busy} onClick={()=>void search(true)}>{t('重新检查')}</Button>}
    </>}>
      <dl className="update-versions"><div><dt>{t('当前版本')}</dt><dd>{version||'—'}</dd></div>{update&&<div><dt>{t('新版本')}</dt><dd>{update.version}</dd></div>}</dl>
      <p role="status" className="system-feedback">{phase==='current'?t('已是最新版本'):phase==='downloading'?t('正在下载更新'):phase==='installing'?t('正在安装更新'):phase==='restarting'?t('正在重启主程序'):label}</p>
      {phase==='downloading'&&<div className="update-progress"><progress aria-label={t('更新下载进度')} value={total?Math.min(bytes,total):undefined} max={total||1}/><span>{(bytes/1048576).toFixed(1)} MB{total?` / ${(total/1048576).toFixed(1)} MB`:''}</span></div>}
      {update?.body&&<details className="update-notes"><summary>{t('版本说明')}</summary><p>{update.body}</p></details>}
      {error&&<p role="alert" className="system-feedback error">{t(installed.current?'安装完成，但重启失败：{error}':'更新失败：{error}',{error})}</p>}
    </AppDialog>
  </>;
}
