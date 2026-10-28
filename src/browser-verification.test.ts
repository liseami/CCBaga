/**
 * [INPUT]: 跨平台虚构宿主上下文和网页观测。
 * [OUTPUT]: 红/黄/绿状态、失败原因、断连/未知值与目标时区变化与 Chicago 严格 12 项核验回归。
 * [POS]: 真实页面与纯核验模型的契约测试，不调用原生权限或外部服务。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {test,expect} from 'bun:test';
import {verifyBrowser} from './browser-verification';
import type {BrowserContext,Report} from './browser-model';
const context:BrowserContext={ui_language:'zh',exit_services:[],region:{id:'us-la',label:'LA',country:'US',language:'en-US',locale:'en_US',timezone:'America/Los_Angeles'},config:{seed:1,proxy:'',follow_machine:false,language:'en-US',timezone:'America/Los_Angeles',native_graphics:true},machine:{platform:'windows',version:'10.0.26100',cores:8,timezone:'America/Los_Angeles',languages:['en-US'],locale:'en_US',locale_country:'US',location_enabled:false,timezones:[]},engine_version:'148.0.7778.215'};
const report:Report={language:'en-US',languages:['en-US'],timezone:'America/Los_Angeles',platform:'Win32',platform_version:'10.0.26100',cores:8,user_agent:'fixture',browser_version:'148.0.7778.215',geolocation:'denied',exit:{ip:'203.0.113.1',country:'US',city:'LA',timezone:'America/Los_Angeles',isp:'fixture'},error:''};
test('只有全部实测匹配且回传连接有效时才变绿，重测立即变黄',()=>{
 expect(verifyBrowser(context,report,true,'',false).phase).toBe('passed');
 expect(verifyBrowser(context,report,true,'',true).phase).toBe('checking');
 expect(verifyBrowser(context,report,false,'',false).phase).toBe('failed');
 expect(verifyBrowser(context,report,true,'report failed',false).problems).toContain('report failed');
 expect(verifyBrowser(null,null,false,'',false).phase).toBe('failed');
});
test('所有失败都有具体值或原因，未知出口与定位不能显示成功',()=>{
 const result=verifyBrowser({...context,machine:{...context.machine,location_enabled:null}},{...report,cores:4,exit:null,error:'HTTP 429'},true,'',false);
 expect(result.phase).toBe('failed');expect(result.problems).toContain('HTTP 429');
 expect(result.failures.find(row=>row.label==='逻辑 CPU')).toMatchObject({actual:'4',expected:'8'});
 expect(result.failures.find(row=>row.label==='本机定位')?.actual).toBe('尚未核验');
 expect(verifyBrowser({...context,config:{...context.config,timezone:'America/New_York'}},report,true,'',false).phase).toBe('failed');
 for(const [platform,value] of [['macos','MacIntel'],['linux','Linux x86_64']] as const){
  const ctx={...context,machine:{...context.machine,platform}};
  const observed={...report,platform:value};
  expect(verifyBrowser(ctx,observed,true,'',false).phase).toBe('passed');
 }
});

test('跟随机器按有效区域识别台湾并保留简体偏好，不放过实际冲突',()=>{
 const ctx:BrowserContext={...context,region:{id:'tw',label:'Taipei',country:'TW',language:'zh-Hant',locale:'zh_TW',timezone:'Asia/Taipei'},config:{...context.config,follow_machine:true},machine:{...context.machine,timezone:'Asia/Taipei',languages:['zh-Hans-TW','zh-Hant-TW'],locale:'zh_CN@rg=twzzzz',locale_country:'TW'}};
 const observed:Report={...report,language:'zh-Hans-TW',languages:['zh-Hans-TW','zh'],timezone:'Asia/Taipei',exit:{...report.exit!,country:'TW',timezone:'Asia/Taipei'}};
 expect(verifyBrowser(ctx,observed,true,'',false).phase).toBe('passed');
 const scriptMismatch=verifyBrowser(ctx,{...observed,language:'zh-Hant-TW'},true,'',false);
 expect(scriptMismatch.failures.find(row=>row.label==='网页语言')).toMatchObject({ok:false,expected:'zh-Hans-TW'});
 expect(verifyBrowser(ctx,{...observed,exit:{...observed.exit!,country:'CN'}},true,'',false).failures.find(row=>row.label==='出口国家')?.ok).toBe(false);
 expect(verifyBrowser({...ctx,machine:{...ctx.machine,locale_country:'CN'}},observed,true,'',false).failures.find(row=>row.label==='本机区域')?.ok).toBe(false);
 expect(verifyBrowser({...ctx,machine:{...ctx.machine,location_enabled:true}},observed,true,'',false).failures.find(row=>row.label==='本机定位')?.ok).toBe(false);
});
test('目标未知和未读到有效区域保持待测，不能冒充已知冲突或通过',()=>{
 const result=verifyBrowser({...context,region:null},report,true,'',false);
 expect(result.phase).toBe('failed');
 expect(result.checks.find(row=>row.label==='出口国家')?.ok).toBeNull();
 expect(result.checks.find(row=>row.label==='本机区域')?.ok).toBeNull();
 expect(result.checks.find(row=>row.label==='网页语言')?.ok).toBe(true);
 expect(verifyBrowser({...context,machine:{...context.machine,locale_country:''}},report,true,'',false).checks.find(row=>row.label==='本机区域')?.ok).toBeNull();
});
test('手动预设继续检查所选脚本；跟随机器保留原有 en 到 en-US 归一化',()=>{
 const ctx={...context,config:{...context.config,follow_machine:true},machine:{...context.machine,languages:['en']}};
 expect(verifyBrowser(ctx,report,true,'',false).phase).toBe('passed');
 const manual={...context,config:{...context.config,language:'zh-Hant'},machine:{...context.machine,languages:['zh-Hans-TW']}};
 expect(verifyBrowser(manual,{...report,language:'zh-Hans-TW'},true,'',false).checks.find(row=>row.label==='网页语言')?.ok).toBe(false);
});

test('Chicago 已知地区的 12 项实测匹配通过，冲突和缺失地区仍失败',()=>{
 const ctx:BrowserContext={...context,region:{id:'us-central',label:'美国 · 芝加哥',country:'US',language:'en-US',locale:'en_US',timezone:'America/Chicago'},config:{...context.config,follow_machine:true},machine:{...context.machine,timezone:'America/Chicago'}};
 const observed:Report={...report,timezone:'America/Chicago',exit:{...report.exit!,city:'Chicago',timezone:'America/Chicago'}};
 for(const follow_machine of [true,false]){
  const configured={...ctx,config:{...ctx.config,follow_machine,timezone:'America/Chicago'}};
  const result=verifyBrowser(configured,observed,true,'',false);
  expect(result.phase).toBe('passed');
  expect(result.checks).toHaveLength(12);
  expect(result.checks.every(check=>check.ok===true)).toBe(true);
  expect(verifyBrowser(configured,{...observed,exit:{...observed.exit!,country:'CA'}},true,'',false).failures.find(row=>row.label==='出口国家')?.ok).toBe(false);
  expect(verifyBrowser(configured,{...observed,exit:{...observed.exit!,timezone:'America/New_York'}},true,'',false).failures.find(row=>row.label==='出口时区')?.ok).toBe(false);
  expect(verifyBrowser(configured,{...observed,language:'en-CA'},true,'',false).failures.find(row=>row.label==='网页语言')?.ok).toBe(false);
  expect(verifyBrowser({...configured,machine:{...ctx.machine,locale_country:'CA'}},observed,true,'',false).failures.find(row=>row.label==='本机区域')?.ok).toBe(false);
 }
 const missing=verifyBrowser({...ctx,region:null},observed,true,'',false);
 expect(missing.phase).toBe('failed');
 expect(missing.checks.filter(check=>check.ok===true)).toHaveLength(10);
});
