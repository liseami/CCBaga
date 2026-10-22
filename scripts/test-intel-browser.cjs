// [INPUT]: Supplied engine/launcher and disposable profiles on macOS.
// [OUTPUT]: CPU/language/timezone/canvas behavior and the observed Intel configuration.
// [POS]: Loopback-only test page; no real user data or external requests.
const {chromium}=require('playwright-core');
const {mkdtempSync,rmSync}=require('node:fs');
const {tmpdir}=require('node:os');
const {join}=require('node:path');
const assert=require('node:assert/strict');
const {createServer}=require('node:http');
const executablePath=process.argv[2];
let pageUrl;
if(!executablePath)throw new Error('Usage: node scripts/test-intel-browser.cjs /path/to/Chromium');
async function sample(seed,language,timezone,cores=7,native=false){
  const profile=mkdtempSync(join(tmpdir(),'ccbaga-intel-smoke-'));
  let context;
  try{
    context=await chromium.launchPersistentContext(profile,{
      executablePath,headless:true,
      args:[`--fingerprint=${seed}`,'--fingerprint-platform=macos',`--fingerprint-hardware-concurrency=${cores}`,'--fingerprint-platform-version=15.7.7','--fingerprint-brand=Chrome','--fingerprint-brand-version=148.0.7778.215',`--lang=${language}`,`--accept-lang=${language}`,`--timezone=${timezone}`,`--disable-spoofing=${native?'font,audio,gpu,canvas,clientrects':'font,audio,gpu'}`,'--no-first-run','--disable-background-networking'],
    });
    const page=await context.newPage();
    await page.goto(pageUrl);
    return await page.evaluate(async()=>{
      const c=document.createElement('canvas');c.width=240;c.height=80;
      const ctx=c.getContext('2d');ctx.fillStyle='#f60';ctx.fillRect(0,0,240,80);
      ctx.fillStyle='#123';ctx.font='18px Arial';ctx.fillText('CCBaga Intel fingerprint test',5,45);
      const metadata=await navigator.userAgentData.getHighEntropyValues(['fullVersionList','platformVersion']);
      return {metadata,language:navigator.language,timezone:Intl.DateTimeFormat().resolvedOptions().timeZone,cores:navigator.hardwareConcurrency,canvas:c.toDataURL(),platform:navigator.platform};
    });
  }finally{if(context)await context.close();rmSync(profile,{recursive:true,force:true});}
}
(async()=>{
  const server=createServer((req,res)=>res.end('<!doctype html><title>isolated fixture</title>'));
  await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
  pageUrl=`http://127.0.0.1:${server.address().port}/`;
  try {
  const a=await sample(123456,'en-US','America/New_York');
  const b=await sample(654321,'ja-JP','Asia/Tokyo');
  const c=await sample(654321,'en-US','America/New_York');
  assert.equal(a.cores,7,'Engine must honor fingerprint CPU flag');
  assert.equal(b.cores,7);
  assert.equal(a.language,'en-US');assert.equal(b.language,'ja-JP');
  assert.equal(a.timezone,'America/New_York');assert.equal(b.timezone,'Asia/Tokyo');
  assert.notEqual(a.canvas,c.canvas,'Changing only the seed must change canvas fingerprint');
  const intel=await sample(495574189,'zh-Hans-CN','Asia/Shanghai',8,true);
  assert.equal(intel.cores,8);assert.equal(intel.language,'zh-Hans-CN');
  assert.equal(intel.timezone,'Asia/Shanghai');assert.equal(intel.metadata.platformVersion,'15.7.7');
  assert(intel.metadata.fullVersionList.some(x=>x.version==='148.0.7778.215'),'Real webpage metadata must match packaged engine');
  console.log(JSON.stringify({passed:true,checks:['CPU override','language override','timezone override','canvas seed rotation','Intel CPU 8/platform 15.7.7/full version .215'],observed:[{...a,canvas:'[omitted]'},{...b,canvas:'[omitted]'},{...intel,canvas:'[omitted]'}]},null,2));
  }finally{await new Promise(resolve=>server.close(resolve));}
})().catch(e=>{console.error(e);process.exit(1);});
