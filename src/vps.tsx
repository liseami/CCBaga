/**
 * [INPUT]: 固定资源入口、服务规格、Claude 核心域名分流诊断、Cromma Tabs/Textarea、双语提示词与原生剪贴板。
 * [OUTPUT]: Laper 三步入口、Claude 连通性与独立系统 DNS 出口检测、五项 AI 图标分段与双语提示词一键复制。
 * [POS]: 网络资源、只读连通性与系统 DNS 观测；应用不收集密钥或执行远端部署，不修改网络配置。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {Button,Tabs,Textarea} from 'cromma-ui';
import {Server as ServerIcon,HomeDefault as HomeIcon,Refresh} from 'cromma-ui/pika';
import {useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {useI18n} from './i18n';
import {AppDialog} from './dialog';
import {agents,installationPrompt} from './network-skill';
import {DnsDiagnostics} from './dns-diagnostics';

export type DomainDiagnosis = {
    domain: string;
    ok: boolean;
    status_code: number | null;
    latency_ms: number | null;
    ip: string | null;
    message: string;
};

const claudeDomains=['claude.ai','api.anthropic.com','platform.claude.com','downloads.claude.ai'] as const;
export function VpsPanel() {
    const {t,locale}=useI18n();
    const [selected,setSelected]=useState('generic'), [copied,setCopied]=useState(''), [error,setError]=useState('');
    const [guide,setGuide]=useState<'dmit'|'webshare'|null>(null);
    const [diagnosing,setDiagnosing]=useState(false);
    const [diagnostics,setDiagnostics]=useState<DomainDiagnosis[]|null>(null);
    const [diagnosticsError,setDiagnosticsError]=useState('');
    const results=claudeDomains.map(domain=>diagnostics?.find(item=>item.domain===domain));
    const passed=results.filter(item=>item?.ok).length;
    const phase=diagnosing?'checking':diagnosticsError?'failed':diagnostics?(passed===claudeDomains.length?'passed':'failed'):'idle';
    const statusText=phase==='checking'?'检测中':phase==='passed'?'检测成功':phase==='failed'?'检测失败':'待检测';
    const prompt=installationPrompt(selected,locale);

    async function open(provider:'dmit'|'webshare'|'skill') {
        setError('');
        try { await invoke('network_open_resource',{provider}); }
        catch (e) { setError(String(e)); }
    }

    async function copy() {
        setError('');
        try { await invoke('copy_result',{text:prompt});setCopied(prompt); }
        catch(e) { setError(String(e)); }
    }

    async function runDiagnostics() {
        if(diagnosing)return;
        setDiagnosing(true);
        setError('');
        setDiagnostics(null);
        setDiagnosticsError('');
        try {
            const results = await invoke<DomainDiagnosis[]>('network_diagnose_domains');
            if(!Array.isArray(results))throw new Error('检测结果无效，请重试。');
            setDiagnostics(results);
            if(claudeDomains.some(domain=>!results.some(item=>item.domain===domain)))setDiagnosticsError('部分检测未返回结果，请重试。');
        } catch (e) {
            setDiagnosticsError(String(e));
        } finally {
            setDiagnosing(false);
        }
    }

    return <section className="system-page network-page"><div className="cleanup-title"><h1>{t('网络环境')}</h1></div>
        <ol className="network-steps">{([
            ['dmit','DMIT','获取 VPS'],['webshare','Webshare','获取家宽'],['skill','Skill','安装 Skill']
        ] as const).map(([provider,name,label],index)=><li className="network-step" key={provider}>
            <div className="network-step-heading"><span className="network-step-number" aria-hidden="true">{String(index+1).padStart(2,'0')}</span><h2>{name}</h2></div>
            <div className="network-step-actions"><Button variant={provider==='dmit'?'info':provider==='webshare'?'success':'secondary'} size="small" leftIcon={provider==='dmit'?<ServerIcon size={15} aria-hidden="true"/>:provider==='webshare'?<HomeIcon size={15} aria-hidden="true"/>:undefined} onClick={()=>void open(provider)}>{t(label)}</Button>{provider!=='skill'&&<Button variant="secondary" size="small" round aria-label={t(provider==='dmit'?'VPS 帮助':'家宽帮助')} onClick={()=>setGuide(provider)}>?</Button>}</div>
        </li>)}</ol>
        <div className="landing-section-band" aria-hidden="true"/>
        <section className="domain-diagnostics" aria-labelledby="diagnostics-title" data-status={phase} aria-busy={diagnosing}>
            <div className="domain-diagnostics-header">
                <h2 id="diagnostics-title">{t('Claude 出口')}</h2>
                <Button variant="secondary" size="small" disabled={diagnosing} leftIcon={<Refresh aria-hidden size={15} className={diagnosing?'spin':''}/>} onClick={()=>void runDiagnostics()}>{diagnosing?t('检测中'):phase==='idle'?t('开始检测'):t('重新检测')}</Button>
            </div>
            <div className={`domain-outcome ${phase}`} role="status" aria-live="polite">
                <span className="domain-outcome-symbol" aria-hidden="true">{phase==='passed'?'✓':phase==='failed'?'×':phase==='checking'?'◌':'·'}</span>
                <strong>{t(statusText)}</strong>
                {diagnostics&&!diagnosticsError&&<span>{t('已通过 {v0} / {v1} 项',{v0:passed,v1:claudeDomains.length})}</span>}
                {diagnosticsError&&<p>{t(diagnosticsError)}</p>}
            </div>
            <ul className="domain-diagnostics-list">{claudeDomains.map((domain,index)=>{
                const item=results[index];
                const status=diagnosing?'checking':item?item.ok?'passed':'failed':phase==='failed'?'failed':'idle';
                return <li key={domain} className={`domain-diagnostics-item ${status}`}>
                    <div className="domain-item-info"><span className="domain-name">{domain}</span>{item&&<span className="domain-detail">{item.status_code?`HTTP ${item.status_code} · `:''}{t(item.message)}</span>}{item?.ip&&<span className="domain-ip" title={t('连接对端，不代表出口 IP')}>{item.ip}</span>}</div>
                    <div className="domain-item-status">{item?.latency_ms!=null&&<span className="domain-latency">{item.latency_ms} ms</span>}<span className={`domain-badge ${status}`}><span aria-hidden="true">{status==='passed'?'✓':status==='failed'?'×':status==='checking'?'◌':'·'}</span>{t(status==='checking'?'检测中':item?item.ok?'检测成功':'检测失败':phase==='failed'?'未完成':'待检测')}</span></div>
                </li>;
            })}</ul>
        </section>
        <div className="landing-section-band" aria-hidden="true"/>
        <DnsDiagnostics/>
        <div className="landing-section-band" aria-hidden="true"/>
        <section className="skill-install" aria-labelledby="skill-install-title">
            <h2 id="skill-install-title">{t('Skill')}</h2>
            <Tabs className="coding-tabs" size="small" variant="segmented" value={selected} onValueChange={setSelected} tabs={agents.map(agent=>({value:agent.id,label:<span className="coding-agent"><img src={agent.icon} width={18} height={18} alt=""/>{agent.id==='generic'?t('通用'):agent.name}</span>}))}/>
            <div className="skill-prompt-surface"><div className="skill-prompt-toolbar"><span>{selected==='generic'?t('通用'):agents.find(agent=>agent.id===selected)?.name}</span><span>{locale.toUpperCase()}</span></div>
                <Textarea id="skill-prompt" aria-labelledby="skill-install-title" readOnly value={prompt} rows={3}/>
                <div className="skill-prompt-actions"><Button variant="ink" onClick={()=>void copy()}>{copied===prompt?t('已复制'):t('复制提示词')}</Button></div>
            </div>
        </section>
        <AppDialog open={guide!==null} onOpenChange={value=>{if(!value)setGuide(null);}} title={guide==='dmit'?'DMIT':'Webshare'} width={440} footer={<Button variant="ink" onClick={()=>setGuide(null)}>{t('关闭')}</Button>}>
            <dl className="resource-specs">{(guide==='dmit'?[
                ['系统','Debian 12 / 13'],['地址','IPv4'],['登录','SSH Key · root:22'],['文件','PEM · PPK · PUB'],['密钥选项','Generate SSH Key after order']
            ]:[
                ['类型','Static Residential (ISP)'],['连接','Direct Connection'],['认证','Username / Password'],['协议','SOCKS5'],['端口','以面板为准']
            ]).map(([label,value])=><div key={label}><dt>{t(label)}</dt><dd>{t(value)}</dd></div>)}</dl><p className="resource-stock-note">{t('缺货时可在原站点选择其他地区的同类套餐，或等待补货；购买前核对所需规格。')}</p>
        </AppDialog>
        {error&&<p role="alert" className="system-feedback error">{t(error)}</p>}
    </section>;
}
