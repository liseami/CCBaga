/**
 * [INPUT]: 用户显式提供的 VPS 地址、三文件密钥包与家宽 SOCKS5 参数、固定供应商打开命令、远端设置命令和共享活动日志与 YAML 文案。
 * [OUTPUT]: 网络环境页面的绿色家宽/蓝色 VPS 获取入口、按需选购说明、三文件导入与密钥连接检查、设置与订阅获取、远端修改确认和 TUN 使用提示。
 * [POS]: 远程设置的 UI 边界；SSH 仅提交文件路径，家宽密码只存本次组件内存，任务运行期间保留面板，订阅不写日志。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { Button, Input, Panel, PanelHeader } from 'cromma-ui';
import { useRef, useState } from 'react';
import {useI18n} from "./i18n";
import { invoke } from '@tauri-apps/api/core';
import {AppDialog} from './dialog';
import {VpsKeys, type SshKeys} from './vps-keys';
import { CleanupActivity, useCleanupActivity } from './cleaner-activity';
type Connection = {
    host: string;
    keys: SshKeys;
};
type HomeProxy = {
    host: string;
    port: number;
    username: string;
    password: string;
    expected_ip: string;
};
type Result = {
    subscription: string;
    message: string;
};
export function VpsPanel({ onBusy }: {
    onBusy: (busy: boolean) => void;
}) {
    const {t}=useI18n();
    const [connection, setConnection] = useState<Connection>({ host: '', keys: {pem:'',ppk:'',public_key:''} });
    const [home, setHome] = useState<HomeProxy>({ host: '', port: 1080, username: '', password: '', expected_ip: '' });
    const [busy, setBusy] = useState(''), [error, setError] = useState(''), [result, setResult] = useState<Result | null>(null), [copied, setCopied] = useState(false);
    const [confirm,setConfirm]=useState(false);
    const [help,setHelp]=useState<'home'|'vps'|null>(null);
    const gate = useRef(false);
    const activity = useCleanupActivity();
    async function openProvider(provider:'webshare'|'dmit') {
        try { await invoke('vps_open_provider', { provider }); }
        catch (e) { setError(String(e)); }
    }
    function sshEdit<K extends keyof Connection>(key: K, value: Connection[K]) { setConnection(old => ({ ...old, [key]: value })); setResult(null); setCopied(false); }
    function homeEdit<K extends keyof HomeProxy>(key: K, value: HomeProxy[K]) { setHome(old => ({ ...old, [key]: value })); setResult(null); setCopied(false); }
    async function run(setup: boolean) {
        if (gate.current)
            return;
        gate.current = true;
        setBusy(setup ? 'setup' : 'connect');
        onBusy(true);
        setError('');
        setResult(null);
        setCopied(false);
        const onProgress = activity.start('vps');
        try {
            if (setup) {
                const next = await invoke<Result>('vps_setup', { connection, home, onProgress });
                setResult(next);
                activity.finish(true, "VPS 自检通过，订阅地址已生成。");
            }
            else {
                const message = await invoke<string>('vps_connect', { connection, onProgress });
                activity.finish(true, message);
            }
        }
        catch (e) {
            setError(String(e));
            activity.finish(false, String(e));
        }
        finally {
            if (setup) {
                setHome(old => ({ ...old, password: '' }));
            }
            gate.current = false;
            setBusy('');
            onBusy(false);
        }
    }
    const disabled = !!busy;
    const sshReady = !!connection.host.trim() && Object.values(connection.keys).every(Boolean);
    const homeReady = !!home.host.trim() && home.port > 0 && !!home.username && !!home.password;
    return <section className="system-page vps-page"><div className="cleanup-title"><div><h1>{t("网络环境")}</h1></div></div>
    <form autoComplete="off" onSubmit={e => { e.preventDefault(); if (sshReady && homeReady)
        setConfirm(true); }}>
    <div className="browser-grid"><Panel className="system-card"><PanelHeader title={t("VPS · SSH 登录")}/><div className="system-card-body">
      <div className="provider-actions"><Button variant="info" size="small" disabled={disabled} onClick={()=>void openProvider('dmit')}>{t("获取 VPS")}</Button><Button variant="secondary" size="small" round disabled={disabled} aria-label={t("VPS 选购说明")} title={t("VPS 选购说明")} onClick={()=>setHelp('vps')}>?</Button></div>
      <label htmlFor="vps-host">{t("VPS 地址")}</label><Input id="vps-host" value={connection.host} disabled={disabled} placeholder={t("IP 或域名，不带 ssh://")} onChange={e => sshEdit('host', e.currentTarget.value.trim())}/>
      <VpsKeys keys={connection.keys} busy={disabled} onChange={keys=>{sshEdit('keys',keys);setError('');}} onError={setError}/>
      <div className="system-card-actions"><Button type="button" size="small" variant="secondary" loading={busy === 'connect'} disabled={disabled || !sshReady} onClick={() => void run(false)}>{t("检查 SSH 连接")}</Button></div>
    </div></Panel><Panel className="system-card"><PanelHeader title={t("家宽 · SOCKS5 出口")}/><div className="system-card-body">
      <div className="provider-actions"><Button variant="success" size="small" disabled={disabled} onClick={()=>void openProvider('webshare')}>{t("获取家宽")}</Button><Button variant="secondary" size="small" round disabled={disabled} aria-label={t("家宽选购说明")} title={t("家宽选购说明")} onClick={()=>setHelp('home')}>?</Button></div>
      <label htmlFor="home-host">{t("家宽地址")}</label><Input id="home-host" value={home.host} disabled={disabled} placeholder={t("家宽 SOCKS5 主机 IP 或域名")} onChange={e => homeEdit('host', e.currentTarget.value.trim())}/>
      <label htmlFor="home-port">{t("家宽端口")}</label><Input id="home-port" type="number" min={1} max={65535} value={home.port} disabled={disabled} onChange={e => homeEdit('port', Number(e.currentTarget.value))}/>
      <label htmlFor="home-user">{t("家宽账号")}</label><Input id="home-user" value={home.username} disabled={disabled} onChange={e => homeEdit('username', e.currentTarget.value)}/>
      <label htmlFor="home-password">{t("家宽密码")}</label><Input id="home-password" type="password" autoComplete="new-password" value={home.password} disabled={disabled} onChange={e => homeEdit('password', e.currentTarget.value)}/>
      <label htmlFor="home-exit">{t("预期出口 IP（可选）")}</label><Input id="home-exit" value={home.expected_ip} disabled={disabled} placeholder={t("默认使用家宽地址；域名地址需填写")} onChange={e => homeEdit('expected_ip', e.currentTarget.value.trim())}/>
    </div></Panel></div>
    
    <div className="browser-actions"><Button type="submit" variant="ink" loading={busy === 'setup'} disabled={disabled || !sshReady || !homeReady}>{t("一键设置，获得订阅链接")}</Button><span className="system-hint">{t("约 10–30 分钟")}</span></div></form>
    {error && <p role="alert" className="system-feedback error">{t(error)}</p>}
    {result && <Panel className="vps-result"><PanelHeader title={t("订阅链接")}/><div className="system-card-body"><label htmlFor="subscription">{t("私密订阅地址")}</label><Input id="subscription" readOnly value={result.subscription}/><div className="browser-actions"><Button type="button" variant="secondary" onClick={async () => { try {
        await invoke('copy_result', { text: result.subscription });
        setCopied(true);
    }
    catch (e) {
        setError(String(e));
    } }}>{copied ? t("已复制") : t("复制订阅链接")}</Button></div><strong>{t("只打开 TUN 模式，不要打开系统代理。")}</strong><p className="system-hint">{t(result.message)}</p><p className="system-hint">{t("订阅包含节点凭据，请妥善保存。")}</p></div></Panel>}
    <AppDialog open={confirm} onOpenChange={setConfirm} title={t("确认搭建网络")} description={t("Debian 12/13 · root · SSH 22。设置将更新系统、防火墙与 SSH，安装 Xray，并关闭密码登录。")} footer={<><Button variant="secondary" onClick={()=>setConfirm(false)}>{t("取消")}</Button><Button variant="ink" onClick={()=>{setConfirm(false);void run(true);}}>{t("开始搭建")}</Button></>}><code>root@{connection.host}:22</code></AppDialog>
    <AppDialog open={!!help} onOpenChange={open=>{if(!open)setHelp(null);}} title={help==='home'?t("家宽怎么买"):t("VPS 怎么选")} width={540} className="purchase-guide" footer={<Button variant="secondary" onClick={()=>setHelp(null)}>{t("知道了")}</Button>}>
      {help==='home'?<><section><h3>{t("选择套餐")}</h3><p>{t("选择 Webshare 的 Static Residential / ISP（静态住宅代理），国家与目标地区一致。固定出口使用 Direct Connection。")}</p></section><section><h3>{t("连接与认证")}</h3><p>{t("使用 SOCKS5 协议，认证选择 Username/Password。静态住宅是 ISP 分配的固定 IP，不等同于实际家庭宽带线路。")}</p></section><section><h3>{t("买完怎么填")}</h3><p>{t("Proxy Address → 家宽地址；Port → 家宽端口；Username → 家宽账号；Password → 家宽密码。端口以面板为准。")}</p></section></>:<><section><h3>{t("参考套餐")}</h3><p>{t("参考 DMIT LAX.AN4.Pro.MINI：洛杉矶，4 vCPU、4 GB 内存、80 GB SSD、5 TB 月流量。按实际需求选择配置。")}</p></section><section><h3>{t("系统与登录")}</h3><p>{t("选择 Debian 12/13、公网 IP、root 登录与默认 SSH 22 端口。只支持下载的 SSH 密钥包，不支持密码或 Agent。")}</p></section><section><h3>{t("买完怎么填")}</h3><p>{t("购买时选择「Generate SSH Key after order」。下载并解压密钥包，选择 .pem、.ppk、.pub 三个文件，再填入服务器公网 IP。")}</p></section><section><h3>{t("网络路径")}</h3><p>{t("VPS 负责中转，最终出口由右侧的家宽 SOCKS5 提供。设置成功后导入订阅，只开启 TUN 模式。")}</p></section></>}
    </AppDialog>
    <CleanupActivity activity={activity.state}/>
  </section>;
}
