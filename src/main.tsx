/**
 * [INPUT]: Cromma UI、共享 YAML 界面语言、清理/浏览器/系统页面及快捷操作与 VPS 面板。
 * [OUTPUT]: 极简侧栏：傻瓜设置置顶，分隔线后为网络环境、Claude 清理、浏览器环境、电脑环境；语言切换固定底部。
 * [POS]: 单窗口编排层；任务运行时锁定导航，完成后重新挂载高级页面以刷新快照。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { Panel, NavButton, MotionRoot, TooltipProvider } from 'cromma-ui';
import { EraserDefault as BroomIcon, HomeDefault as HomeIcon, WindowBrowser as BrowserIcon, Settings01 as SettingsIcon, Share01 as NetworkIcon } from 'cromma-ui/pika';
import { Fragment, useState } from 'react';
import {useI18n} from "./i18n";
import { createRoot } from 'react-dom/client';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { CleanerPanel } from './cleaner';
import { BrowserPanel } from './browser';
import { SystemPanel } from './system';
import { useQuickActions } from './quick-actions';
import { VpsPanel } from './vps';
import { LanguageSwitch } from './language-switch';
import './index.css';
import './style.css';
type Page = 'quick' | 'cleaner' | 'browser' | 'system' | 'vps';
const titles: Record<Page, string> = { quick: '傻瓜设置', cleaner: 'Claude 清理', browser: '浏览器环境', system: '电脑环境', vps: '网络环境' };
const icons: Record<Page, typeof HomeIcon> = { quick: HomeIcon, cleaner: BroomIcon, browser: BrowserIcon, system: SettingsIcon, vps: NetworkIcon };
function App() {
    const {t}=useI18n();
    const [page, setPage] = useState<Page>('quick'), [revision, setRevision] = useState(0), [vpsBusy, setVpsBusy] = useState(false), [advancedBusy, setAdvancedBusy] = useState(false);
    const quick = useQuickActions(() => setPage('quick'), () => setRevision(v => v + 1));
    const locked = quick.busy || vpsBusy || advancedBusy;
    return <><div className="drag-strip" onMouseDown={e => { if (e.button === 0)
        void getCurrentWindow().startDragging(); }}/><div className="app-shell"><aside className="app-sidebar">
    <div className="app-brand"><BroomIcon size={23}/><strong>{t("CC清理器")}</strong></div>
    <nav aria-label={t("主要页面")}>{(['quick', 'vps', 'cleaner', 'browser', 'system'] as const).map(p => { const Icon = icons[p]; return <Fragment key={p}>{p==='vps'&&<hr className="nav-divider"/>}<NavButton className="workspace-nav" active={page === p} disabled={locked} icon={<Icon size={17} aria-hidden="true"/>} onClick={() => setPage(p)}>{t(titles[p])}</NavButton></Fragment>; })}</nav>
    <div className="sidebar-bottom"><LanguageSwitch /></div>
  </aside><Panel frame className="workspace"><fieldset disabled={vpsBusy || advancedBusy} className={`cleaner-slot quick-fieldset ${page !== 'quick' ? 'page-hidden' : ''}`}>{quick.content}</fieldset>
    {page === 'cleaner' && <div className="cleaner-slot"><CleanerPanel key={revision} onBusy={setAdvancedBusy}/></div>}{page === 'browser' && <div className="cleaner-slot"><BrowserPanel key={revision} onBusy={setAdvancedBusy}/></div>}{page === 'system' && <div className="cleaner-slot"><SystemPanel key={revision} onBusy={setAdvancedBusy}/></div>}{page === 'vps' && <div className="cleaner-slot"><VpsPanel onBusy={setVpsBusy}/></div>}
  </Panel></div></>;
}
createRoot(document.getElementById('app')!).render(<MotionRoot><TooltipProvider delayDuration={600} skipDelayDuration={300}><App /></TooltipProvider></MotionRoot>);
