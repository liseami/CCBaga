/**
 * [INPUT]: 原生全机/目录快照与实时进程、CLI 安装状态与管理、拖入文件夹、直接清理命令与 cleaner-model 选择规则与共享 YAML 文案。
 * [OUTPUT]: 范围隔离的 Claude 清理页、分类与搜索、完整预览、结束全部 Claude 的确认流程、CLI 卸载/重装与共享日志和任务锁状态。
 * [POS]: 本地清理主界面；只提交快照编号，与快捷入口复用同一清理事务。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { Button, Input, Checkbox, Panel, cn } from 'cromma-ui';
import { ShieldCheck as ShieldCheckIcon, Refresh as ArrowClockwiseIcon, FolderDefault as FoldersIcon, SearchDefault as MagnifyingGlassIcon, CheckTickCircle as CheckCircleIcon } from 'cromma-ui/pika';
import { useEffect, useState } from 'react';
import {useI18n} from "./i18n";
import { AppDialog } from './dialog';
import { invoke } from '@tauri-apps/api/core';
import { groups, riskLabels, effectiveItems, deselectItems, presetIds, reselectIds, formatBytes, type Scan, type Preset, type CleanResult, type Item } from './cleaner-model';
import { CleanupActivity, useCleanupActivity } from './cleaner-activity';
import { FolderScope } from './cleaner-folder';
import { useLiveProcesses } from './cleaner-processes';
type Review = {
    ids: string[];
    full: boolean;
} | null;
type CliStatus = { method: string; path: string; version: string };
type CliAction = 'uninstall' | 'reinstall';
export function CleanerPanel({ onFolderDrop, onBusy }: {
    onFolderDrop?: () => void;
    onBusy?: (busy: boolean) => void;
}) {
    const {t}=useI18n();
    const activity = useCleanupActivity();
    const [scan, setScan] = useState<Scan | null>(null);
    const [ids, setIds] = useState<string[]>([]);
    const [phase, setPhase] = useState<'scan' | 'clean' | 'stop' | CliAction | null>('scan');
    const [cli, setCli] = useState<CliStatus | null>(null);
    const [cliError, setCliError] = useState('');
    const [cliLoading, setCliLoading] = useState(true);
    const [cliReview, setCliReview] = useState<CliAction | null>(null);
    const [error, setError] = useState('');
    const [group, setGroup] = useState('all');
    const [query, setQuery] = useState('');
    const [org, setOrg] = useState('');
    const [preset, setPreset] = useState<Preset | null>('cache');
    const [review, setReview] = useState<Review>(null);
    const [stopReview, setStopReview] = useState(false);
    const [report, setReport] = useState<CleanResult | null>(null);
    const busy = phase !== null;
    useEffect(() => { onBusy?.(busy); return () => onBusy?.(false); }, [busy, onBusy]);
    const items = scan?.items || [];
    const selected = effectiveItems(items, ids);
    const unique = effectiveItems(items, items.map(i => i.id));
    const bytes = unique.reduce((n, i) => n + i.bytes, 0);
    const visible = items.filter(i => (group === 'all' || i.group === group) && `${t(i.title)} ${i.title} ${i.path} ${i.keys.join(' ')}`.toLowerCase().includes(query.toLowerCase()));
    const preview = effectiveItems(items, review?.ids || []);
    useEffect(() => { void rescan(); }, []);
    useEffect(() => { void readCli(); }, []);
    async function readCli() {
        setCliLoading(true);
        try { setCli(await invoke<CliStatus>('claude_cli_read')); setCliError(''); }
        catch (e) { setCli(null); setCliError(String(e)); }
        finally { setCliLoading(false); }
    }
    async function manageCli(action: CliAction) {
        setCliReview(null); setPhase(action); setError(''); setCliError('');
        const onProgress = activity.start('cli');
        try {
            activity.log('stop', 'running', "正在结束所有 Claude 进程。");
            const stopped = await invoke<Scan>('cleaner_stop');
            setScan(stopped); setIds(reselectIds(items, ids, stopped.items));
            if (stopped.running) throw new Error("仍有 Claude 进程在运行。");
            const status = await invoke<CliStatus>('claude_cli_manage', { action, onProgress });
            setCli(status);
            activity.finish(true, action === 'uninstall' ? "Claude Code CLI 已卸载。" : "Claude Code CLI 已重装，版本核验通过。");
        } catch (e) {
            setError(String(e)); activity.finish(false, String(e)); await readCli();
        } finally { setPhase(null); }
    }
    function accept(result: Scan, select: Preset = 'cache') {
        setScan(result);
        setIds(presetIds(result.items, select));
        setPreset(select);
    }
    async function rescan(folder: string | null = scan?.folder || null) {
        const onProgress = activity.start('scan');
        setPhase('scan');
        setError('');
        setReport(null);
        setReview(null);
        try {
            const result = await invoke<Scan>('cleaner_scan', { folder, orgId: folder ? null : org.trim() || null, onProgress });
            accept(result);
            activity.finish(true, result.message);
        }
        catch (e) {
            setScan(null);
            setIds([]);
            setError(String(e));
            activity.finish(false, String(e));
        }
        finally {
            setPhase(null);
        }
    }
    async function stop() {
        const previous = review;
        activity.start('stop');
        setStopReview(false);
        setPhase('stop');
        setError('');
        try {
            const result = await invoke<Scan>('cleaner_stop');
            accept(result);
            if (previous) {
                const nextIds = previous.full ? presetIds(result.items, 'full') : reselectIds(items, previous.ids, result.items);
                setIds(nextIds);
                setReview({ ...previous, ids: nextIds });
            }
            activity.finish(!result.running, result.message);
        }
        catch (e) {
            setError(String(e));
            activity.finish(false, String(e));
        }
        finally {
            setPhase(null);
        }
    }
    const processError = useLiveProcesses(!!review && !busy, setScan);
    function choose(mode: Preset) { setPreset(mode); setIds(presetIds(items, mode)); }
    function isSelected(item: Item) { return ids.includes(item.id) || selected.some(parent => item.path.startsWith(parent.path + '/')); }
    function toggle(id: string, checked: boolean) { setPreset(null); setIds(old => checked ? [...new Set([...old, id])] : deselectItems(items, old, [id])); }
    async function clean() {
        if (!review)
            return;
        const onProgress = activity.start('clean');
        const selectedIds = review.ids;
        setReview(null);
        setPhase('clean');
        setError('');
        try {
            const result = await invoke<CleanResult>('cleaner_clean', { ids: selectedIds, onProgress });
            setReport(result);
            setError(result.errors.join('\n'));
            setIds([]);
            setScan(null);
            activity.log('verify', 'running', "正在复查本地状态。");
            let verified = true;
            try {
                const next = await invoke<Scan>('cleaner_scan', { folder: scan?.folder || null, orgId: scan?.folder ? null : org.trim() || null, onProgress: activity.channel('verify') });
                accept(next);
            }
            catch {
                verified = false;
                activity.log('verify', 'error', "清理后复查失败，请重新扫描。");
                setError(previous => [previous, "清理后复查失败，请重新扫描。"].filter(Boolean).join('\n'));
            }
            activity.finish(result.completed && verified, verified ? (result.completed ? `已处理 ${result.cleaned} 项，复查完成。` : `已处理 ${result.cleaned} 项；部分完成，请检查失败记录。`) : "已结束操作；复查失败，请重新扫描。");
        }
        catch (e) {
            setError(String(e));
            activity.finish(false, String(e));
        }
        finally {
            setPhase(null);
        }
    }
    return <section className="cleanup-page">
    <div className="cleanup-scroll">
      <div className="cleanup-title"><h1>{scan?.folder ? t("文件夹 Claude 清理") : t("Claude 本地清理")}</h1><div className="cleanup-title-actions">
        <Button variant="secondary" size="small" disabled={busy} leftIcon={<ArrowClockwiseIcon size={16} className={phase === 'scan' ? 'spin' : ''}/>} onClick={() => void rescan()}>{phase === 'scan' ? t("正在扫描") : t("深度扫描")}</Button>
        <Button variant="ink" size="small" disabled={busy || !items.length} onClick={() => { choose('full'); setReview({ ids: presetIds(items, 'full'), full: true }); }}>{t("一键彻底清理")}</Button>
      </div></div>
      <section className="cli-management" aria-label={t("Claude Code CLI")}><div><h2>{t("Claude Code CLI")}</h2><span>{cliLoading ? t("正在检查安装状态…") : cli ? cli.method ? `${cli.version || t("已安装")} · ${cli.method.replaceAll('native', t("原生安装"))}` : t("未安装") : t("安装状态不可读取")}</span></div><div className="cleanup-title-actions"><Button variant="secondary" size="small" disabled={busy || cliLoading || !cli?.method} loading={phase === 'uninstall'} onClick={() => setCliReview('uninstall')}>{t("一键卸载")}</Button><Button variant="ink" size="small" disabled={busy || cliLoading} loading={phase === 'reinstall'} onClick={() => setCliReview('reinstall')}>{t("一键重装")}</Button></div></section>
      {cliError && <p className="cleanup-feedback error" role="alert">{t(cliError)}</p>}
      <FolderScope busy={busy} folder={scan?.folder || ''} onScan={path => { onFolderDrop?.(); setGroup('all'); setOrg(''); void rescan(path); }} onError={setError}/>
      <div className="cleanup-summary">
        <span>{t("发现 ")}<strong>{scan ? items.length : '—'}</strong>{t(" 项")}</span>
        <span title={unique.some(i => !i.exact) ? t("部分目录为下限统计") : t("父子目录已去重")}>{unique.some(i => !i.exact) ? t("至少 ") : ''}<strong>{scan ? formatBytes(bytes) : '—'}</strong></span>
        <span>{t("已选 ")}<strong>{selected.length}</strong>{t(" 项")}</span>
        <span className="cleanup-state">{!scan ? t("检查中") : scan.running ? t("请先退出 · {v0} 个进程", { v0: scan.processes.length }) : t("可以清理")}</span>
      </div>
      {error && <p className="cleanup-feedback error" role="alert">{t(error)}</p>}
      <CleanupActivity activity={activity.state}/>
      {scan?.running && <div className="process-notice"><div><strong>{t("Claude 正在使用本地数据")}</strong><p>{scan.processes.map(p => `${p.name} · ${p.pid}`).join(' / ')}</p></div><Button size="small" variant="secondary" disabled={busy} onClick={() => setStopReview(true)}>{t("结束所有 Claude 进程")}</Button></div>}
      {report && <div className={`cleanup-report ${report.completed ? 'report-ok' : 'report-partial'}`}><CheckCircleIcon size={22}/><div><strong>{report.completed ? t("清理已完成") : report.cleaned ? t("部分清理已完成") : t("清理未完成")}</strong><p>{t("已清理 ")}{report.cleaned}{t(" 项。")}{report.cleaned ? t("已完成项目直接删除，无撤回记录。") : t("没有项目完成清理，请检查失败日志。")}</p></div></div>}
      <div className="cleanup-section-heading"><h2>{t("清理范围")}</h2><div className="cleanup-presets">{([['cache', t("只清缓存")], ['reset', t("重置登录")], ['full', t("全部可清理项")]] as const).map(([id, label]) => <Button size="small" variant={preset === id ? 'selected' : 'ghost'} key={id} aria-pressed={preset === id} disabled={busy || !scan} onClick={() => choose(id)}>{t(label)}</Button>)}</div></div>
      <div className="cleanup-explorer">
        <nav className="cleanup-categories" aria-label={t("清理分类")}>{groups.map(category => <Button size="small" className="category-button" variant={group === category.id ? 'selected' : 'ghost'} key={category.id} aria-pressed={group === category.id} onClick={() => setGroup(category.id)}><span>{t(category.label)}</span><small>{category.id === 'all' ? items.length : items.filter(i => i.group === category.id).length}</small></Button>)}</nav>
        <Panel flat className="cleanup-files"><div className="cleanup-search"><MagnifyingGlassIcon size={17}/><Input aria-label={t("搜索清理项目")} placeholder={t("搜索名称、路径或配置键…")} value={query} onChange={event => setQuery(event.currentTarget.value)}/></div>
          <div className="cleanup-list-heading"><Checkbox label={t("当前列表 · {v0} 项", { v0: visible.length })} disabled={busy || !visible.length} checked={!!visible.length && visible.every(isSelected)} onCheckedChange={checked => { setPreset(null); setIds(old => checked ? [...new Set([...old, ...visible.map(i => i.id)])] : deselectItems(items, old, visible.map(i => i.id))); }}/><span>{t("内容与影响")}</span></div>
          {phase === 'scan' && !scan ? <div className="cleanup-empty"><ArrowClockwiseIcon size={30} className="spin"/><strong>{t("正在深度发现")}</strong></div> : !visible.length ? <div className="cleanup-empty"><FoldersIcon size={34}/><strong>{query ? t("没有匹配的项目") : t("这个分类没有可清理的数据")}</strong></div> : visible.map(item => <div className={cn('cleanup-file', isSelected(item) && 'selected')} key={item.id}>
            <div className="cleanup-file-top"><Checkbox label={t(item.title)} checked={isSelected(item)} disabled={busy} onCheckedChange={checked => toggle(item.id, !!checked)}/><span className={`risk-badge risk-${item.risk}`}>{t(riskLabels[item.risk] || "需核对")}</span></div>
            <code>{item.path.replace(/^\/Users\/[^/]+\//, '~/')}</code>
            <div className="cleanup-file-meta"><span>{item.bytes ? t("{v0}{v1} · {v2} 个文件", { v0: item.exact ? '' : t("至少 "), v1: formatBytes(item.bytes), v2: item.files.toLocaleString() }) : item.path.startsWith('macOS 钥匙串') ? t("凭据内容不展示") : t("空文件或目录")}</span></div>
            <details className="cleanup-file-details"><summary>{t("查看影响")}{item.keys.length ? t(" · {v0} 个配置键", { v0: item.keys.length }) : ''}</summary><p>{t(item.impact)}</p>
              {!!item.keys.length && <div className="config-keys">{item.keys.map(key => <span key={key}>{key}</span>)}</div>}
            </details>
          </div>)}
        </Panel>
      </div>
      {!scan?.folder && <div className="organization-search"><div><h3>{t("按旧组织编号发现")}</h3></div><form onSubmit={e => { e.preventDefault(); setGroup('organization'); void rescan(); }}><Input aria-label={t("旧组织编号")} placeholder={t("输入 org_id")} value={org} onChange={event => setOrg(event.currentTarget.value)} disabled={busy}/><Button size="small" variant="secondary" disabled={busy || !org.trim()} type="submit">{t("发现文件")}</Button></form>{scan?.org_id && <span className="org-current">{t("当前检索：")}{scan.org_id}</span>}</div>}
      <details className="environment-details"><summary>{t("环境检查与扫描说明 ")}<span>{scan?.warnings.length ? t("{v0} 条需留意", { v0: scan.warnings.length }) : t("本地检测")}</span></summary><div className="diagnostic-grid">{scan?.diagnostics.map(d => <div key={d.title}><strong>{t(d.title)}</strong><p>{t(d.detail)}</p></div>)}</div>{!!scan?.warnings.length && <ul>{scan.warnings.map((warning, index) => <li key={index}>{t(warning)}</li>)}</ul>}<p className="scope-note">{t("依据 claude-cleanup 白名单扩展。本机清理不能删除服务端记录或保证账号不被封。浏览器登录数据、系统代理与时区不会自动修改。")}</p></details>
    </div>
    <div className="cleanup-bottom"><div><ShieldCheckIcon size={17}/><span>{t("已选 ")}<strong>{selected.length}</strong>{t(" 项")}<span className="selection-size"> · {formatBytes(selected.reduce((n, i) => n + i.bytes, 0))}</span></span></div><div><Button variant="ink" size="small" leftIcon={phase === 'clean' ? <ArrowClockwiseIcon size={15} className="spin"/> : undefined} disabled={busy || !selected.length} onClick={() => setReview({ ids: [...ids], full: false })}>{phase === 'clean' ? t("直接清理中…") : t("清理所选项目")}</Button></div></div>
    <AppDialog open={!!cliReview} onOpenChange={open => { if (!open) setCliReview(null); }} title={cliReview === 'uninstall' ? t("卸载 Claude Code CLI？") : t("重装 Claude Code CLI？")} width={480} className="cleanup-confirm"><p>{t("将结束所有 Claude 进程并替换 CLI 程序。配置、技能、会话与项目文件保留。")}</p><div className="confirm-actions"><Button variant="ghost" size="small" onClick={() => setCliReview(null)}>{t("取消")}</Button><Button variant="ink" disabled={busy} onClick={() => { if (cliReview) void manageCli(cliReview); }}>{cliReview === 'uninstall' ? t("结束进程并卸载") : t("结束进程并重装")}</Button></div></AppDialog>
    <AppDialog open={!!review && !stopReview} onOpenChange={open => { if (!open)
        setReview(null); }} width={640} className="cleanup-confirm" title={review?.full ? t("彻底清理这些本地数据？") : t("确认所选清理范围")}><p>{t("直接删除以下 ")}{preview.length}{t(" 项，不备份、不移入废纸篓、不可撤回。配置只移除列出的键。技能、插件和实际项目文件保留。")}</p>{preview.some(i => i.risk === 'history') && <div className="confirm-notice">{t("包含本地历史和会话。清理后无法从本软件恢复。")}</div>}{preview.some(i => i.risk === 'config') && <div className="confirm-notice">{t("包含共享代理或认证配置。已开的终端不会自动更新，请清理后重开终端。")}</div>}{preview.some(i => i.path.startsWith('macOS 钥匙串')) && <div className="confirm-notice">{t("钥匙串凭据直接删除，不保留密码或撤回记录，需要重新登录。")}</div>}{preview.some(i => i.group === 'backups') && <div className="confirm-notice">{t("包含历史备份，将连同其内容永久删除。")}</div>}{scan?.running && <div className="confirm-notice">{t("检测到 ")}{scan.processes.length}{t(" 个 Claude 进程：")}{scan.processes.map(p => `${p.name} · ${p.pid}`).join(" / ")}{t("。点击下方结束进程，再核对重新扫描的清单。")}</div>}<ul>{preview.map(item => <li key={item.id}><strong>{t(item.title)}</strong><span>{t(riskLabels[item.risk])}</span><code>{item.path}</code>{!!item.keys.length && <small>{t("移除：")}{item.keys.join('、')}</small>}</li>)}</ul>{(error || processError) && <p className="system-feedback error" role="alert">{t(error || processError)}</p>}<div className="confirm-actions"><Button size="small" variant="ghost" onClick={() => setReview(null)}>{t("取消")}</Button><Button variant="ink" size="small" disabled={busy || !!processError || !preview.length} onClick={() => scan?.running ? setStopReview(true) : void clean()}>{phase === 'stop' ? t("正在结束进程并重新扫描…") : scan?.running ? t("结束所有 Claude 进程并重新扫描") : t("直接{v0}", { v0: review?.full ? t("彻底清理") : t("清理") })}</Button></div></AppDialog>
    <AppDialog open={stopReview} onOpenChange={setStopReview} width={460} className="cleanup-confirm" title={t("结束所有 Claude 进程？")}><p>{t("未保存的任务会中断。将结束当前用户的 Claude Desktop、全部辅助进程和 Claude Code（含 Node 入口），逐轮重新核验。不会关闭终端或其他应用。")}</p><div className="confirm-actions"><Button size="small" variant="ghost" onClick={() => setStopReview(false)}>{t("取消")}</Button><Button size="small" disabled={busy} onClick={() => void stop()}>{t("结束所有进程并重新扫描")}</Button></div></AppDialog>
  </section>;
}
