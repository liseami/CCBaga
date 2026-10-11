/**
 * [INPUT]: 模拟 HTTP 限流、备用服务响应和浏览器出口解析规则。
 * [OUTPUT]: 限流后切换备用服务、非法出口拒绝与全部失败原因保留的回归检查。
 * [POS]: 网络规则验证；不访问外网、不启动浏览器、不修改用户资料。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {test,expect} from 'bun:test';
import {parseExit,probeBrowserExit} from './browser-network';
test('主服务限流后经同一浏览器请求备用服务',async()=>{
  const calls:string[]=[];
  const request=(async(url:string)=>{calls.push(String(url));return calls.length===1?new Response('',{status:429}):Response.json({ip:'203.0.113.1',country_code:'US',timezone:'America/New_York',org:'fixture'});});
  const exit=await probeBrowserExit(['https://primary.test/','https://backup.test/json/'],request);
  expect(calls.length).toBe(2);expect(exit.timezone).toBe('America/New_York');expect(exit.ip).toBe('203.0.113.1');
});
test('无效 IP 与失败响应不能被当作核验成功，保留 HTTP 原因',async()=>{
  expect(()=>parseExit({success:true,ip:'999.2.3.4',country_code:'US'})).toThrow();
  expect(()=>parseExit({success:false,ip:'203.0.113.1',country_code:'US'})).toThrow();
  expect(parseExit({ip:'2001:db8::1',country_code:'JP',timezone:'Asia/Tokyo'}).ip).toBe('2001:db8::1');
  const request=(async()=>new Response('',{status:429}));
  await expect(probeBrowserExit(['https://primary.test/','https://backup.test/'],request)).rejects.toThrow('HTTP 429');
});
