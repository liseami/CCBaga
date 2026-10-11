/**
 * [INPUT]: 原生只读 cleaner_processes 命令、当前清理预览与 React 状态更新。
 * [OUTPUT]: useLiveProcesses，预览期间每两秒刷新进程状态，外部退出后解除旧扫描的锁定。
 * [POS]: 快捷首页和高级清理共用的进程状态观察；不改变文件快照、不结束进程。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {useEffect,useState,type Dispatch,type SetStateAction} from 'react';
import {invoke} from '@tauri-apps/api/core';
import type {Scan} from './cleaner-model';
export function useLiveProcesses(enabled:boolean,setScan:Dispatch<SetStateAction<Scan|null>>):string {
  const [error,setError]=useState('');
  useEffect(()=>{
    if(!enabled)return;
    let live=true;
    const refresh=async()=>{try{
      const processes=await invoke<Scan['processes']>('cleaner_processes');
      if(live){setError('');setScan(old=>old?{...old,processes,running:!!processes.length}:old);}
    }catch(e){if(live)setError(String(e));}};
    void refresh();const timer=setInterval(()=>void refresh(),2000);
    return()=>{live=false;clearInterval(timer);};
  },[enabled,setScan]);
  return error;
}
