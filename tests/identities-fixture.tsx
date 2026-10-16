/**
 * [INPUT]: Tauri 官方 mockIPC、真实身份/浏览器组件与虚构的内存状态。
 * [OUTPUT]: 40 个虚构身份、残留清单、可显式释放的 IPC 与官方 Channel 事件，供无头浏览器验证真实进度/失败；browser 模式检查仅缓存与全清理确认，不触碰磁盘资料。
 * [POS]: 独立测试入口；没有原生命令、真实目录、账号或网络，生产 rollup 不包含此入口。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {createRoot} from 'react-dom/client';
import {mockIPC} from '@tauri-apps/api/mocks';
import {MotionRoot,TooltipProvider} from 'cromma-ui';
import {IdentityPanel} from '../src/identities';
import {BrowserPanel} from '../src/browser';
import type {BrowserState} from '../src/browser-model';
import {defaultPolicy,type IdentityState} from '../src/identity-model';
import type {Scan,CleanResult} from '../src/cleaner-model';
import type {Progress} from '../src/cleaner-activity';
import {setLocale} from '../src/i18n';
import '../src/index.css';
import '../src/style.css';
const id=(number:number)=>number.toString(16).padStart(32,'0');
const user={present:true,display_name:'Fixture user',email:'fixture@example.invalid',organization:'Fixture team',plan:'Pro',auth_status:'valid',unavailable:false};
const summaries=Object.fromEntries(Array.from({length:40},(_,n)=>[id(n+1),{email:`user${n+1}@example.invalid`,organization:`Org ${n+1}`,plan:n%2===0?'Max 20x':'Pro',auth_status:'valid'}]));
const state:IdentityState={browser:{enabled:true,bound_ids:[]},browser_warning:null,active:id(1),recovery:null,running:true,sessions:[{pid:4201,workspace:'CCBaga',entrypoint:'VS Code'}],current:{...user},summaries,records:Array.from({length:40},(_,n)=>({id:id(n+1),name:`Identity ${n+1}`,saved_at:1780000000000+n,bytes:2048,policy:{...defaultPolicy}}))};
const browserMode=new URLSearchParams(location.search).get('panel')==='browser';
const browserState:BrowserState={ui_language:'zh',exit_services:[],region:null,config:{seed:123,proxy:'',follow_machine:true,language:'en-US',timezone:'America/New_York',native_graphics:true},machine:{platform:'macos',version:'fixture',cores:8,timezone:'America/New_York',languages:['en-US'],locale:'en_US',locale_country:'US',location_enabled:false,timezones:['America/New_York']},engine_version:'fixture',engine_ready:true,running:false,profile:'/fixture/profiles/current',identity_linked:true,cache:[],cache_error:'',report:null,report_received_at:null,machine_exit:null,checks:[]};
const commands:{command:string;args:any}[]=[];
let fail='';
let identityReads=0;
const residue={scans:0,cleanMode:'success',remaining:['fixture-cache','fixture-history','fixture-account','fixture-incomplete']};
const residueItems:Scan['items']=[
 {id:'fixture-cache',path:'/fixture/Claude/Cache',title:'虚构缓存',group:'cache',risk:'safe',impact:'可再生成',bytes:4096,files:2,exact:true,keys:[]},
 {id:'fixture-history',path:'/fixture/.claude/history.jsonl',title:'虚构会话历史',group:'history',risk:'history',impact:'包含历史',bytes:2048,files:1,exact:true,keys:[]},
 {id:'fixture-account',path:'/fixture/.claude.json',title:'虚构登录标识',group:'account',risk:'login',impact:'需重新登录',bytes:128,files:1,exact:true,keys:['oauthAccount','userID']},
 {id:'fixture-incomplete',path:'/fixture/Claude/UnreadableCache',title:'虚构不可完整统计缓存',group:'cache',risk:'safe',impact:'目录统计不完整，拒绝直接清理',bytes:0,files:0,exact:false,keys:[]},
];
type Pending={command:string;args:any;resolve:(value:unknown)=>void;reject:(error:Error)=>void;index:number};
const deferred=new Set<string>(),pending:Pending[]=[];
const fixture={commands,state,residue,browser:browserState,browserBusy:false,get identityReads(){return identityReads;},get pending(){return pending.map(row=>row.command);},language:setLocale,setFailure:(command:string)=>{fail=command;},recover:()=>{state.recovery=id(1);},
 defer:(...names:string[])=>{for(const name of names)deferred.add(name);},
 emitProgress:(command:string,event:Progress)=>{
  const entry=pending.find(row=>row.command===command);if(!entry)throw Error(`No pending command: ${command}`);
  const channel=entry.args.onProgress;
  const callback=typeof channel==='string'?Number(channel.replace('__CHANNEL__:','')):channel?.id;
  if(typeof callback!=='number')throw Error(`No progress channel: ${command}`);
  // ----- mockIPC 保留 Channel 对象；用官方回调分发器模拟 Rust 的有序消息包 -----
  (window as any).__TAURI_INTERNALS__.runCallback(callback,{index:entry.index++,message:event});
 },
 release:(command:string,error?:string)=>{
  const at=pending.findIndex(row=>row.command===command);if(at<0)throw Error(`No pending command: ${command}`);
  const [entry]=pending.splice(at,1);
  if(error){entry.reject(Error(error));return;}
  try{entry.resolve(execute(command,entry.args));}catch(error){entry.reject(error as Error);}
 },
};
Object.assign(window,{identityFixture:fixture});
mockIPC((command,args:any)=>{
 if(!['identities_read','claude_cli_read'].includes(command))commands.push({command,args});
 if(deferred.delete(command))return new Promise((resolve,reject)=>pending.push({command,args,resolve,reject,index:0}));
 return execute(command,args);
});
function execute(command:string,args:any){
 if(command==='plugin:event|listen')return 1;
 if(command==='plugin:event|unlisten')return;
 if(fail===command){fail='';throw Error('fixture operation failed');}
 if(browserMode&&command==='browser_read')return structuredClone(browserState);
 if(browserMode&&command==='browser_default_read')return {is_default:true,http:'fixture',https:'fixture',installed:true,app:'CCBaga-B'};
 if(browserMode&&command==='browser_clear'){browserState.cache=[];return structuredClone(browserState);}
 if(command==='identities_browser'){state.browser.enabled=args.enabled;if(args.enabled&&!state.browser.bound_ids.includes(state.active))state.browser.bound_ids.push(state.active);return structuredClone(state);}
 if(command==='browser_launch'){state.browser_warning=null;return {};}
 if(command==='identities_read'){identityReads++;return structuredClone(state);}
 if(command==='claude_residue_scan'){
  residue.scans++;
  return {folder:'/fixture',items:structuredClone(residueItems.filter(row=>residue.remaining.includes(row.id))),running:state.running,processes:state.running?[{pid:100,name:'Claude fixture'}]:[],message:'虚构扫描完成',warnings:['虚构警告：部分目录无法完整统计。'],diagnostics:[],org_id:''} satisfies Scan;
 }
 if(command==='claude_residue_clean'){
  const selected=args.ids as string[];
  if(residue.cleanMode==='partial'){
   residue.remaining=residue.remaining.filter(id=>id!==selected[0]);
   return {cleaned:selected.length?1:0,errors:['fixture cleanup partly failed'],completed:false} satisfies CleanResult;
  }
  residue.remaining=residue.remaining.filter(id=>!selected.includes(id));
  return {cleaned:selected.length,errors:[],completed:true} satisfies CleanResult;
 }
 if(command==='claude_cli_read')return {method:'',path:'',version:''};
 if(command==='identities_stop'){state.running=false;state.sessions=[];return {running:false};}
 if(command==='identities_save'){
  const existing=state.records.find(row=>row.id===state.active);
  if(existing){existing.name=args.currentName;existing.saved_at=1780000010000+commands.length;existing.policy={...args.policy};}
  else {const nextId=id(1000+commands.length);state.records.push({id:nextId,name:args.currentName,saved_at:1780000010000+commands.length,bytes:4096,policy:{...args.policy}});state.active=nextId;state.summaries[nextId]={email:state.current.email,organization:state.current.organization,plan:state.current.plan,auth_status:state.current.auth_status};}
  if(state.browser.enabled&&!state.browser.bound_ids.includes(state.active))state.browser.bound_ids.push(state.active);
  return structuredClone(state);
 }
 if(command==='identities_switch'){
  if(args.recover){state.recovery=null;state.current={...user};return structuredClone(state);}
  if(state.current.present){const existing=state.records.find(row=>row.id===state.active);if(existing){existing.saved_at=1780000010000+commands.length;existing.policy={...args.policy};}else{const nextId=id(1000+commands.length);state.records.push({id:nextId,name:args.currentName,saved_at:1780000010000+commands.length,bytes:4096,policy:{...args.policy}});state.summaries[nextId]={email:state.current.email,organization:state.current.organization,plan:state.current.plan,auth_status:state.current.auth_status};}}
  if(state.browser.enabled){if(state.active!=='empty'&&!state.browser.bound_ids.includes(state.active))state.browser.bound_ids.push(state.active);if(args.targetId!=='empty'&&!state.browser.bound_ids.includes(args.targetId))state.browser.bound_ids.push(args.targetId);}
  state.active=args.targetId;
  state.current=args.targetId==='empty'?{present:false,display_name:null,email:null,organization:null,plan:null,auth_status:null,unavailable:false}:{...user,display_name:state.records.find(row=>row.id===args.targetId)!.name};
  return structuredClone(state);
 }
 if(command==='identities_rename'){state.records.find(row=>row.id===args.id)!.name=args.value;return structuredClone(state);}
 if(command==='identities_delete'){
  if(args.id==='current'){state.current={present:false,display_name:null,email:null,organization:null,plan:null,auth_status:null,unavailable:false};state.active='empty';}
  else{state.records=state.records.filter(row=>row.id!==args.id);delete state.summaries[args.id];}
  return structuredClone(state);
 }
 throw Error(`Unexpected fixture command: ${command}`);
}
createRoot(document.getElementById('fixture')!).render(<MotionRoot><TooltipProvider><div style={{width:740,height:752,margin:'20px auto',border:'1px solid #ddd'}}>{browserMode?<BrowserPanel onBusy={value=>{fixture.browserBusy=value;}}/>:<IdentityPanel/>}</div></TooltipProvider></MotionRoot>);
