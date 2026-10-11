/**
 * [INPUT]: 原生文件选择和拖放路径、父级密钥草稿与任务锁、共享 YAML 文案。
 * [OUTPUT]: PEM/PPK/PUB 三个可选文件占位和整组导入；只向父级提交路径。
 * [POS]: VPS 的单一登录输入；不读取私钥正文，不接收密码或 Agent，实际格式与配对交给 Rust。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {useEffect, useRef, useState} from 'react';
import {Button} from 'cromma-ui';
import {invoke} from '@tauri-apps/api/core';
import {getCurrentWebview} from '@tauri-apps/api/webview';
import {useI18n} from './i18n';
export type SshKeys = {pem:string; ppk:string; public_key:string};
type Props = {keys:SshKeys; busy:boolean; onChange:(keys:SshKeys)=>void; onError:(error:string)=>void};
const files = [{field:'pem', extension:'pem'}, {field:'ppk', extension:'ppk'}, {field:'public_key', extension:'pub'}] as const;
export function VpsKeys(props:Props) {
    const {t}=useI18n();
    const current=useRef(props); current.current=props;
    const choosing=useRef(false);
    const [dragging,setDragging]=useState(false), [picking,setPicking]=useState(false);
    function accept(paths:string[]) {
        if(current.current.busy || !paths.length) return;
        const next={...current.current.keys};
        const seen=new Set<string>();
        for(const path of paths) {
            const extension=path.split('.').at(-1);
            const file=files.find(item=>item.extension===extension);
            if(!file || seen.has(file.field)) {
                current.current.onError('每种文件只选一个：.pem、.ppk、.pub。');
                return;
            }
            seen.add(file.field); next[file.field]=path;
        }
        current.current.onChange(next);
    }
    async function pick(extension:string) {
        if(choosing.current || current.current.busy) return;
        choosing.current=true; setPicking(true);
        try { accept(await invoke<string[]>('vps_choose_keys',{extension})); }
        catch(error) {current.current.onError(String(error));}
        finally {choosing.current=false;setPicking(false);}
    }
    useEffect(()=>{
        let disposed=false, cleanup:(()=>void)|undefined;
        void getCurrentWebview().onDragDropEvent(event=>{
            if(disposed) return;
            setDragging(!current.current.busy && (event.payload.type==='enter' || event.payload.type==='over'));
            if(event.payload.type==='drop') accept(event.payload.paths);
        }).then(unlisten=>{if(disposed)unlisten();else cleanup=unlisten;}).catch(error=>{
            if(!disposed)current.current.onError(String(error));
        });
        return ()=>{disposed=true;cleanup?.();};
    },[]);
    return <div className={`vps-keys ${dragging?'is-dragging':''}`}>
        <div className="vps-key-heading"><strong>{t('SSH 密钥登录')}</strong><Button size="small" variant="secondary" disabled={props.busy || picking} onClick={()=>void pick('all')}>{t('选择三个文件')}</Button></div>
        <p className="vps-key-instruction">{t('在 DMIT 选择「Generate SSH Key after order」，下载并解压密钥包，提供以下三个文件。')}</p>
        <div className="vps-key-grid">{files.map(file=>{
            const path=props.keys[file.field], name=path.split('/').at(-1);
            return <button key={file.field} type="button" className={`vps-key-file ${path?'is-selected':''}`} disabled={props.busy || picking} onClick={()=>void pick(file.extension)} aria-label={t('选择 {v0} 文件',{v0:`.${file.extension}`})} title={name || `id_rsa.${file.extension}`}>
                <svg className="vps-file-icon" viewBox="0 0 64 76" fill="none" aria-hidden="true"><path d="M10 2h30l14 14v54a4 4 0 0 1-4 4H10a4 4 0 0 1-4-4V6a4 4 0 0 1 4-4Z" fill="var(--panel)" stroke="currentColor" strokeWidth="1.5"/><path d="M40 2v14h14" stroke="currentColor" strokeWidth="1.5"/><text x="30" y="48" textAnchor="middle" fill="currentColor" fontSize="12" fontWeight="600">{file.extension.toUpperCase()}</text></svg>
                <span className="vps-file-name">{name || `id_rsa.${file.extension}`}</span><span className="vps-file-state">{path?t('已选择'):t('选择文件')}</span>
            </button>;
        })}</div>
        <span className="vps-key-drop">{dragging?t('松开以导入密钥文件'):t('也可以把三个文件拖到这里')}</span>
    </div>;
}
