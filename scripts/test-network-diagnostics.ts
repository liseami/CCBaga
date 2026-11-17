/**
 * [INPUT]: 隔离 Playwright 引擎、Vite 与网络页虚构 IPC。
 * [OUTPUT]: Claude 连通性与系统 DNS 的延期检测、原始 IPv4/IPv6/出口、混合/未知/失败拒绝与双语 DOM 回归。
 * [POS]: 共用 UI 回归的功能证明，不截图或访问外部服务。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {strict as assert} from 'node:assert';
import type {Browser} from 'playwright-core';
export async function testNetworkDiagnostics(browser:Browser,base:string) {
 const context=await browser.newContext();
 await context.route('**/*',route=>route.request().url().startsWith(base+'/')?route.continue():route.abort());
 const page=await context.newPage();page.setDefaultTimeout(10000);
 try{
  await page.goto(base+'/tests/network.html');
  await page.getByRole('heading',{name:'Claude 出口',exact:true}).waitFor();
  const panel=page.locator('.domain-diagnostics'),rows=panel.locator('.domain-diagnostics-item');
  assert.equal(await panel.getAttribute('data-status'),'idle');assert.equal(await rows.count(),4);
  assert.deepEqual(await panel.locator('.domain-name').allInnerTexts(),['claude.ai','api.anthropic.com','platform.claude.com','downloads.claude.ai']);
  await panel.getByRole('button',{name:'开始检测',exact:true}).click();
  await page.waitForFunction(()=>document.querySelector('.domain-diagnostics')?.getAttribute('data-status')==='checking');
  assert.equal(await panel.locator('.domain-badge.checking').count(),4);
  assert.equal(await panel.getByRole('button',{name:'检测中',exact:true}).isDisabled(),true);
  await page.evaluate(()=>(window as any).networkFixture.finish());
  await page.waitForFunction(()=>document.querySelector('.domain-diagnostics')?.getAttribute('data-status')==='passed');
  assert.equal(await panel.locator('.domain-badge.passed').count(),4);
  assert.match(await panel.locator('.domain-outcome').innerText(),/检测成功/);
  assert.match(await rows.first().innerText(),/HTTP 403.*服务可达，需浏览器验证/);
  for(const mode of ['partial','error','empty','invalid']){
   await page.evaluate(mode=>(window as any).networkFixture.setMode(mode),mode);
   await panel.getByRole('button',{name:'重新检测',exact:true}).click();
   assert.equal(await panel.locator('.domain-badge.passed').count(),0);
   await page.evaluate(()=>(window as any).networkFixture.finish());
   await page.waitForFunction(()=>document.querySelector('.domain-diagnostics')?.getAttribute('data-status')==='failed');
   assert.match(await panel.locator('.domain-outcome').innerText(),/检测失败/);
   if(mode==='partial'){
    assert.equal(await panel.locator('.domain-badge.failed').count(),1);
    assert.match(await panel.locator('.domain-diagnostics-item.failed').innerText(),/HTTP 403.*受到安全拦截或限制/);
   }else assert.match(await panel.locator('.domain-outcome').innerText(),mode==='error'?/fixture connection interrupted/:mode==='invalid'?/检测结果无效/:/部分检测未返回结果/);
  }
  await page.evaluate(()=>(window as any).networkFixture.language('en'));
  await page.getByRole('heading',{name:'Claude Egress',exact:true}).waitFor();
  assert.match(await panel.locator('.domain-outcome').innerText(),/Check failed/);
  await panel.getByRole('button',{name:'Check again',exact:true}).waitFor();
  await page.evaluate(()=>(window as any).networkFixture.language('zh'));
  const dns=page.locator('.dns-diagnostics');
  assert.equal(await dns.getAttribute('data-status'),'idle');
  for(const mode of ['observed','mixed','inconclusive','partial','empty','error','invalid']){
   await page.evaluate(mode=>(window as any).networkFixture.setDnsMode(mode),mode);
   await dns.getByRole('button',{name:mode==='observed'?'开始检测':'重新检测',exact:true}).click();
   await page.waitForFunction(()=>document.querySelector('.dns-diagnostics')?.getAttribute('data-status')==='checking');
   assert.equal(await dns.getByRole('button',{name:'检测中',exact:true}).isDisabled(),true);
   assert.equal(await dns.getByText('1.1.1.1',{exact:true}).count(),0);
   await page.evaluate(()=>(window as any).networkFixture.finishDns());
   const expected=['observed','mixed'].includes(mode)?mode:'inconclusive';
   await page.waitForFunction(expected=>document.querySelector('.dns-diagnostics')?.getAttribute('data-status')===expected,expected);
   if(['observed','mixed','inconclusive'].includes(mode)){
    assert.match(await dns.innerText(),/203\.0\.113\.5/);
    assert.equal(await dns.getByText('2606:4700:4700::1111',{exact:true}).count(),1);
    assert.match(await dns.innerText(),/已观测 12 \/ 12/);
   }else assert.equal(await dns.getByText('1.1.1.1',{exact:true}).count(),0);
  }
  await page.evaluate(()=>(window as any).networkFixture.language('en'));
  await dns.getByRole('heading',{name:'System DNS check',exact:true}).waitFor();
  assert.match(await dns.locator('.domain-outcome').innerText(),/Inconclusive/);
  assert.match(await dns.innerText(),/Browser secure DNS and WebRTC are not tested/);
  return 15;
 } finally {await context.close();}
}
