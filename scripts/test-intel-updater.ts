/**
 * [INPUT]: Real AppUpdate component with mocked React hooks, native architecture and updater IPC.
 * [OUTPUT]: Startup and user checks never contact the official updater on Intel; existing routes preserved.
 * [POS]: Isolated process regression, no actual updater requests, installation, native host or user data.
 * [PROTOCOL]: Update this contract and scripts/CLAUDE.md when changing this file.
 */
import {mock} from 'bun:test';
import {strict as assert} from 'node:assert';
import {resolve} from 'node:path';
let platform='',architecture='',requests=0,installs=0;
let effects:Array<()=>unknown>=[],states:unknown[]=[];
const element=(type:unknown,props:unknown)=>({type,props});
mock.module('react',()=> ({useState:(initial:unknown)=>[initial,(value:unknown)=>states.push(value)],useRef:(current:unknown)=>({current}),useEffect:(effect:()=>unknown)=>effects.push(effect)}));
mock.module('react/jsx-runtime',()=> ({jsx:element,jsxs:element,Fragment:'fragment'}));
mock.module('react/jsx-dev-runtime',()=> ({jsxDEV:element,Fragment:'fragment'}));
mock.module('@tauri-apps/api/core',()=> ({invoke:async(command:string)=>{
 if(command==='runtime_platform')return platform;
 if(command==='runtime_arch')return architecture;
 throw Error('Unexpected native call: '+command);
}}));
mock.module('@tauri-apps/api/app',()=> ({getVersion:async()=>'0.2.0'}));
mock.module('@tauri-apps/plugin-updater',()=> ({check:async()=>{requests++;return null;}}));
mock.module('@tauri-apps/plugin-process',()=> ({relaunch:async()=>{installs++;}}));
mock.module('cromma-ui',()=> ({Button:'button',NavButton:'navbutton'}));
mock.module('cromma-ui/pika',()=> ({Refresh:'refresh'}));
mock.module(resolve('src/dialog.tsx'),()=> ({AppDialog:'dialog'}));
mock.module(resolve('src/i18n.ts'),()=> ({useI18n:()=>({t:(text:string)=>text})}));
const {AppUpdate}=await import('../src/app-update.tsx');
function find(node:any,type:string):any {
 if(!node)return;
 if(Array.isArray(node)){for(const child of node){const found=find(child,type);if(found)return found;}return;}
 if(node.type===type)return node;
 return find(node.props?.children,type);
}
const settle=async()=>{for(let i=0;i<5;i++)await Promise.resolve();};
for(const fixture of [{platform:'macos',arch:'x86_64',official:false},{platform:'macos',arch:'aarch64',official:true},{platform:'windows',arch:'x86_64',official:true},{platform:'linux',arch:'x86_64',official:false}]){
 platform=fixture.platform;architecture=fixture.arch;requests=0;installs=0;effects=[];states=[];
 const tree=AppUpdate({blocked:false,onBusy:()=>{throw Error('No installation expected');}});
 for(const effect of effects)effect();
 await settle();
 assert.equal(requests,fixture.official?1:0,'startup update route '+platform+'/'+architecture);
 const button=find(tree,'navbutton');assert(button,'Missing real update button');
 button.props.onClick();await settle();
 assert.equal(requests,fixture.official?2:0,'user update route '+platform+'/'+architecture);
 assert.equal(installs,0);
 if(platform==='macos'&&architecture==='x86_64')assert(states.includes('此 Intel 构建请使用完整 A+B 安装包更新，官方更新源未提供 Intel 版本。'));
 console.log('PASS startup and user check:',platform,architecture,fixture.official?'official updater':'manual package');
}
console.log('4 updater route scenarios passed; no real network or installation.');
