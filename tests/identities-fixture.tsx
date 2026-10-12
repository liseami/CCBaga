/**
 * [INPUT]: Tauri 官方 mockIPC、真实身份组件与虚构的内存状态。
 * [OUTPUT]: 40 个虚构身份、命令审计与失败/恢复场景，供无头浏览器验证。
 * [POS]: 独立测试入口；没有原生命令、真实目录、账号或网络，生产 rollup 不包含此入口。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {createRoot} from 'react-dom/client';
import {mockIPC} from '@tauri-apps/api/mocks';
import {MotionRoot,TooltipProvider} from 'cromma-ui';
import {IdentityPanel} from '../src/identities';
import {defaultPolicy,type IdentityState} from '../src/identity-model';
import {setLocale} from '../src/i18n';
import '../src/index.css';
import '../src/style.css';
const id=(number:number)=>number.toString(16).padStart(32,'0');
const user={present:true,display_name:'Fixture user',email:'fixture@example.invalid',organization:'Fixture team',unavailable:false};
const state:IdentityState={active:'current',recovery:null,running:true,current:{...user},records:Array.from({length:40},(_,n)=>({id:id(n+1),name:`Identity ${n+1}`,saved_at:1780000000000+n,bytes:2048,policy:{...defaultPolicy}}))};
const commands:{command:string;args:any}[]=[];
let fail='';
const fixture={commands,state,language:setLocale,setFailure:(command:string)=>{fail=command;},recover:()=>{state.recovery=id(1);}};
Object.assign(window,{identityFixture:fixture});
mockIPC((command,args:any)=>{
 if(!['identities_read','claude_cli_read'].includes(command))commands.push({command,args});
 if(fail===command){fail='';throw Error('fixture operation failed');}
 if(command==='identities_read')return structuredClone(state);
 if(command==='claude_cli_read')return {method:'',path:'',version:''};
 if(command==='cleaner_stop'){state.running=false;return {running:false};}
 if(command==='identities_switch'){
  if(args.recover){state.recovery=null;state.current={...user};return structuredClone(state);}
  if(state.current.present)state.records.push({id:id(1000+commands.length),name:args.currentName,saved_at:1780000010000+commands.length,bytes:4096,policy:{...args.policy}});
  state.active=args.targetId;
  state.current=args.targetId==='empty'?{present:false,display_name:null,email:null,organization:null,unavailable:false}:{...user,display_name:state.records.find(row=>row.id===args.targetId)!.name};
  return structuredClone(state);
 }
 if(command==='identities_rename'){state.records.find(row=>row.id===args.id)!.name=args.value;return structuredClone(state);}
 if(command==='identities_delete'){
  if(args.id==='current'){state.current={present:false,display_name:null,email:null,organization:null,unavailable:false};state.active='empty';}
  else state.records=state.records.filter(row=>row.id!==args.id);
  return structuredClone(state);
 }
 throw Error(`Unexpected fixture command: ${command}`);
});
createRoot(document.getElementById('fixture')!).render(<MotionRoot><TooltipProvider><div style={{width:740,height:752,margin:'20px auto',border:'1px solid #ddd'}}><IdentityPanel/></div></TooltipProvider></MotionRoot>);
