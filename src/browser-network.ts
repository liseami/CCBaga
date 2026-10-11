/**
 * [INPUT]: 宿主提供的 HTTPS 出口服务目录与浏览器 fetch；请求始终走当前浏览器路由。
 * [OUTPUT]: probeBrowserExit、parseExit，校验真实出口并在服务限流/故障时切换备用服务。
 * [POS]: 起始页外部请求的唯一边界；不借宿主代测浏览器出口，不在代理失败时切换直连。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type {Exit} from './browser-model';
function validIp(ip:string):boolean {
  if(ip.includes(':')){try{return new URL(`http://[${ip}]/`).hostname.startsWith('[');}catch{return false;}}
  return ip.split('.').length===4&&ip.split('.').every(x=>/^(0|[1-9]\d{0,2})$/.test(x)&&Number(x)<=255);
}
export function parseExit(value:unknown):Exit {
  const data=value as Record<string,unknown>|null;
  if(!data||data.success===false||data.error===true||typeof data.ip!=='string'||!validIp(data.ip)||typeof data.country_code!=='string'||!/^[A-Z]{2}$/.test(data.country_code))throw new Error('出口检测返回无效数据');
  const field=(value:unknown):string=>{if(value==null)return '';if(typeof value!=='string'||value.length>=300)throw new Error('出口字段无效');return value;};
  const zone=data.timezone as {id?:unknown}|undefined, connection=data.connection as {isp?:unknown}|undefined;
  const exit={ip:data.ip,country:data.country_code,city:field(data.city),timezone:field(typeof data.timezone==='string'?data.timezone:zone?.id),isp:field(data.org??connection?.isp)};
  return exit;
}
export async function probeBrowserExit(services:readonly string[],request:(address:string,options?:RequestInit)=>Promise<Response>=fetch):Promise<Exit> {
  const errors:string[]=[];
  for(const address of services){
    try{
      const response=await request(address,{cache:'no-store',signal:AbortSignal.timeout(8000)});
      if(!response.ok)throw new Error(`HTTP ${response.status}`);
      const text=await response.text();if(text.length>32768)throw new Error('出口响应过大');
      return parseExit(JSON.parse(text));
    }catch(e){errors.push(`${new URL(address).hostname}：${e instanceof Error?e.message:String(e)}`);}
  }
  throw new Error(`出口检测未完成（${errors.join('；')}）。请检查 TUN/代理连接后重试。`);
}
