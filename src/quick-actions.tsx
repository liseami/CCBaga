/**
 * [INPUT]: 共用国家地区目录、浏览器/系统命令与 Claude 白名单快照与共享 YAML 文案。
 * [OUTPUT]: 傻瓜设置页的国家选择、四个带图标快捷按钮、执行记录和完整 Claude 清理预览与结束全部进程的确认流程。
 * [POS]: 傻瓜设置页的交互边界；地区与操作集中展示，清理先核验范围，执行后由宿主刷新高级页面。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { Button, Select } from 'cromma-ui';
import {WindowBrowser,Refresh,ClockDefault,EraserDefault} from 'cromma-ui/pika';
import { useEffect, useRef, useState } from 'react';
import {useI18n} from "./i18n";
import { invoke } from '@tauri-apps/api/core';
import { AppDialog } from './dialog';
import { useLiveProcesses } from './cleaner-processes';
import type { BrowserState } from './browser-model';
import { CleanupActivity, useCleanupActivity } from './cleaner-activity';
import { effectiveItems, formatBytes, type Scan, type CleanResult } from './cleaner-model';
export type Region = {
    id: string;
    label: string;
    language: string;
    locale: string;
    timezone: string;
};
export function useQuickActions(onStart: () => void, onChanged: () => void) {
    const {t}=useI18n();
    const [regions, setRegions] = useState<Region[]>([]), [region, setRegion] = useState('us-west'), [busy, setBusy] = useState(''), [error, setError] = useState(''), [review, setReview] = useState<Scan | null>(null);
    const gate = useRef(false);
    const activity = useCleanupActivity();
    const processError = useLiveProcesses(!!review && !busy, setReview);
    useEffect(() => { void invoke<Region[]>('get_regions').then(setRegions).catch(e => setError(String(e))); }, []);
    async function act(kind: 'cache' | 'fingerprint' | 'system' | 'claude') {
        if (gate.current)
            return;
        gate.current = true;
        setBusy(kind);
        setError('');
        onStart();
        const onProgress = activity.start(kind === 'system' ? 'system' : kind === 'claude' ? 'scan' : 'browser');
        try {
            if (kind === 'cache') {
                activity.log('browser', 'running', "先正常退出 CC浏览器；退出失败时停止清理。");
                const before = await invoke<BrowserState>('browser_read');
                await invoke('browser_quit');
                await invoke('browser_clear', { includeSiteData: false, onProgress });
                if (before.running)
                    await invoke('browser_launch', { onProgress });
                activity.finish(true, before.running ? "缓存已清理，登录、书签和密码保留；浏览器已重新启动。" : "缓存已清理，登录、书签和密码保留；浏览器保持关闭。");
            }
            else if (kind === 'fingerprint') {
                await invoke('browser_rotate', { region });
                activity.log('browser', 'ok', "新指纹与目标地区已保存，正在重新启动浏览器。");
                await invoke('browser_launch', { onProgress });
                activity.finish(true, "CC浏览器已使用新指纹、地区语言与时区启动，网络跟随 TUN。");
            }
            else if (kind === 'system') {
                activity.log('system', 'running', "请完成 macOS 管理员授权；时区先应用，再保存语言与地区格式。");
                const result = await invoke<{
                    message: string;
                }>('system_set_region', { region });
                activity.finish(true, `时区已核验。${result.message}`);
            }
            else {
                const scan = await invoke<Scan>('cleaner_scan', { folder: null, orgId: null, onProgress });
                if (!scan.items.length) {
                    activity.finish(true, "没有发现可清理的 Claude 本地痕迹。");
                    return;
                }
                setReview(scan);
                activity.finish(true, "已列出完整清理范围，确认后直接删除。");
            }
        }
        catch (e) {
            setError(String(e));
            activity.finish(false, String(e));
        }
        finally {
            gate.current = false;
            setBusy('');
            onChanged();
        }
    }
    async function stop() {
        if (gate.current)
            return;
        gate.current = true;
        setBusy('claude');
        setError('');
        activity.start('stop');
        try {
            const scan = await invoke<Scan>('cleaner_stop');
            setReview(scan);
            activity.finish(!scan.running, scan.message);
        }
        catch (e) {
            setError(String(e));
            activity.finish(false, String(e));
        }
        finally {
            gate.current = false;
            setBusy('');
            onChanged();
        }
    }
    async function clean() {
        if (!review || gate.current)
            return;
        const scan = review;
        setReview(null);
        gate.current = true;
        setBusy('claude');
        setError('');
        const onProgress = activity.start('clean');
        try {
            const result = await invoke<CleanResult>('cleaner_clean', { ids: scan.items.map(i => i.id), onProgress });
            if (!result.completed) {
                throw new Error(`已处理 ${result.cleaned} 项；${result.errors.join('；')}`);
            }
            const fresh = await invoke<Scan>('cleaner_scan', { folder: null, orgId: null, onProgress: activity.channel('verify') });
            activity.finish(true, `已处理 ${result.cleaned} 项，复查完成；${fresh.items.length} 项当前仍可发现。`);
        }
        catch (e) {
            setError(String(e));
            activity.finish(false, String(e));
        }
        finally {
            gate.current = false;
            setBusy('');
            onChanged();
        }
    }
    const selected = regions.find(r => r.id === region);
    const actionIcons={cache:WindowBrowser,fingerprint:Refresh,system:ClockDefault,claude:EraserDefault};
    const controls = <div className="quick-controls"><div className="quick-region"><Select aria-label={t("国家地区")} size="small" value={region} disabled={!!busy || !regions.length || !!review} onValueChange={v => { if (v)
        setRegion(v); }} items={regions.map(r => ({ value: r.id, label: t(r.label) }))}/></div><div className="quick-buttons">{([
            ['cache', t("清理浏览器缓存")], ['fingerprint', t("切换浏览器指纹")], ['system', t("重置时区、语言")], ['claude', t("清理 Claude 痕迹")]
        ] as const).map(([kind, label]) => {const Icon=actionIcons[kind];return <Button key={kind} leftIcon={<Icon size={19} aria-hidden="true"/>} variant="secondary" size="small" loading={busy === kind} disabled={!!busy || !selected || !!review} onClick={() => void act(kind)}>{t(label)}</Button>;})}</div></div>;
    const content = <section className="system-page quick-page"><div className="cleanup-title"><div><h1>{t("你想当哪国人？")}</h1></div></div>{controls}{error && <p role="alert" className="system-feedback error">{t(error)}</p>}<CleanupActivity activity={activity.state}/><AppDialog open={!!review} onOpenChange={v => { if (!v)
        setReview(null); }} title={t("清理 Claude 痕迹")} description={review?.running ? t("将结束全部已识别的 Claude 进程，未保存任务会中断；重新扫描后再次确认删除范围。") : t("下列白名单项目将直接删除，包含登录、历史与既有备份，不可撤回。")} footer={<><Button variant="secondary" onClick={() => setReview(null)}>{t("取消")}</Button><Button variant="ink" disabled={!!busy || !!processError || (!review?.running && !review?.items.length)} onClick={() => review?.running ? void stop() : void clean()}>{busy ? t("正在处理…") : review?.running ? t("结束所有 Claude 进程并重新扫描") : t("确认清理本地痕迹")}</Button></>}><div className="quick-review">{(error || processError) && <p role="alert" className="system-feedback error">{t(error || processError)}</p>}{review?.running && <p className="system-hint">{t("正在运行：")}{review.processes.map(p => `${p.name} · ${p.pid}`).join(" / ")}{t("。先结束全部进程，再确认最新清理范围。")}</p>}{review?.warnings.map(w => <p className="system-hint" key={w}>{t(w)}</p>)}{effectiveItems(review?.items || [], review?.items.map(i => i.id) || []).map(item => <div key={item.id}><strong>{t(item.title)} · {formatBytes(item.bytes)}</strong><code>{item.path}</code><small>{t(item.impact)}</small></div>)}</div></AppDialog></section>;
    return { content, busy: !!busy || !!review };
}
