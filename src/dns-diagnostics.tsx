/**
 * [INPUT]: 用户触发的原生系统 DNS 权威端观测与双语文案。
 * [OUTPUT]: 观测进度、完整性、实际公网与递归 DNS 出口及观测范围。
 * [POS]: 网络页独立只读观测；清除旧结果，不显示自动泄漏判断。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {useState} from 'react';
import {invoke} from '@tauri-apps/api/core';
import {Button} from 'cromma-ui';
import {Refresh} from 'cromma-ui/pika';
import {useI18n} from './i18n';

type DnsClient={ip:string;country:string|null;org:string|null};
type DnsResolver=DnsClient&{operator:string|null;asn:number|null};
type DnsDiagnosis={complete:boolean;client:DnsClient;resolvers:DnsResolver[];lookups:number;expected_lookups:number};
function isClient(value:unknown):value is DnsClient {
    return typeof value==='object'&&value!==null&&'ip' in value&&typeof value.ip==='string'&&value.ip.length>0
        &&'country' in value&&(value.country===null||typeof value.country==='string')
        &&'org' in value&&(value.org===null||typeof value.org==='string');
}
function isDiagnosis(value:unknown):value is DnsDiagnosis {
    if(typeof value!=='object'||value===null||!('complete' in value)||typeof value.complete!=='boolean'
        ||!('client' in value)||!isClient(value.client)||!('resolvers' in value)||!Array.isArray(value.resolvers)
        ||!value.resolvers.every(item=>isClient(item)&&'operator' in item&&(item.operator===null||typeof item.operator==='string')&&'asn' in item&&(item.asn===null||typeof item.asn==='number'))
        ||!('lookups' in value)||typeof value.lookups!=='number'||!Number.isInteger(value.lookups)||value.lookups<0||value.lookups>12
        ||!('expected_lookups' in value)||value.expected_lookups!==12)return false;
    return !value.complete||(value.lookups===12&&value.resolvers.length>0);
}

export function DnsDiagnostics() {
    const {t}=useI18n();
    const [running,setRunning]=useState(false),[result,setResult]=useState<DnsDiagnosis|null>(null),[error,setError]=useState('');
    const phase=running?'checking':error?'failed':result?result.complete?'complete':'partial':'idle';
    const label=phase==='checking'?'观测中':phase==='complete'?'观测完成':phase==='partial'?'观测未完成':phase==='failed'?'观测失败':'待观测';
    async function run() {
        if(running)return;
        setRunning(true);setResult(null);setError('');
        try {
            const value=await invoke<unknown>('network_diagnose_dns');
            if(!isDiagnosis(value))throw new Error('检测结果无效，请重试。');
            setResult(value);
        }catch(e){setError(String(e));}
        finally{setRunning(false);}
    }
    return <section className="dns-diagnostics" aria-labelledby="dns-diagnostics-title" data-status={phase} aria-busy={running}>
        <div className="domain-diagnostics-header">
            <h2 id="dns-diagnostics-title">{t('系统 DNS 出口观测')}</h2>
            <Button variant="secondary" size="small" disabled={running} leftIcon={<Refresh aria-hidden size={15} className={running?'spin':''}/>} onClick={()=>void run()}>{running?t('观测中'):phase==='idle'?t('开始观测'):t('重新观测')}</Button>
        </div>
        <p className="domain-detail">{t('通过系统解析器查询随机域名，由 dnsleak.dev 权威端观测解析出口。该服务会看到请求出口 IP 和测试域名，不接收账号或浏览记录。')}</p>
        <div className={`domain-outcome ${running?'checking':'idle'}`} role="status" aria-live="polite">
            <strong>{t(label)}</strong>
            {result&&<span>{t('已观测 {v0} / {v1} 个测试域名',{v0:result.lookups,v1:result.expected_lookups})}</span>}
            {error&&<p>{t(error)}</p>}
        </div>
        {result&&<>
            <p className="domain-detail">{t('公网出口')}：{result.client.ip} · {result.client.country??t('未知')} · {result.client.org??t('未知')}</p>
            {result.resolvers.length>0&&<ul className="domain-diagnostics-list" aria-label={t('DNS 实际解析服务器')}>{result.resolvers.map((resolver,index)=><li className="domain-diagnostics-item" key={`${resolver.ip}-${index}`}>
                <div className="domain-item-info"><span className="domain-name">{resolver.ip}</span><span className="domain-detail">{resolver.operator??resolver.org??t('未知')} · {resolver.country??t('未知')}{resolver.asn!==null?` · AS${resolver.asn}`:''}</span></div>
            </li>)}</ul>}
        </>}
        <p className="domain-detail">{t('本页仅展示实际观测，不自动判断 DNS 是否泄露；请核对解析服务商是否符合代理设置。地理位置为估计，IPv6 解析服务器不代表本机 IPv6 绕行；不覆盖浏览器自带的安全 DNS 或 WebRTC。')}</p>
    </section>;
}
