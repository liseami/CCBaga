/**
 * [INPUT]: Node.js 标准管道、已锁定 playwright-core、固定引擎、Vite 与 tests/ 的虚构 Tauri IPC。
 * [OUTPUT]: 工牌顺序/数量、横滚弹簧/视差、双语、保留选项与确认/删除/失败/恢复的真实 DOM 交互验收。
 * [POS]: 发布前无头行为回归；无截图、无真实账号/文件修改，不加载原生宿主，不允许外网请求。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {strict as assert} from 'node:assert';
import {resolve,join} from 'node:path';
import {mkdtemp,mkdir,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {createServer} from 'vite';
import {chromium} from 'playwright-core';
const engine=process.platform==='darwin'?'Browser.app/Contents/MacOS/Chromium':process.platform==='win32'?'WindowsBrowser/chrome.exe':'LinuxBrowser/chrome';
const server=await createServer({server:{host:'127.0.0.1',port:0,strictPort:false,watch:{ignored:['**/src-tauri/**','**/browser-runtime/**','**/releases/**','**/dist/**']}},optimizeDeps:{include:['@tauri-apps/api/mocks']}});
await server.listen();
const address=server.httpServer!.address() as {port:number};
const base=`http://127.0.0.1:${address.port}`;
const env=Object.fromEntries(Object.entries(process.env).filter(([key])=>!key.startsWith('APPLE_')&&!key.startsWith('TAURI_SIGNING_PRIVATE_KEY'))) as Record<string,string>;
// ----- 临时 HOME/XDG 避免容器环境的只读配置和真实浏览器资料 -----
const temporary=await mkdtemp(join(tmpdir(),'ccbaga-ui-'));
for(const name of ['config','cache'])await mkdir(join(temporary,name));
Object.assign(env,{HOME:temporary,XDG_CONFIG_HOME:join(temporary,'config'),XDG_CACHE_HOME:join(temporary,'cache')});
let browser;
try{browser=await chromium.launch({executablePath:resolve('browser-runtime',engine),headless:true,env});}
catch(error){await server.close();await rm(temporary,{recursive:true,force:true});throw error;}
let checks=0;
try{
 const context=await browser.newContext({viewport:{width:1040,height:820},reducedMotion:'no-preference'});
 await context.route('**/*',route=>route.request().url().startsWith(base+'/')?route.continue():route.abort());
 const page=await context.newPage();page.setDefaultTimeout(10000);
 const errors:string[]=[];page.on('console',message=>{if(message.type()==='error')console.error(message.text());});page.on('pageerror',error=>{errors.push(error.message);console.error(error.message);});
 await page.goto(base+'/tests/identities.html');
 const cards=page.locator('.identity-card');
 await page.getByText('fixture@example.invalid',{exact:true}).waitFor();
 assert.equal(await cards.count(),42);assert.equal(await cards.first().getByRole('heading').innerText(),'空身份');
 assert.equal(await page.locator('.identity-switcher').count(),0);
 assert.match(await cards.first().innerText(),/新账户登录时务必选择/);checks++;
 const geometry=await cards.evaluateAll(nodes=>nodes.slice(0,3).map(node=>{const r=node.getBoundingClientRect();return {x:r.x,y:r.y,width:r.width,height:r.height};}));
 assert.equal(geometry[0].y,geometry[2].y);assert(geometry[2].x+geometry[2].width<1040-150);assert(Math.abs(geometry[0].width/geometry[0].height-54/85.6)<.01);checks++;
 const current=page.locator('.identity-card.current');
 const light=await current.evaluate(node=>getComputedStyle(node,'::before').boxShadow);assert.match(light,/67, 228, 132/);
 await page.mouse.move(geometry[1].x+150,geometry[1].y+85);
 await page.waitForFunction(()=>document.querySelector('.identity-card.current')?.getAttribute('style')?.includes('rotate'));
 await page.mouse.move(10,10);checks++;
 const rail=page.locator('.identity-card-rail');
 await rail.hover();await page.mouse.wheel(0,800);
 await page.waitForFunction(()=>document.querySelector('.identity-card-rail')!.scrollLeft>50);
 const mid=await rail.evaluate(node=>node.scrollLeft);assert(mid<801);
 await page.waitForFunction(()=>Math.abs(document.querySelector('.identity-card-rail')!.scrollLeft-800)<3);checks++;
 await rail.evaluate(node=>{node.scrollLeft=0;});
 await current.getByRole('button',{name:'直接删除',exact:true}).click();
  let dialog=page.getByRole('alertdialog');await dialog.waitFor();
 assert.match(await dialog.innerText(),/不可撤销/);assert.equal(await dialog.getByRole('button',{name:'确认直接删除'}).isEnabled(),false);
 assert.equal(await dialog.getByRole('button',{name:'取消'}).evaluate(node=>node===document.activeElement),true);
 await page.mouse.click(3,3);assert.equal(await dialog.count(),1);
 await dialog.getByRole('button',{name:'取消'}).click();
 assert.equal(await page.evaluate(()=>((window as any).identityFixture.commands as any[]).filter(row=>row.command==='identities_delete').length),0);checks++;
 await page.getByText('保留在工作区',{exact:true}).click();
 await page.getByRole('checkbox',{name:'Skills 与命令'}).uncheck();
 await cards.first().getByRole('button',{name:'使用空身份'}).click();
 dialog=page.getByRole('dialog');await dialog.waitFor();await dialog.getByLabel('当前身份保存为').fill('First archive');
 await dialog.getByRole('button',{name:'确认暂存并切换'}).click();
 await page.waitForFunction(()=>!(window as any).identityFixture.state.current.present);
 assert.equal(await page.locator('.identity-card.current').count(),0);
 const request=await page.evaluate(()=>((window as any).identityFixture.commands as any[]).find(row=>row.command==='identities_switch').args);
 assert.equal(request.policy.skills,false);assert.equal(request.targetId,'empty');assert.equal(request.currentName,'First archive');checks++;
 const target=cards.filter({has:page.getByRole('heading',{name:'Identity 3',exact:true})});
 await target.getByRole('button',{name:'切换',exact:true}).click();
 await page.getByRole('dialog').getByRole('button',{name:'确认暂存并切换'}).click();await page.getByText('Fixture team',{exact:true}).waitFor();checks++;
 await target.getByRole('button',{name:'重命名',exact:true}).click();
 await page.getByRole('dialog').getByRole('textbox',{name:'身份名称'}).fill('Renamed fixture');
 await page.getByRole('dialog').getByRole('button',{name:'保存',exact:true}).click();
 const renamed=cards.filter({has:page.getByRole('heading',{name:'Renamed fixture',exact:true})});await renamed.waitFor();
 await renamed.getByRole('button',{name:'删除',exact:true}).click();dialog=page.getByRole('alertdialog');
 await dialog.getByRole('checkbox',{name:'我知道此操作无法撤销'}).check();
 await dialog.getByRole('button',{name:'确认直接删除'}).click();await renamed.waitFor({state:'detached'});
 assert.equal(await page.evaluate(()=>(window as any).identityFixture.state.current.present),true);checks++;
 await page.evaluate(()=>{(window as any).identityFixture.language('en');});
 await cards.first().getByRole('button',{name:'Use empty identity'}).waitFor();assert.match(await cards.first().innerText(),/new account/);checks++;
 await page.evaluate(()=>{(window as any).identityFixture.setFailure('identities_switch');});
 await cards.first().getByRole('button',{name:'Use empty identity'}).click();
 await page.getByRole('dialog').getByRole('button',{name:'Archive & switch'}).click();
 await page.getByText(/fixture operation failed/).first().waitFor();assert.equal(await page.evaluate(()=>(window as any).identityFixture.state.current.present),true);checks++;
 await page.evaluate(()=>{(window as any).identityFixture.recover();});
 await page.getByRole('button',{name:'Refresh',exact:true}).click();
 await page.getByRole('button',{name:'Restore previous identity',exact:true}).click();
 await page.getByRole('dialog').getByRole('button',{name:'Restore'}).click();
 await page.waitForFunction(()=>(window as any).identityFixture.state.recovery===null);checks++;
 await page.emulateMedia({reducedMotion:'reduce'});
 await page.mouse.move(10,10);await page.waitForTimeout(250);
 const before=await current.getAttribute('style');await current.hover();await page.waitForTimeout(150);
 assert.equal(await current.getAttribute('style'),before);checks++;
 await current.getByRole('button',{name:'Delete permanently',exact:true}).click();dialog=page.getByRole('alertdialog');
 await dialog.getByRole('checkbox',{name:'I understand this cannot be undone'}).check();
 const count=await page.evaluate(()=>(window as any).identityFixture.state.records.length);
 await dialog.getByRole('button',{name:'Confirm deletion',exact:true}).click();
 await current.waitFor({state:'detached'});assert.equal(await page.evaluate(()=>(window as any).identityFixture.state.records.length),count);checks++;
 assert.equal(errors.length,0,errors.join('\n'));
 console.log(`${checks} identity UI simulation checks passed; no native commands or user data used.`);
}finally{await browser.close();await server.close();await rm(temporary,{recursive:true,force:true});}
