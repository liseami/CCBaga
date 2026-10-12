/**
 * [INPUT]: Tauri 单次调用的有序进度流和清理、CLI 安装、系统和浏览器页面的操作结果与共享 YAML 文案。
 * [OUTPUT]: 本次活动状态、紧凑真实进度、实际计数与默认收起的实时日志；原始消息保留供语言切换重新呈现。
 * [POS]: 本地操作的观察层；调用通道隔离不同操作，不制造进度或改变清理范围。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { Button, Panel, cn } from 'cromma-ui';
import { Refresh as ArrowClockwiseIcon } from 'cromma-ui/pika';
import { useEffect, useRef, useState } from 'react';
import {useI18n} from "./i18n";
import { Channel } from '@tauri-apps/api/core';
export type Progress = {
    stage: string;
    status: 'running' | 'ok' | 'warning' | 'error';
    message: string;
    current: number;
    total: number | null;
    elapsed_ms: number;
};
type Kind = 'scan' | 'clean' | 'stop' | 'system' | 'browser' | 'cli' | 'identity';
type Activity = {
    kind: Kind;
    started: number;
    entries: Progress[];
    running: boolean;
    failed: boolean;
};
const labels: Record<string, string> = { delete:'直接删除',archive:'身份暂存',switch:'身份切换',identity:'身份管理',validate: '检查', clean: '逐项清理', verify: '复查', scan: '深度扫描', stop: '进程检查', system: '应用设置', browser: '浏览器操作', cli: 'CLI 安装', done: '结果' };
export function useCleanupActivity() {
    const [state, setState] = useState<Activity | null>(null);
    const sequence = useRef(0);
    const began = useRef(0);
    function append(event: Progress, token = sequence.current) {
        if (token !== sequence.current)
            return;
        setState(old => {
            if (!old)
                return old;
            const entries = [...old.entries];
            const done = entries.at(-1)?.stage === 'done' ? entries.pop() : undefined;
            entries.push(event);
            if (done)
                entries.push(done);
            return { ...old, entries: entries.slice(-2000) };
        });
    }
    function log(stage: string, status: Progress['status'], message: string, current = 0, total: number | null = null) {
        append({ stage, status, message, current, total, elapsed_ms: Math.round(performance.now() - began.current) });
    }
    function channel(remap?: string) {
        const token = sequence.current;
        return new Channel<Progress>(event => append({ ...event, stage: remap || event.stage, elapsed_ms: Math.round(performance.now() - began.current) }, token));
    }
    function start(kind: Kind) {
        sequence.current++;
        began.current = performance.now();
        setState({ kind, started: began.current, entries: [], running: true, failed: false });
        log(kind === 'clean' ? 'validate' : kind, 'running', kind === 'clean' ? "已提交所选清单，等待快照核验。" : kind === 'scan' ? "开始扫描本地数据。" : kind === 'system' ? "开始应用电脑设置。" : kind === 'browser' ? "开始浏览器操作。" : kind === 'identity' ? "开始本地身份操作。" : kind === 'cli' ? "开始 Claude Code CLI 安装管理。" : "检查并结束已确认的 Claude 进程。");
        return channel();
    }
    function finish(success: boolean, message: string) {
        log('done', success ? 'ok' : 'error', message);
        setState(old => old ? { ...old, running: false, failed: !success } : old);
    }
    return { state, start, channel, log, finish };
}
export function CleanupActivity({ activity }: {
    activity: Activity | null;
}) {
    const {t}=useI18n();
    const [open, setOpen] = useState(false);
    const [now, setNow] = useState(performance.now());
    const logBox = useRef<HTMLDivElement>(null);
    const panel = useRef<HTMLDivElement>(null);
    const follow = useRef(true);
    useEffect(() => {
        if (!activity)
            return;
        setOpen(false);
        follow.current = true;
        if (activity.kind === 'clean' || activity.kind === 'system')
            panel.current?.scrollIntoView({ block: 'nearest', behavior: 'smooth' });
    }, [activity?.started]);
    useEffect(() => {
        if (!activity?.running)
            return;
        const timer = setInterval(() => setNow(performance.now()), 200);
        return () => clearInterval(timer);
    }, [activity?.running]);
    useEffect(() => { if (follow.current && logBox.current)
        logBox.current.scrollTop = logBox.current.scrollHeight; }, [activity?.entries.length, open]);
    if (!activity)
        return null;
    const last = activity.entries.at(-1);
    const elapsed = activity.running ? Math.max(0, now - activity.started) : last?.elapsed_ms || 0;
    const lastStage = last?.stage === 'done' ? [...activity.entries].reverse().find(e => e.stage !== 'done')?.stage : last?.stage;
    const count = last?.total && last.stage !== 'done' ? `${last.current} / ${last.total}` : null;
    const determinate = activity.running && (last?.stage === 'clean' || last?.stage === 'browser') && !!last?.total;
    const fraction = determinate ? Math.min(100, 100 * (last?.current || 0) / (last?.total || 1)) : 0;
    return <Panel flat ref={panel} className={cn('cleanup-activity', activity.running && 'activity-running', activity.failed && 'activity-failed')} aria-label={t("操作活动日志")}>
    <div className="activity-header"><div>{activity.running ? <ArrowClockwiseIcon className="spin" size={17}/> : <span className="activity-result-symbol">{activity.failed ? '!' : '✓'}</span>}<strong>{activity.kind === 'clean' ? t("清理执行记录") : activity.kind === 'stop' ? t("进程操作记录") : activity.kind === 'system' ? t("电脑设置记录") : activity.kind === 'browser' ? t("浏览器操作记录") : activity.kind === 'identity' ? t("身份操作记录") : activity.kind === 'cli' ? t("CLI 安装记录") : t("扫描记录")}</strong><span className="activity-state">{activity.running ? t("运行中") : activity.failed ? t("已停止 · 请检查") : t("已完成")}</span></div><div><time>{(elapsed / 1000).toFixed(1)} s</time><Button variant="ghost" size="small" type="button" onClick={() => setOpen(!open)} aria-expanded={open}>{open ? t("收起日志") : t("展开日志")} [{activity.entries.length}]</Button></div></div>
    <div className={`activity-progress ${activity.running && !determinate ? 'indeterminate' : ''}`} role="progressbar" aria-label={t(labels[lastStage || 'scan'] || "操作进度")} aria-valuemin={0} aria-valuemax={100} aria-valuenow={determinate ? Math.round(fraction) : !activity.running && !activity.failed ? 100 : undefined}><span style={determinate ? { width: `${fraction}%` } : !activity.running && !activity.failed ? { width: '100%' } : { width: 0 }}/></div>
    <div className="activity-current" role="status" aria-live="polite"><span>{t(last?.message || "准备中…")}{activity.running && <i className="activity-cursor">_</i>}</span>{count && <strong>{count}</strong>}</div>
    {open && <div className="activity-log" ref={logBox} role="log" aria-label={t("本次操作日志")} aria-live="off" onScroll={() => { const box = logBox.current; if (box)
        follow.current = box.scrollHeight - box.scrollTop - box.clientHeight < 40; }}>{activity.entries.map((entry, i) => <div className={`log-line log-${entry.status}`} key={i}><time>+{(entry.elapsed_ms / 1000).toFixed(2)}s</time><span className="log-tag">{entry.status === 'running' ? 'RUN' : entry.status === 'ok' ? 'OK' : entry.status === 'warning' ? 'NOTE' : 'FAIL'}</span><span>{t(entry.message)}</span></div>)}</div>}
  </Panel>;
}
