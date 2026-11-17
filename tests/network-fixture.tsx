/**
 * [INPUT]: 真实网络页、Tauri mockIPC 与四类虚构诊断结果（四个固定域名，含 Cloudflare 浏览器质询可达）。
 * [OUTPUT]: Claude 连通性与系统 DNS 的延期成功/混合/不完整/命令异常/无效结果，检测期间由测试显式释放。
 * [POS]: 无真实连接、密钥或外部导航的 GUI 回归样本。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {createRoot} from 'react-dom/client';
import {mockIPC} from '@tauri-apps/api/mocks';
import {MotionRoot,TooltipProvider} from 'cromma-ui';
import {VpsPanel} from '../src/vps';
import {setLocale} from '../src/i18n';
import '../src/index.css';
import '../src/style.css';
let mode='success';
let release=()=>{};
let dnsMode='observed';
let releaseDns=()=>{};
Object.assign(window,{networkFixture:{setMode:(value:string)=>{mode=value;},finish:()=>release(),language:setLocale,setDnsMode:(value:string)=>{dnsMode=value;},finishDns:()=>releaseDns()}});
mockIPC(command=>{
 if(command==='network_diagnose_dns')return new Promise((resolve,reject)=>{releaseDns=()=>{
  if(dnsMode==='error')return reject('系统 DNS 查询超时，检测未完成。');
  if(dnsMode==='invalid')return resolve(null);
  const result={status:dnsMode==='partial'?'observed':dnsMode,client:{ip:'203.0.113.5',country:'United States',org:'Fixture proxy'},lookups:dnsMode==='partial'?11:12,expected_lookups:12,resolvers:[{ip:'1.1.1.1',operator:'Cloudflare',org:'Cloudflare',country:'United States',asn:13335},{ip:'2606:4700:4700::1111',operator:'Cloudflare',org:'Cloudflare',country:'United States',asn:13335}]};
  if(dnsMode==='empty')return resolve({...result,status:'observed',resolvers:[]});
  resolve(result);
 };});
 if(command!=='network_diagnose_domains')throw Error(`Unexpected fixture command: ${command}`);
 return new Promise((resolve,reject)=>{release=()=>{
  if(mode==='error')return reject('fixture connection interrupted');
  if(mode==='empty')return resolve([]);
  if(mode==='invalid')return resolve(null);
  resolve(['claude.ai','api.anthropic.com','platform.claude.com','downloads.claude.ai'].map((domain,n)=>({domain,ok:mode!=='partial'||n!==1,status_code:mode==='partial'&&n===1?403:n===0?403:200,latency_ms:35+n,ip:'203.0.113.1',message:mode==='partial'&&n===1?'受到安全拦截或限制':n===0?'服务可达，需浏览器验证':'连接正常'})));
 };});
});
setLocale('zh');
createRoot(document.getElementById('fixture')!).render(<MotionRoot><TooltipProvider><VpsPanel/></TooltipProvider></MotionRoot>);
