/**
 * [INPUT]: 原生身份命令、保留策略、Motion 弹簧/浏览器媒体订阅、共享确认窗/进度与 CLI 维护。
 * [OUTPUT]: 空/当前/暂存身份的竖式立体工牌，顶部绿光标记当前状态；弹簧横滚、悬停视差（操作区冻结动态）、账号显示与永久删除 Alert 警告（默认取消、需勾选确认）。
 * [POS]: 身份管理 GUI；只呈现脱敏元数据，身份正文留在原生保险库。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {useEffect, useRef, useState, useSyncExternalStore, type ReactNode} from 'react';
import {Button, Checkbox, Input} from 'cromma-ui';
import {ShieldCheck, Refresh} from 'cromma-ui/pika';
import {motion, useMotionValue, useSpring} from 'motion/react';
import {invoke} from '@tauri-apps/api/core';
import {useI18n} from './i18n';
import {AppDialog} from './dialog';
import {CleanupActivity, useCleanupActivity} from './cleaner-activity';
import {formatBytes, type Scan} from './cleaner-model';
import {ClaudeCliPanel} from './claude-cli-panel';
import {defaultPolicy, policyRows, type Identity, type IdentityState} from './identity-model';
type Target = Identity | 'empty' | 'recovery';
export function IdentityPanel({onBusy, initialEmpty=false}: {onBusy?: (busy:boolean)=>void; initialEmpty?:boolean}) {
 const {t, locale}=useI18n();
 const [state,setState]=useState<IdentityState|null>(null);
 const [policy,setPolicy]=useState({...defaultPolicy});
 const [loading,setLoading]=useState(true), [busy,setBusy]=useState(false), [cliBusy,setCliBusy]=useState(false);
 const [error,setError]=useState('');
 const [review,setReview]=useState<Target|null>(initialEmpty?'empty':null);
 const [name,setName]=useState(t('当前身份'));
 const [rename,setRename]=useState<Identity|null>(null), [renameValue,setRenameValue]=useState('');
 const [deletion,setDeletion]=useState<Identity|'current'|null>(null), [accepted,setAccepted]=useState(false);
 const activity=useCleanupActivity();
 const rail=useRef<HTMLDivElement>(null);
 const reduceMotion=useIdentityReducedMotion();
 const scrollSpring=useSpring(0,{stiffness:170,damping:25,mass:.8});
 const scrollTarget=useRef(0);
 const locked=busy||cliBusy||!!review||!!rename||!!deletion;
 const active=state?.records.find(row=>row.id===state.active);
 const current=state?.current;
 const currentName=current?.display_name||current?.email?.split('@')[0]||active?.name||t('当前身份');
 const blocked=locked||loading||!state||!!state?.recovery;
 const kept=policyRows.filter(([key])=>policy[key]).map(([,label])=>t(label));
 const removed=policyRows.filter(([key])=>!policy[key]).map(([,label])=>t(label));
 useEffect(()=>{void read();},[]);
 // ----- 用原生 scrollLeft 保留滚动条/键盘；Motion 仅提供滚轮弹簧 -----
 useEffect(()=>{
  const element=rail.current;if(!element)return;
  const maximum=()=>Math.max(0,element.scrollWidth-element.clientWidth);
  if(reduceMotion){scrollTarget.current=element.scrollLeft;scrollSpring.jump(element.scrollLeft);}
  const unsubscribe=scrollSpring.on('change',value=>{element.scrollLeft=Math.max(0,Math.min(maximum(),value));});
  function interrupt(){scrollTarget.current=element!.scrollLeft;scrollSpring.jump(element!.scrollLeft);}
  function scroll(){if(Math.abs(element!.scrollLeft-Math.max(0,Math.min(maximum(),scrollSpring.get())))>2)interrupt();}
  function wheel(event:WheelEvent){
   if(event.ctrlKey||event.metaKey)return;
   const amount=Math.abs(event.deltaX)>Math.abs(event.deltaY)?event.deltaX:event.deltaY;
   const delta=amount*(event.deltaMode===1?24:event.deltaMode===2?element!.clientWidth:1);
   const next=Math.max(0,Math.min(maximum(),scrollTarget.current+delta));
   if(next===scrollTarget.current)return;
   event.preventDefault();scrollTarget.current=next;
   if(reduceMotion)scrollSpring.jump(next);else scrollSpring.set(next);
  }
  element.addEventListener('wheel',wheel,{passive:false});
  element.addEventListener('scroll',scroll,{passive:true});
  element.addEventListener('pointerdown',interrupt);
  return()=>{unsubscribe();element.removeEventListener('wheel',wheel);element.removeEventListener('scroll',scroll);element.removeEventListener('pointerdown',interrupt);};
 },[scrollSpring,reduceMotion]);
 useEffect(()=>{onBusy?.(locked);return()=>onBusy?.(false);},[locked,onBusy]);
 async function read() {
  setLoading(true);
  try {setState(await invoke<IdentityState>('identities_read'));setError('');}
  catch(e) {setError(String(e));} finally {setLoading(false);}
 }
 async function refreshAfterError() {
  try {setState(await invoke<IdentityState>('identities_read'));} catch {/* 保留操作错误，不掩盖恢复失败。 */}
 }
 function choose(target:Target) {
  setName(target==='recovery'?t('恢复前状态'):currentName.slice(0,64));
  if(target==='recovery') {const previous=state?.records.find(row=>row.id===state.recovery);if(previous)setPolicy({...previous.policy});}
  setReview(target);setError('');
 }
 async function stop() {
  const result=await invoke<Scan>('cleaner_stop');
  if(result.running)throw new Error('仍有 Claude 进程在运行。');
 }
 async function change() {
  if(!review||busy)return;
  const target=review;setReview(null);setBusy(true);setError('');
  const onProgress=activity.start('identity');
  try {
   await stop();
   setState(await invoke<IdentityState>('identities_switch', {targetId:typeof target==='object'?target.id:'empty', currentName:name.trim(), policy, recover:target==='recovery', onProgress}));
   activity.finish(true,target==='recovery'?'原身份已恢复。':'身份切换完成，原身份已压缩暂存。');
  } catch(e) {setError(String(e));activity.finish(false,String(e));await refreshAfterError();}
  finally {setBusy(false);}
 }
 function confirmDelete(target:Identity|'current') {setDeletion(target);setAccepted(false);setError('');}
 async function remove() {
  if(!deletion||!accepted||busy)return;
  const target=deletion;setDeletion(null);setBusy(true);setError('');
  const onProgress=activity.start('identity');
  try {
   if(target==='current')await stop();
   setState(await invoke<IdentityState>('identities_delete',{id:target==='current'?'current':target.id,policy,onProgress}));
   activity.finish(true,'所选本地身份已直接删除。');
  } catch(e) {setError(String(e));activity.finish(false,String(e));await refreshAfterError();}
  finally {setBusy(false);}
 }
 async function saveName() {
  if(!rename||busy)return;
  setBusy(true);
  try {setState(await invoke<IdentityState>('identities_rename',{id:rename.id,value:renameValue}));setRename(null);setError('');}
  catch(e) {setError(String(e));} finally {setBusy(false);}
 }
 const options=<div className="identity-policy-grid">{policyRows.map(([key,label])=><div className="identity-policy-row" key={key}><Checkbox label={t(label)} checked={policy[key]} disabled={locked} onCheckedChange={checked=>setPolicy(old=>({...old,[key]:checked}))}/></div>)}</div>;
 return <section className="identity-page">
  <header className="cleanup-title"><h1>{t('Claude 身份管理')}</h1><Button variant="secondary" size="small" disabled={locked||loading} leftIcon={<Refresh aria-hidden size={16} className={loading?'spin':''}/>} onClick={()=>void read()}>{t('刷新')}</Button></header>
  {error&&<p role="alert" className="cleanup-feedback error">{t(error)}</p>}
  {state?.recovery&&<div className="identity-recovery" role="alert"><strong>{t('有一次身份切换尚未完成')}</strong><Button size="small" variant="ink" disabled={locked||loading} onClick={()=>choose('recovery')}>{t('恢复原身份')}</Button></div>}
    <section className="identity-library"><div className="identity-section-head"><h2>{t('身份')}</h2><span className="identity-count">{state?.records.length||0} {t('暂存身份')}</span></div>
   <div className="identity-card-rail" ref={rail} tabIndex={0} role="region" aria-label={t('身份')}>
    <IdentityCard className={`identity-card empty${current?.present===false?' selected':''}`} label={current?.present===false?`${t('空身份')} · ${t('当前身份')}`:t('空身份')}>
     <div className="identity-card-head"><div className="identity-avatar" aria-hidden="true">∅</div></div>
     <h3>{t('空身份')}</h3><p className="identity-empty-note">{t('相当于没有痕迹。新账户登录时务必选择。')}</p>
     <div className="identity-card-footer"><Button variant="secondary" size="small" disabled={blocked} onClick={()=>choose('empty')}>{t('使用空身份')}</Button></div>
    </IdentityCard>
    {(loading||current?.present)&&<IdentityCard className="identity-card current" label={t('当前身份')}>
     <div className="identity-card-head"><div className="identity-avatar" aria-hidden="true">{current?.display_name||current?.email?Array.from(currentName)[0]?.toUpperCase():<ShieldCheck size={20}/>}</div><span className="identity-live-badge">{loading?t('检查中'):state?.running?t('运行中'):t('进程已停止')}</span></div>
     <h3>{currentName}</h3>
     <div className="identity-account">{current?.email?<span className="identity-email">{current.email}</span>:<span className="identity-account-unknown">{t(current?.unavailable?'账号信息暂不可读':'账号信息未识别')}</span>}{current?.organization&&<span className="identity-organization">{current.organization}</span>}</div>
     {(current?.email||current?.display_name)&&<span className="identity-account-source">Claude Code</span>}
     <div className="identity-card-footer"><Button variant="secondary" size="small" disabled={blocked} onClick={()=>choose('empty')}>{t('暂存')}</Button><Button variant="ghost" size="small" disabled={blocked} onClick={()=>confirmDelete('current')}>{t('直接删除')}</Button></div>
    </IdentityCard>}
    {state?.records.map(row=><IdentityCard className={`identity-card${state.active===row.id?' selected':''}`} key={row.id}>
     <div className="identity-card-head"><div className="identity-avatar" aria-hidden="true">{Array.from(row.name)[0]?.toUpperCase()}</div>{row.id===state.active&&<span className="identity-card-badge">{t('上次使用')}</span>}<span className="identity-size">{formatBytes(row.bytes)}</span></div>
     <h3>{row.name}</h3><time>{new Date(row.saved_at).toLocaleString(locale==='zh'?'zh-CN':'en-US',{month:'short',day:'numeric',hour:'2-digit',minute:'2-digit'})}</time>
     <div className="identity-card-footer"><Button variant="secondary" size="small" disabled={blocked} onClick={()=>choose(row)}>{t('切换')}</Button><Button variant="ghost" size="small" disabled={blocked} onClick={()=>{setRename(row);setRenameValue(row.name);}}>{t('重命名')}</Button><Button variant="ghost" size="small" disabled={blocked} onClick={()=>confirmDelete(row)}>{t('删除')}</Button></div>
    </IdentityCard>)}
   </div>
  </section>
  <details className="identity-policy"><summary><strong>{t('保留在工作区')}</strong><span>{kept.length} / {policyRows.length}</span></summary>{options}</details>
  <CleanupActivity activity={activity.state}/>
  <ClaudeCliPanel blocked={locked||loading||!!state?.recovery} onBusy={setCliBusy}/>
  <AppDialog open={!!review} onOpenChange={open=>{if(!open&&!busy)setReview(null);}} title={review==='recovery'?t('恢复原身份'):review==='empty'?t('切换到空身份'):t('切换身份')} description={t('先压缩暂存当前 Claude Code 与 Desktop 状态，再切换。将结束已识别的 Claude 进程，未保存任务会中断。')} footer={<><Button variant="secondary" onClick={()=>setReview(null)}>{t('取消')}</Button><Button variant="ink" disabled={busy||loading||!state||!name.trim()} onClick={()=>void change()}>{review==='recovery'?t('确认恢复'):t('确认暂存并切换')}</Button></>}>
   <div className="identity-confirm"><label htmlFor="identity-current-name">{t('当前身份保存为')}</label><Input id="identity-current-name" maxLength={64} value={name} onChange={e=>setName(e.target.value)}/>{typeof review==='object'&&review&&<div className="identity-confirm-target"><span>{t('目标身份')}</span><strong>{review.name}</strong></div>}<div className="identity-confirm-keep"><span>{t('留在工作区')}</span><strong>{kept.join(' · ')||t('无')}</strong></div><p>{t('只管理本地状态；失效的登录可能需要重新认证。')}</p></div>
  </AppDialog>
  <AppDialog alert className="identity-delete-alert" open={!!deletion} onOpenChange={open=>{if(!open&&!busy)setDeletion(null);}} title={deletion==='current'?t('直接删除当前身份'):t('删除暂存身份')} description={deletion==='current'?t('将结束 Claude 进程并删除所选本地数据。不会新增备份，已有暂存身份不受影响。'):t('永久删除这份暂存文件。当前工作区不受影响。')} footer={<><Button variant="secondary" data-dialog-cancel onClick={()=>setDeletion(null)}>{t('取消')}</Button><Button className="identity-delete-button" variant="ink" disabled={busy||!accepted} onClick={()=>void remove()}>{t('确认直接删除')}</Button></>}>
   <div className="identity-confirm"><div className="identity-delete-warning"><span aria-hidden="true">!</span><strong>{t('警告：此操作不可撤销，不会创建新备份。')}</strong></div>{deletion==='current'?<><div className="identity-confirm-keep"><span>{t('删除范围')}</span><strong>{[t('登录与身份数据'),...removed].join(' · ')}</strong></div><div className="identity-confirm-keep"><span>{t('留在工作区')}</span><strong>{kept.join(' · ')||t('无')}</strong></div></>:<strong>{deletion?.name}</strong>}<Checkbox checked={accepted} onCheckedChange={setAccepted} label={t('我知道此操作无法撤销')}/></div>
  </AppDialog>
  <AppDialog open={!!rename} onOpenChange={open=>{if(!open&&!busy)setRename(null);}} title={t('重命名身份')} footer={<><Button variant="secondary" onClick={()=>setRename(null)}>{t('取消')}</Button><Button variant="ink" disabled={busy||!renameValue.trim()} onClick={()=>void saveName()}>{t('保存')}</Button></>}><Input aria-label={t('身份名称')} maxLength={64} value={renameValue} onChange={e=>setRenameValue(e.target.value)}/>{error&&<p role="alert">{t(error)}</p>}</AppDialog>
 </section>;
}

