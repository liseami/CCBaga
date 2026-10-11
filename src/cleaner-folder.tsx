/**
 * [INPUT]: Tauri 原生拖放事件、当前扫描目录与父级扫描事务与共享 YAML 文案。
 * [OUTPUT]: 文件夹拖入反馈、路径扫描和全机范围切换入口。
 * [POS]: 目录选择边界；只请求扫描，不直接执行删除，拖放统一进入现有清理页面。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { Button, Input } from 'cromma-ui';
import { useEffect, useRef, useState } from 'react';
import {useI18n} from "./i18n";
import { getCurrentWebview } from '@tauri-apps/api/webview';
type Props = {
    busy: boolean;
    folder: string;
    onScan: (path: string | null) => void;
    onError: (error: string) => void;
};
export function FolderScope(props: Props) {
    const {t}=useI18n();
    const current = useRef(props);
    current.current = props;
    const [path, setPath] = useState(props.folder), [dragging, setDragging] = useState(false);
    useEffect(() => { setPath(props.folder); }, [props.folder]);
    useEffect(() => {
        let disposed = false;
        let cleanup: (() => void) | undefined;
        void getCurrentWebview().onDragDropEvent(event => {
            if (disposed)
                return;
            setDragging(event.payload.type === 'enter' || event.payload.type === 'over');
            if (event.payload.type !== 'drop')
                return;
            if (current.current.busy) {
                current.current.onError(t("已有操作正在执行，请完成后再拖入文件夹。"));
                return;
            }
            if (event.payload.paths.length !== 1) {
                current.current.onError(t("每次拖入一个文件夹；扫描后可逐项选择。"));
                return;
            }
            const dropped = event.payload.paths[0];
            setPath(dropped);
            current.current.onScan(dropped);
        }).then(unlisten => { if (disposed)
            unlisten();
        else
            cleanup = unlisten; }).catch(error => { if (!disposed)
            current.current.onError(t("文件夹拖放不可用：{v0}", { v0: String(error) })); });
        return () => { disposed = true; cleanup?.(); };
    }, []);
    return <div className={`cleanup-folder ${dragging ? 'folder-dragging' : ''}`}><div><strong>{dragging ? t("松开以扫描文件夹") : t("拖入文件夹，检查 Claude 残留")}</strong></div><form onSubmit={e => { e.preventDefault(); if (path.trim())
        props.onScan(path.trim()); }}><Input aria-label={t("扫描文件夹路径")} placeholder={t("或输入文件夹的绝对路径…")} value={path} disabled={props.busy} onChange={e => setPath(e.currentTarget.value)}/><Button size="small" variant="secondary" type="submit" disabled={props.busy || !path.trim()}>{t("扫描文件夹")}</Button>{props.folder && <Button size="small" variant="ghost" disabled={props.busy} onClick={() => props.onScan(null)}>{t("返回全机扫描")}</Button>}</form>{props.folder && <code>{props.folder}</code>}</div>;
}
