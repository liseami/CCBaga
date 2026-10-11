/**
 * [INPUT]: 随机令牌回环服务上下文、浏览器原生 navigator/Intl/Permissions 与实际 HTTPS 出口与服务备援目录与宿主界面语言。
 * [OUTPUT]: 实测起始页，同步宿主中英文界面，显示本机通讯状态与回报时间，观测上报宿主；已测项目全部匹配且回传成功才显示整页绿色与大对勾。
 * [POS]: 独立 Chromium 入口，复用工具箱控件；成功状态只由真实观测决定，不接入 Tauri 权限，不伪造地理位置。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { Button, Panel, PanelHeader, MotionRoot, TooltipProvider } from 'cromma-ui';
import { useEffect, useState } from 'react';
import {translate as t,setLocale,useI18n} from "./i18n";
import { createRoot } from 'react-dom/client';
import type { BrowserContext, Report, Exit } from './browser-model';
import { probeBrowserExit } from './browser-network';
import './index.css';
import './style.css';
async function api<T>(route: string, body?: unknown): Promise<T> { const response = await fetch(new URL(route, location.href), { signal: AbortSignal.timeout(3000), ...(body ? { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(body) } : {}) }); if (!response.ok)
    throw new Error("本地核验连接失败，请从CC清理器重新打开浏览器。"); return response.json(); }
function Start() {
    const {t,locale}=useI18n();
    const [context, setContext] = useState<BrowserContext | null>(null), [report, setReport] = useState<Report | null>(null), [busy, setBusy] = useState(false), [error, setError] = useState(''), [linked, setLinked] = useState(false), [reportedAt, setReportedAt] = useState(0);
    async function measure() {
        setBusy(true);
        setError('');
        try {
            const context = await api<BrowserContext>('context');
            setContext(context);setLocale(context.ui_language);
            setLinked(true);
            const ua = (navigator as Navigator & {
                userAgentData?: {
                    getHighEntropyValues: (keys: string[]) => Promise<{
                        platformVersion?: string;
                        uaFullVersion?: string;
                    }>;
                };
            }).userAgentData;
            const high = await ua?.getHighEntropyValues(['platformVersion', 'uaFullVersion']).catch(() => null);
            const permission = await navigator.permissions.query({ name: 'geolocation' }).then(x => x.state).catch(() => 'unknown');
            let exit: Exit | null = null, networkError = '';
            try {
                exit = await probeBrowserExit(context.exit_services);
            }
            catch (e) {
                networkError = e instanceof Error ? e.message : String(e);
            }
            const observed: Report = { language: navigator.language, languages: [...navigator.languages], timezone: Intl.DateTimeFormat().resolvedOptions().timeZone, platform: navigator.platform, cores: navigator.hardwareConcurrency, user_agent: navigator.userAgent, platform_version: high?.platformVersion || '', browser_version: high?.uaFullVersion || navigator.userAgent.match(/(?:Chrome|Chromium)\/([\d.]+)/)?.[1] || '', geolocation: permission, exit, error: networkError };
            await api('report', observed);
            setReport(observed);
            setReportedAt(Date.now());
            setLinked(true);
        }
        catch (e) {
            setLinked(false);
            setError(String(e));
        }
        finally {
            setBusy(false);
        }
    }
    useEffect(() => { void measure(); let live = true; const timer = setInterval(() => { void api<BrowserContext>('context').then(next => { if (live) {
        setContext(next);setLocale(next.ui_language);
        setLinked(true);
    } }).catch(() => { if (live)
        setLinked(false); }); }, 10000); return () => { live = false; clearInterval(timer); }; }, []);
    const matching = report && context && context.region && report.exit?.country === context.region.country && report.timezone === report.exit.timezone && report.timezone === context.machine.timezone && report.language === context.region.language && report.cores === context.machine.cores && report.platform_version === context.machine.version && report.browser_version === context.engine_version && report.geolocation === 'denied' && !context.machine.location_enabled && context.machine.locale.startsWith(context.region.locale) && context.machine.languages[0]?.startsWith(context.region.language);
    const passed = Boolean(matching && !busy && linked && !error && !report?.error);
    return <div className={`browser-verification-page${passed ? ' is-verified' : ''}`}><main className="browser-start"><div className="cleanup-title"><div><h1>{t("浏览器环境")}</h1></div><Button variant="ink" loading={busy} disabled={busy} onClick={() => void measure()}>{t("重新实测")}</Button></div>
    {passed && <div className="browser-verification-success" role="status"><svg viewBox="0 0 96 96" aria-hidden="true"><circle cx="48" cy="48" r="46" fill="currentColor"/><path d="m27 49 14 14 28-31" fill="none" stroke="white" strokeWidth="8" strokeLinecap="round" strokeLinejoin="round"/></svg><strong>{t("检测通过")}</strong></div>}
    <Panel><PanelHeader title={busy ? t("正在读取浏览器与网络…") : passed ? t("已测项目与目标地区一致") : t("请核对环境差异")} actions={<span className="local-badge">Chromium {context?.engine_version || '…'}</span>}/><div className="browser-observations">{[[t("出口 IP"), report?.exit?.ip], [t("国家 / 城市"), report?.exit ? `${report.exit.country} / ${report.exit.city}` : null], [t("出口时区"), report?.exit?.timezone], [t("网页时区"), report?.timezone], [t("网页语言"), report?.languages.join(' → ')], [t("设备平台 / 版本"), report ? `${report.platform} / ${report.platform_version || t("不可读取")}` : null], [t("逻辑 CPU"), report?.cores.toString()], [t("浏览器实际版本"), report?.browser_version], [t("网页定位权限"), report?.geolocation], [t("本机时区"), context?.machine.timezone], [t("本机语言 / 区域"), context ? `${context.machine.languages.join(' → ')} / ${context.machine.locale}` : null], [t("本机系统 / CPU"), context ? t("macOS {v0} / {v1} 核", { v0: context.machine.version, v1: context.machine.cores }) : null]].map(([label, value]) => <div key={label}><span>{t(label || '')}</span><strong>{value || t("尚未核验")}</strong></div>)}</div></Panel>
    {(error || report?.error) && <p className="system-feedback error" role="alert">{t(error || report?.error || '')}</p>}
    <p className="system-hint" role="status">{linked ? t("已连接 CC清理器{v0}", { v0: reportedAt ? t(" · 结果回传成功 {v0}", { v0: new Date(reportedAt).toLocaleTimeString(locale==='zh'?'zh-CN':'en-US') }) : t(" · 等待本次实测回传") }) : t("与 CC清理器的连接已断开，请在清理器内重新打开实测页。")}</p>
  </main></div>;
}
createRoot(document.getElementById('app')!).render(<MotionRoot><TooltipProvider><Start /></TooltipProvider></MotionRoot>);