// ----- 工牌跟随指针微倾斜，离开后由弹簧回正；触摸与减弱动态保持静止 -----
function IdentityCard({className,children,label}:{className:string;children:ReactNode;label?:string}){
 const reduced=useIdentityReducedMotion();
 const tiltX=useMotionValue(0),tiltY=useMotionValue(0),lift=useMotionValue(0);
 const rotateX=useSpring(tiltX,{stiffness:240,damping:22}),rotateY=useSpring(tiltY,{stiffness:240,damping:22}),y=useSpring(lift,{stiffness:240,damping:22});
 useEffect(()=>{if(reduced){tiltX.jump(0);tiltY.jump(0);lift.jump(0);rotateX.jump(0);rotateY.jump(0);y.jump(0);}},[reduced,tiltX,tiltY,lift,rotateX,rotateY,y]);
 const freeze=()=>{rotateX.jump(rotateX.get());rotateY.jump(rotateY.get());y.jump(y.get());};
 return <motion.article className={className} aria-label={label} style={{rotateX,rotateY,y,transformPerspective:850}}
  onPointerDown={freeze}
  onPointerMove={event=>{if(reduced||event.pointerType!=='mouse')return;if((event.target as Element).closest('button')){freeze();return;}const box=event.currentTarget.getBoundingClientRect();tiltX.set(-(event.clientY-box.top-box.height/2)/box.height*9);tiltY.set((event.clientX-box.left-box.width/2)/box.width*9);lift.set(-4);}}
  onPointerLeave={()=>{tiltX.set(0);tiltY.set(0);lift.set(0);}}>
  {children}
 </motion.article>;
}

// ----- 原生媒体订阅：当前 Motion hook 只记录初始偏好，故动态切换由浏览器事件驱动 -----
const motionPreference=typeof window==='undefined'?null:window.matchMedia('(prefers-reduced-motion: reduce)');
const subscribeMotion=(notify:()=>void)=>{motionPreference?.addEventListener('change',notify);return()=>motionPreference?.removeEventListener('change',notify);};
function useIdentityReducedMotion(){return useSyncExternalStore(subscribeMotion,()=>motionPreference?.matches??true,()=>true);}
