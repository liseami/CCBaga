/**
 * [INPUT]: Cromma UI 表单、原生电脑环境快照与经核验的设置命令与共享 YAML 文案。
 * [OUTPUT]: 独立电脑设置页面，提供时区、首选语言、定位关闭及真实执行记录、权限/钥匙串入口与任务锁状态。
 * [POS]: 系统设置的交互边界；草稿与当前状态分离，不在页面加载时修改电脑。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { Button, Input, Select, Panel, PanelHeader } from 'cromma-ui';
import { ClockDefault as ClockIcon, Translate as TranslateIcon, MapPin as MapPinIcon, Refresh as ArrowClockwiseIcon } from 'cromma-ui/pika';
import { useEffect, useRef, useState } from 'react';
import {useI18n} from "./i18n";
import { invoke } from '@tauri-apps/api/core';
import { CleanupActivity, useCleanupActivity } from './cleaner-activity';
type SystemState = {
    timezone: string;
    timezones: string[];
    automatic_timezone: boolean | null;
    languages: string[];
    location_enabled: boolean;
    accessibility: boolean;
};
type SystemResult = {
    state: SystemState;
    message: string;
};
const languages: Record<string, string> = { 'zh-Hans': '简体中文', 'zh-Hant': '繁體中文', en: 'English', fr: 'Français', de: 'Deutsch', es: 'Español', it: 'Italiano', 'pt-BR': 'Português (Brasil)', ja: '日本語', ko: '한국어', ru: 'Русский', ar: 'العربية' };
function languageChoice(code: string) { return Object.keys(languages).find(key => code === key || code.startsWith(key + '-')); }
export function SystemPanel({ onBusy }: {
    onBusy?: (busy: boolean) => void;
}) {
    const {t}=useI18n();
    const [state, setState] = useState<SystemState | null>(null);
    const [zone, setZone] = useState('');
    const [language, setLanguage] = useState('en');
    const [filter, setFilter] = useState('');
    const [busy, setBusy] = useState<string | null>(null);
    const [error, setError] = useState('');
    const busyRef = useRef(false);
    const live = useRef(true);
    const activity = useCleanupActivity();
    useEffect(() => { onBusy?.(busy !== null); return () => onBusy?.(false); }, [busy, onBusy]);
    async function refresh(initial = false) {
        const next = await invoke<SystemState>('system_read');
        if (!live.current)
            return;
        setState(next);
        if (initial) {
            setZone(next.timezone);
            setLanguage(languageChoice(next.languages[0] || 'en') || 'en');
        }
        return next;
    }
    useEffect(() => {
        live.current = true;
        void refresh(true).catch(e => { if (live.current)
            setError(String(e)); });
        const onFocus = () => { if (!busyRef.current)
            void refresh().catch(e => setError(String(e))); };
        window.addEventListener('focus', onFocus);
        return () => { live.current = false; window.removeEventListener('focus', onFocus); };
    }, []);
    async function apply(kind: 'timezone' | 'language' | 'location') {
        if (busyRef.current)
            return;
        busyRef.current = true;
        setBusy(kind);
        setError('');
        activity.start('system');
        activity.log('system', 'running', kind === 'timezone' ? `正在应用 ${zone}，请完成 macOS 管理员授权。` : kind === 'language' ? `正在保存首选语言：${languages[language]}。` : "正在关闭定位；需要时请完成系统设置授权。");
        try {
            const result = await invoke<SystemResult>(kind === 'timezone' ? 'system_set_timezone' : kind === 'language' ? 'system_set_language' : 'system_disable_location', kind === 'timezone' ? { timezone: zone } : kind === 'language' ? { language } : {});
            setState(result.state);
            const confirmed = kind !== 'location' || !result.state.location_enabled;
            activity.log('system', confirmed ? 'ok' : 'warning', result.message);
            activity.finish(confirmed, result.message);
        }
        catch (e) {
            const message = String(e);
            setError(message);
            activity.log('system', 'error', message);
            activity.finish(false, message);
            try {
                await refresh();
            }
            catch {
                setState(null);
            }
        }
        finally {
            busyRef.current = false;
            setBusy(null);
        }
    }
    async function reload() {
        if (busyRef.current)
            return;
        busyRef.current = true;
        setBusy('read');
        setError('');
        try {
            await refresh(!zone);
        }
        catch (e) {
            setState(null);
            setError(String(e));
        }
        finally {
            busyRef.current = false;
            setBusy(null);
        }
    }
    async function open(pane: string) { try {
        await invoke('system_open_settings', { pane });
    }
    catch (e) {
        setError(String(e));
    } }
    const zones = state ? Object.fromEntries(state.timezones.filter(value => value === zone || value.toLowerCase().includes(filter.toLowerCase())).map(value => [value, value.replaceAll('_', ' ')])) : {};
    const disabled = busy !== null || !state;
    return <section className="system-page">
    <div className="cleanup-title"><div><h1>{t("电脑环境")}</h1></div><Button variant="secondary" size="small" loading={busy === 'read'} disabled={busy !== null} leftIcon={<ArrowClockwiseIcon size={16}/>} onClick={() => void reload()}>{t("刷新状态")}</Button></div>
    {error && <p className="system-feedback error" role="alert">{t(error)}</p>}
    <div className="system-cards">
      <Panel className="system-card"><PanelHeader title={<span className="system-card-title"><ClockIcon size={18}/>{t("设备时区")}</span>}/><div className="system-card-body"><div className="system-current"><span>{t("当前时区")}</span><strong>{state?.timezone || t("读取中…")}</strong><small>{!state ? t("等待系统状态") : state.automatic_timezone === null ? t("自动时区状态未知") : state.automatic_timezone ? t("按位置自动设置：开启") : t("手动时区")}</small></div>
        <label htmlFor="zone-search">{t("查找时区")}</label><Input id="zone-search" size="small" aria-label={t("查找时区")} value={filter} onChange={event => setFilter(event.currentTarget.value)} placeholder={t("例如 America / Paris / Shanghai")} disabled={disabled}/>
        <label>{t("目标时区")}</label><Select aria-label={t("目标时区")} size="small" value={zone || undefined} onValueChange={value => { if (value)
        setZone(value); }} items={Object.entries(zones).map(([value, label]) => ({ value, label: t(label) }))} disabled={disabled}/><p className="system-hint">{t("需要管理员授权。")}</p>
        <div className="system-card-actions"><Button size="small" variant="ghost" disabled={busy !== null} onClick={() => void open('timezone')}>{t("系统设置")}</Button><Button variant="ink" size="small" loading={busy === 'timezone'} disabled={disabled || !zone || (zone === state?.timezone && state?.automatic_timezone === false)} onClick={() => void apply('timezone')}>{t("应用时区")}</Button></div>
      </div></Panel>
      <Panel className="system-card"><PanelHeader title={<span className="system-card-title"><TranslateIcon size={18}/>{t("系统语言")}</span>}/><div className="system-card-body"><div className="system-current"><span>{t("当前首选语言")}</span><strong>{state ? t(languages[languageChoice(state.languages[0] || '') || ''] || state.languages[0] || "未设置") : t("读取中…")}</strong><small>{state?.languages.join(' → ') || t("等待系统状态")}</small></div>
        <label>{t("目标语言")}</label><Select aria-label={t("目标语言")} size="small" value={language} onValueChange={value => { if (value)
        setLanguage(value); }} items={Object.entries(languages).map(([value, label]) => ({ value, label: t(label) }))} disabled={disabled}/><p className="system-hint">{t("应用重启后生效；系统完整切换可能需要注销。")}</p>
        <div className="system-card-actions"><Button size="small" variant="ghost" disabled={busy !== null} onClick={() => void open('language')}>{t("系统设置")}</Button><Button variant="ink" size="small" loading={busy === 'language'} disabled={disabled || language === languageChoice(state?.languages[0] || 'en')} onClick={() => void apply('language')}>{t("应用语言")}</Button></div>
      </div></Panel>
      <Panel className="system-card system-location"><PanelHeader title={<span className="system-card-title"><MapPinIcon size={18}/>{t("地理位置")}</span>}/><div className="system-card-body"><div className="system-current"><span>{t("定位服务总开关")}</span><strong>{!state ? t("读取中…") : state.location_enabled ? t("开启") : t("已关闭")}</strong></div><p className="system-hint">{state && !state.accessibility ? t("需要辅助功能权限，或在系统设置中手动关闭。") : t("需要时请完成系统授权。")}</p><div className="system-card-actions"><Button size="small" variant="ghost" disabled={busy !== null} onClick={() => void open('location')}>{t("打开定位设置")}</Button><Button variant="ink" size="small" loading={busy === 'location'} disabled={disabled || !state?.location_enabled} onClick={() => void apply('location')}>{state && !state.location_enabled ? t("已关闭定位") : t("关闭地理位置")}</Button></div></div></Panel>
    </div>
    <div className="permission-actions"><Button variant="ghost" size="small" disabled={busy!==null} loading={busy==='keychain'} onClick={async()=>{if(busyRef.current)return;busyRef.current=true;setBusy('keychain');setError('');activity.start('system');try{activity.finish(true,await invoke<string>('unlock_keychain'));}catch(e){setError(String(e));activity.finish(false,String(e));}finally{busyRef.current=false;setBusy(null);}}}>{t("解锁清理钥匙串")}</Button><Button variant="ghost" size="small" disabled={busy!==null} onClick={async()=>{try{await invoke('open_permissions');}catch(e){setError(String(e));}}}>{t("系统授权设置")}</Button></div>
    <CleanupActivity activity={activity.state}/>
  </section>;
}
