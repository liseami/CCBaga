/**
 * [INPUT]: Cromma UI、原生浏览器命令、带回报时间的真实观测事件、共用地区契约与共享活动日志和 YAML 文案。
 * [OUTPUT]: 指纹配置、独立浏览器启动、缓存/站点数据清理、目标地区与机器一致性核验、任务锁状态与 CCBaga-B 一键设为默认浏览器（系统确认）。
 * [POS]: 浏览器交互边界；只展示配置、操作与真实观测，页面加载只读，系统对齐和登录删除必须由用户选择；永久站点删除使用 AlertDialog 警告。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { Button, Input, Select, Checkbox, Panel, PanelHeader } from 'cromma-ui';
import { useEffect, useRef, useState } from 'react';
import {useI18n} from "./i18n";
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { AppDialog } from './dialog';
import { CleanupActivity, useCleanupActivity } from './cleaner-activity';
import type { BrowserConfig, BrowserState, DefaultBrowser } from './browser-model';
import { formatBytes } from './cleaner-model';
export function BrowserPanel({ onBusy }: {
    onBusy?: (busy: boolean) => void;
}) {
    const {t,locale}=useI18n();
    const [state, setState] = useState<BrowserState | null>(null), [draft, setDraft] = useState<BrowserConfig | null>(null), [busy, setBusy] = useState(''), [error, setError] = useState(''), [confirm, setConfirm] = useState<'site' | 'align' | null>(null);
    const gate = useRef(false), live = useRef(true);
    const activity = useCleanupActivity();
    const [preferred, setPreferred] = useState<DefaultBrowser | null>(null), [asking, setAsking] = useState(false), [defaultNote, setDefaultNote] = useState('');
    useEffect(() => { onBusy?.(!!busy || asking); return () => onBusy?.(false); }, [busy, asking, onBusy]);
    async function readDefault() { try {
        const next = await invoke<DefaultBrowser>('browser_default_read');
        if (live.current)
            setPreferred(next);
        return next;
    }
    catch (e) {
        if (live.current)
            setDefaultNote(String(e));
        return null;
    } }
    async function makeDefault() { setAsking(true); setDefaultNote(''); try {
        const next = await invoke<DefaultBrowser>('browser_set_default');
        setPreferred(next);
        setDefaultNote(next.is_default ? "CCBaga-B已是默认浏览器。" : "已向系统请求设为默认浏览器，请检查系统确认。");
        for (let i = 0; i < 20 && live.current; i++) {
            await new Promise(r => setTimeout(r, 1500));
            const fresh = await readDefault();
            if (fresh?.is_default) {
                setDefaultNote("CCBaga-B已设为默认浏览器。");
                break;
            }
        }
    }
    catch (e) {
        setDefaultNote(String(e));
    }
    finally {
        if (live.current)
            setAsking(false);
    } }
    async function read(initial = false) { const next = await invoke<BrowserState>('browser_read'); if (live.current) {
        setState(next);
        if (initial)
            setDraft(next.config);
    } return next; }
    useEffect(() => { live.current = true; void read(true).catch(e => setError(String(e))); void readDefault(); const event = listen('browser-report', () => { if (!gate.current)
        void read().catch(e => setError(String(e))); }); const focus = () => { void readDefault(); if (!gate.current)
        void read().catch(e => setError(String(e))); }; window.addEventListener('focus', focus); return () => { live.current = false; window.removeEventListener('focus', focus); void event.then(f => f()); }; }, []);
    async function act(command: string, args: Record<string, unknown> = {}) { if (gate.current)
        return; gate.current = true; setBusy(command); setError(''); const onProgress = activity.start('browser'); try {
        const next = await invoke<BrowserState>(command, { ...args, ...(['browser_launch', 'browser_clear', 'browser_align'].includes(command) ? { onProgress } : {}) });
        setState(next);
        if (command === 'browser_save' || command === 'browser_align')
            setDraft(next.config);
        activity.finish(true, command === 'browser_clear' ? "清理完成，未保留副本。" : command === 'browser_check' ? "机器出口已检测；浏览器结果由实测页上报。" : "操作完成。");
    }
    catch (e) {
        setError(String(e));
        activity.finish(false, String(e));
        try {
            await read();
        }
        catch {
            setState(null);
        }
    }
    finally {
        gate.current = false;
        setBusy('');
    } }
    const disabled = !!busy || !state || !draft;
    const changed = state && draft && JSON.stringify(state.config) !== JSON.stringify(draft);
    const cache = state?.cache.filter(x => !x.login) || [];
    function edit<K extends keyof BrowserConfig>(key: K, value: BrowserConfig[K]) { setDraft(old => old ? { ...old, [key]: value } : old); }
    return <section className="system-page browser-page"><div className="cleanup-title"><div><h1>{t("浏览器环境")}</h1></div><Button size="small" variant="secondary" disabled={!!busy} onClick={() => void act('browser_read')}>{t("刷新状态")}</Button></div>
    <div className="browser-default"><strong>{state?.running ? t("运行中") : t("未运行")}</strong><Button variant="secondary" size="small" disabled={asking || !preferred?.installed || preferred?.is_default} loading={asking} onClick={() => void makeDefault()}>{preferred?.is_default ? t("已是默认浏览器") : t("设为默认浏览器")}</Button></div>
    {defaultNote && <p className="system-feedback" role="status">{t(defaultNote)}</p>}
    {error && <p role="alert" className="system-feedback error">{t(error)}</p>}
    <div className="browser-actions"><Button variant="ink" disabled={disabled || !state?.engine_ready || !!changed} loading={busy === 'browser_launch'} onClick={() => void act('browser_launch')}>{state?.running ? t("重新打开实测页") : t("打开浏览器并实测")}</Button><Button variant="secondary" disabled={disabled || !state?.running} loading={busy === 'browser_quit'} onClick={() => void act('browser_quit')}>{t("正常退出浏览器")}</Button><Button variant="secondary" disabled={disabled} loading={busy === 'browser_check'} onClick={() => void act('browser_check')}>{t("检测机器出口")}</Button><Button variant="ghost" disabled={disabled || state?.running || !!changed} onClick={() => setConfirm('align')}>{t("对齐美国系统环境")}</Button></div>
    {!!changed && <p className="system-hint">{t("先保存指纹设置，再启动浏览器或对齐机器。")}</p>}
    <div className="browser-grid"><Panel className="system-card"><PanelHeader title={t("指纹与代理设置")}/><div className="system-card-body">
      <label htmlFor="fingerprint-seed">{t("稳定指纹种子")}</label><Input id="fingerprint-seed" type="number" min={1} max={4294967295} value={draft?.seed ?? ''} disabled={disabled || state?.running} onChange={e => edit('seed', Number(e.currentTarget.value))}/>
      <label htmlFor="browser-proxy">{t("浏览器代理")}</label><Input id="browser-proxy" placeholder="http://127.0.0.1:7897" value={draft?.proxy || ''} disabled={disabled || state?.running} onChange={e => edit('proxy', e.currentTarget.value)}/><p className="system-hint">{t("留空使用 TUN；指定代理须含端口。")}</p>
      <label className="browser-toggle"><Checkbox checked={draft?.follow_machine || false} disabled={disabled || state?.running} onCheckedChange={v => edit('follow_machine', v === true)}/>{t("语言与时区跟随机器")}</label>
      {!draft?.follow_machine && <><label>{t("网页语言")}</label><Select aria-label={t("浏览器语言")} value={draft?.language} disabled={disabled || state?.running} onValueChange={v => { if (v)
        edit('language', v); }} items={['en-US', 'en-CA', 'en-GB', 'en-SG', 'en-AU', 'en', 'zh-Hans', 'zh-Hant', 'fr', 'de', 'es', 'it', 'ja', 'ko'].map(v => ({ value: v, label: v }))}/><label>{t("网页时区")}</label><Select aria-label={t("浏览器时区")} value={draft?.timezone} disabled={disabled || state?.running} onValueChange={v => { if (v)
        edit('timezone', v); }} items={(state?.machine.timezones || []).map(v => ({ value: v, label: v }))}/></>}
      <label className="browser-toggle"><Checkbox checked={draft?.native_graphics || false} disabled={disabled || state?.running} onCheckedChange={v => edit('native_graphics', v === true)}/>{t("使用真实设备指纹")}</label>
      <div className="system-card-actions"><Button variant="ink" size="small" disabled={disabled || state?.running || !changed} loading={busy === 'browser_save'} onClick={() => draft && void act('browser_save', { config: draft })}>{t("保存设置")}</Button></div>
    </div></Panel><Panel className="system-card"><PanelHeader title={t("缓存与站点数据")}/><div className="system-card-body"><div className="system-current"><span>{t("可直接清理缓存")}</span><strong>{formatBytes(cache.reduce((sum, x) => sum + x.bytes, 0))}</strong><small>{cache.length}{t(" 个现存项目 · 清理前须退出浏览器")}</small></div>
      <div className="browser-cache">{state?.cache.length ? state.cache.map(x => <div key={x.path}><span>{x.path}</span><small>{x.login ? t("站点数据") : formatBytes(x.bytes)}</small></div>) : <p>{t("未发现缓存项目。")}</p>}</div>{state?.cache_error && <p role="alert">{t(state.cache_error)}</p>}
      <div className="browser-actions"><Button variant="ink" size="small" disabled={disabled || state?.running || !cache.length || !!state?.cache_error} loading={busy === 'browser_clear'} onClick={() => void act('browser_clear', { includeSiteData: false })}>{t("一键清理缓存")}</Button><Button variant="secondary" size="small" disabled={disabled || state?.running || !state?.cache.length || !!state?.cache_error} onClick={() => setConfirm('site')}>{t("清空站点数据")}</Button></div>
    </div></Panel></div>
    <Panel className="browser-checks"><PanelHeader title={t("{v0} · 实际观测", { v0: t(state?.region?.label || "自定义环境") })} actions={<span className="system-card-number">{state?.report_received_at ? t("已连接 · 回报 {v0}", { v0: new Date(state.report_received_at).toLocaleTimeString(locale==='zh'?'zh-CN':'en-US') }) : state?.running ? t("等待浏览器回报") : t("浏览器未运行")}</span>}/><div className="browser-check-grid">{state?.checks.map(x => <div className={`browser-check check-${x.status}`} key={t(x.title)}><span aria-label={t(x.status==='ok'?"已核验":x.status==='mismatch'?"存在冲突":"尚未核验")}>{x.status === 'ok' ? '✓' : x.status === 'mismatch' ? '!' : '—'}</span><div><strong>{t(x.title)}</strong><p>{t(x.detail)}</p></div></div>)}</div>{state?.report?.error && <p className="system-feedback error">{t("浏览器出口检测：")}{t(state.report.error)}</p>}</Panel>
    <CleanupActivity activity={activity.state}/>
    <AppDialog alert={confirm==='site'} className={confirm==='site'?'identity-delete-alert':undefined} open={confirm !== null} onOpenChange={v => { if (!v)
        setConfirm(null); }} title={confirm === 'site' ? t("直接清空站点数据") : t("对齐美国系统环境")} description={confirm === 'site' ? t("删除独立浏览器的缓存、Cookie、站点存储与恢复会话，不创建备份。") : t("先核验代理实际在美国，再将机器时区设为出口时区、首选语言设为 en-US、区域设为 en_US，并关闭系统定位。时区可能需要管理员授权。")} footer={<><Button variant="secondary" data-dialog-cancel onClick={() => setConfirm(null)}>{t("取消")}</Button><Button className={confirm==='site'?'identity-delete-button':undefined} variant="ink" onClick={() => { const action = confirm; setConfirm(null); void act(action === 'site' ? 'browser_clear' : 'browser_align', action === 'site' ? { includeSiteData: true } : {}); }}>{confirm === 'site' ? t("直接删除") : t("检测并应用")}</Button></>}>{confirm==='site'&&<div className="identity-delete-warning"><span aria-hidden="true">!</span><strong>{t("警告：此操作不可撤销，不会创建新备份。")}</strong></div>}<p>{confirm === 'site' ? t("书签、已保存密码和项目文件保留。") : t("不会修改代理节点。若机器与浏览器出口不同，需在代理工具中统一系统路由。更改系统语言后，部分应用需重新启动或注销。")}</p></AppDialog>
  </section>;
}
