/**
 * [INPUT]: 原生身份命令、保留策略、Motion 弹簧/浏览器媒体订阅、共享确认窗/进度、文件残留与 CLI 维护。
 * [OUTPUT]: 空/当前/暂存身份竖式立体工牌、原地备份与订阅/凭据徽章、顶部绿光、弹簧横滚/视差、活跃会话提示与删除确认；卡片下统一显示身份/空身份/文件清理的真实阶段动画和结果。
 * [POS]: 身份管理 GUI；只呈现脱敏元数据，身份正文留在原生保险库；可选浏览器绑定/旧资料采用确认与重开重试，关联浏览器资料删除单独选择。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {useEffect, useRef, useState, useSyncExternalStore, type ReactNode} from 'react';
import {Button, Checkbox, Input} from 'cromma-ui';
import {ShieldCheck, Refresh} from 'cromma-ui/pika';
import {motion, useMotionValue, useSpring} from 'motion/react';
import {invoke} from '@tauri-apps/api/core';
import {useI18n} from './i18n';
import {AppDialog} from './dialog';
import {IdentityOperationProgress, useCleanupActivity} from './cleaner-activity';
import {formatBytes} from './cleaner-model';
import {ClaudeCliPanel} from './claude-cli-panel';
import {ClaudeResiduePanel} from './claude-residue-panel';
import {defaultPolicy, policyRows, type Identity, type IdentityState} from './identity-model';
type Target = Identity | 'empty' | 'recovery' | 'save';
function authLabel(status:string|null|undefined) {
 if(status==='valid')return '凭据有效';
 if(status==='refresh_needed')return '待自动续期';
 if(status==='expired')return '需重登';
 if(status==='api_key')return 'API 密钥';
 return null;
}
export function IdentityPanel({onBusy, initialEmpty=false}: {onBusy?: (busy:boolean)=>void; initialEmpty?:boolean}) {
 const {t, locale}=useI18n();
 const [state,setState]=useState<IdentityState|null>(null);
 const [policy,setPolicy]=useState({...defaultPolicy});
 const [loading,setLoading]=useState(true), [busy,setBusy]=useState(false), [cliBusy,setCliBusy]=useState(false);
 const [residueBusy,setResidueBusy]=useState(false),[residueRevision,setResidueRevision]=useState(0);
 const [error,setError]=useState('');
 const [browserReview,setBrowserReview]=useState<boolean|null>(null),[useExisting,setUseExisting]=useState(false);
 const [review,setReview]=useState<Target|null>(initialEmpty?'empty':null);
 const [name,setName]=useState(t('当前身份'));
 const [rename,setRename]=useState<Identity|null>(null), [renameValue,setRenameValue]=useState('');
 const [deleteBrowser,setDeleteBrowser]=useState(false);
 const [deletion,setDeletion]=useState<Identity|'current'|null>(null), [accepted,setAccepted]=useState(false);
 const activity=useCleanupActivity();
 const rail=useRef<HTMLDivElement>(null);
 const reduceMotion=useIdentityReducedMotion();
 const scrollSpring=useSpring(0,{stiffness:170,damping:25,mass:.8});
 const scrollTarget=useRef(0);
 const locked=busy||cliBusy||residueBusy||!!review||!!rename||!!deletion||browserReview!==null;
 const active=state?.records.find(row=>row.id===state.active);
 const current=state?.current;
 const saved=state?.records.filter(row=>!current?.present||row.id!==state.active)??[];
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
 async function read(invalidateResidue=true) {
  setLoading(true);
  try {
   const next=await invoke<IdentityState>('identities_read');
   setState(next);if(invalidateResidue)setResidueRevision(value=>value+1);setError('');
   if(review==='empty'){
    const resolved=next.current.display_name||next.current.email?.split('@')[0]||next.records.find(row=>row.id===next.active)?.name||t('当前身份');
    setName(old=>old===t('当前身份')?resolved.slice(0,64):old);
   }
   return true;
  }
  catch(e) {setError(String(e));return false;} finally {setLoading(false);}
 }
 async function refreshAfterError() {
  setResidueRevision(value=>value+1);
  try {setState(await invoke<IdentityState>('identities_read'));} catch {/* 保留操作错误，不掩盖恢复失败。 */}
 }
 async function configureBrowser() {
  if(browserReview===null||busy)return;
  const enabled=browserReview;setBusy(true);setError('');
  try {setState(await invoke<IdentityState>('identities_browser',{enabled,useExisting}));setBrowserReview(null);}
  catch(e){setError(String(e));await refreshAfterError();}finally{setBusy(false);}
 }
 async function retryBrowser() {
  if(busy)return;setBusy(true);setError('');
  try {await invoke('browser_launch',{onProgress:activity.start('browser')});activity.finish(true,'浏览器资料已与身份同步。');await read();}
  catch(e){setError(String(e));activity.finish(false,String(e));}finally{setBusy(false);}
 }
 function choose(target:Target) {
  setName(target==='recovery'?t('恢复前状态'):target==='save'?(active?.name||currentName).slice(0,64):currentName.slice(0,64));
  if(target==='recovery') {const previous=state?.records.find(row=>row.id===state.recovery);if(previous)setPolicy({...previous.policy});}
  else if(target==='save'&&active)setPolicy({...active.policy});
  setReview(target);setError('');
 }
 async function stop() {
  await invoke('identities_stop',{onProgress:activity.channel()});
 }
 async function change() {
  if(!review||busy)return;
  const target=review;setReview(null);setBusy(true);setError('');
  const onProgress=activity.start('identity',target==='recovery'?'恢复原身份':target==='save'?'保存当前身份备份':target==='empty'?'切换到空身份':'切换身份');
  try {
   await stop();
   const next=target==='save'
    ?await invoke<IdentityState>('identities_save',{currentName:name.trim(),policy,onProgress})
    :await invoke<IdentityState>('identities_switch', {targetId:typeof target==='object'?target.id:'empty', currentName:name.trim(), policy, recover:target==='recovery', onProgress});
   setState(next);
   setResidueRevision(value=>value+1);
   activity.finish(true,next.browser_warning||(target==='recovery'?'原身份已恢复。':target==='save'?'当前身份已加密备份，工作区保持不变。':'身份切换完成。'));
  } catch(e) {setError(String(e));activity.finish(false,String(e));await refreshAfterError();}
  finally {setBusy(false);}
 }
 function confirmDelete(target:Identity|'current') {setDeletion(target);setAccepted(false);setDeleteBrowser(false);setError('');}
 async function remove() {
  if(!deletion||!accepted||busy)return;
  const target=deletion;setDeletion(null);setBusy(true);setError('');
  const onProgress=activity.start('identity',target==='current'?'直接删除当前身份':'删除暂存身份');
  try {
   if(target==='current')await stop();
   const next=await invoke<IdentityState>('identities_delete',{id:target==='current'?'current':target.id,policy,deleteBrowser:target!=='current'&&deleteBrowser,onProgress});
   setState(next);
   setResidueRevision(value=>value+1);
   activity.finish(true,next.browser_warning||'所选本地身份已直接删除。');
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
    <section className="identity-library"><div className="identity-section-head"><h2>{t('身份')}</h2><div className="identity-library-controls"><span className="identity-count">{saved.length} {t('暂存身份')}</span><Checkbox label={t('联动浏览器')} checked={state?.browser.enabled??false} disabled={blocked} onCheckedChange={value=>{setUseExisting(false);setBrowserReview(value===true);setError('');}}/></div></div>
   <div className="identity-card-rail" ref={rail} tabIndex={0} role="region" aria-label={t('身份')}>
    <IdentityCard className={`identity-card empty${current?.present===false?' selected':''}`} label={current?.present===false?`${t('空身份')} · ${t('当前身份')}`:t('空身份')}>
     <div className="identity-card-head"><div className="identity-avatar" aria-hidden="true">∅</div></div>
     <h3>{t('空身份')}</h3><p className="identity-empty-note">{t('清空本地登录身份；保留选定工作区资料，不影响服务端记录。')}</p>
     <div className="identity-card-footer"><Button variant="secondary" size="small" disabled={blocked} onClick={()=>choose('empty')}>{t('使用空身份')}</Button></div>
    </IdentityCard>
    {(loading||current?.present)&&<IdentityCard className="identity-card current" label={t('当前身份')}>
     <div className="identity-card-head"><div className="identity-avatar" aria-hidden="true">{current?.display_name||current?.email?Array.from(currentName)[0]?.toUpperCase():<ShieldCheck size={20}/>}</div><span className="identity-live-badge">{loading?t('检查中'):state?.running?t('运行中'):t('进程已停止')}</span></div>
     <h3>{currentName}</h3>
     <div className="identity-account">{current?.email?<span className="identity-email">{current.email}</span>:<span className="identity-account-unknown">{t(current?.unavailable?'账号信息暂不可读':'账号信息未识别')}</span>}{current?.organization&&<span className="identity-organization">{current.organization}</span>}</div>
     <div className="identity-badges">
      <span className={`identity-status-badge ${active?'backed-up':'unbacked'}`}>{t(active?'已备份':'未备份')}</span>
      {current?.plan&&<span className="identity-status-badge plan">{current.plan}</span>}
      {authLabel(current?.auth_status)&&<span className={`identity-status-badge auth-${current?.auth_status}`}>{t(authLabel(current?.auth_status)!)}</span>}
      {state?.browser.enabled&&<span className="identity-browser-badge">{t('独立浏览器')}</span>}
     </div>
     <div className="identity-card-footer"><Button variant="secondary" size="small" disabled={blocked} onClick={()=>choose('save')}>{active?t('更新备份'):t('保存备份')}</Button><Button variant="ghost" size="small" disabled={blocked} onClick={()=>confirmDelete('current')}>{t('直接删除')}</Button></div>
    </IdentityCard>}
    {saved.map(row=>{const summary=state?.summaries?.[row.id];return <IdentityCard className="identity-card" key={row.id}>
     <div className="identity-card-head"><div className="identity-avatar" aria-hidden="true">{Array.from(row.name)[0]?.toUpperCase()}</div><span className="identity-size">{formatBytes(row.bytes)}</span></div>
     <h3>{row.name}</h3>
     {summary&&(summary.email||summary.organization)&&<div className="identity-account saved-account">{summary.email&&<span className="identity-email">{summary.email}</span>}{summary.organization&&<span className="identity-organization">{summary.organization}</span>}</div>}
     <div className="identity-badges">
      {summary?.plan&&<span className="identity-status-badge plan">{summary.plan}</span>}
      {authLabel(summary?.auth_status)&&<span className={`identity-status-badge auth-${summary?.auth_status}`}>{t(authLabel(summary?.auth_status)!)}</span>}
      {state?.browser.bound_ids.includes(row.id)&&<span className="identity-browser-badge">{t('独立浏览器')}</span>}
     </div>
     <time>{new Date(row.saved_at).toLocaleString(locale==='zh'?'zh-CN':'en-US',{month:'short',day:'numeric',hour:'2-digit',minute:'2-digit'})}</time>
     <div className="identity-card-footer"><Button variant="secondary" size="small" disabled={blocked} onClick={()=>choose(row)}>{t('切换')}</Button><Button variant="ghost" size="small" disabled={blocked} onClick={()=>{setRename(row);setRenameValue(row.name);setError('');}}>{t('重命名')}</Button><Button variant="ghost" size="small" disabled={blocked} onClick={()=>confirmDelete(row)}>{t('删除')}</Button></div>
    </IdentityCard>;})}
   </div>
   <IdentityOperationProgress activity={activity.state}/>
  </section>
  <details className="identity-policy"><summary><strong>{t('保留在工作区')}</strong><span>{kept.length} / {policyRows.length}</span></summary>{options}</details>
  {state?.browser_warning&&<div className="identity-recovery" role="alert"><span>{t(state.browser_warning)}</span><Button size="small" variant="secondary" disabled={blocked} onClick={()=>void retryBrowser()}>{t('打开浏览器')}</Button></div>}
  <ClaudeResiduePanel activity={activity} blocked={locked||loading||!state||!!state?.recovery} revision={residueRevision} onBusy={setResidueBusy} onChanged={async()=>{if(!await read(false))throw new Error('身份状态刷新失败，请重新检查。');}}/>
  <ClaudeCliPanel blocked={locked||loading||!!state?.recovery} onBusy={setCliBusy}/>
  <AppDialog open={!!review} onOpenChange={open=>{if(!open&&!busy)setReview(null);}} title={review==='recovery'?t('恢复原身份'):review==='save'?t(active?'更新当前身份备份':'保存当前身份备份'):review==='empty'?t('切换到空身份'):t('切换身份')} description={t(review==='save'?'将结束已识别的 Claude 进程，把当前身份原地加密写入保险库；当前工作区与登录保持不变。':'先压缩暂存当前 Claude Code 与 Desktop 状态，再切换。将结束已识别的 Claude 进程，未保存任务会中断。')} footer={<><Button variant="secondary" onClick={()=>setReview(null)}>{t('取消')}</Button><Button variant="ink" disabled={busy||loading||!state||!name.trim()||(review!=='recovery'&&!!state.recovery)} onClick={()=>void change()}>{review==='recovery'?t('确认恢复'):review==='save'?t('确认保存备份'):t('确认暂存并切换')}</Button></>}>
   <div className="identity-confirm"><label htmlFor="identity-current-name">{t('当前身份保存为')}</label><Input id="identity-current-name" maxLength={64} value={name} onChange={e=>setName(e.target.value)}/>{typeof review==='object'&&review&&<div className="identity-confirm-target"><span>{t('目标身份')}</span><strong>{review.name}</strong></div>}<div className="identity-confirm-keep"><span>{t(review==='save'?'纳入加密备份':'留在工作区')}</span><strong>{kept.join(' · ')||t('无')}</strong></div>{(state?.sessions?.length??0)>0&&<div className="identity-active-sessions"><span>{t('运行中的 Claude 会话（将自动结束）')}</span><ul>{state!.sessions.map(session=><li key={session.pid}><code>PID {session.pid}</code><span>{session.entrypoint}</span>{session.workspace&&<strong>{session.workspace}</strong>}</li>)}</ul></div>}{state?.browser.enabled&&review!=='save'&&<p>{t('将关闭并重开关联浏览器，请先保存网页任务。')}</p>}<p>{t('只管理本地状态；失效的登录可能需要重新认证。')}</p></div>
  </AppDialog>
  <AppDialog alert className="identity-delete-alert" open={!!deletion} onOpenChange={open=>{if(!open&&!busy)setDeletion(null);}} title={deletion==='current'?t('直接删除当前身份'):t('删除暂存身份')} description={deletion==='current'?t('将结束 Claude 进程并删除所选本地数据。不会新增备份，已有暂存身份不受影响。'):t('永久删除这份暂存文件。当前工作区不受影响。')} footer={<><Button variant="secondary" data-dialog-cancel onClick={()=>setDeletion(null)}>{t('取消')}</Button><Button className="identity-delete-button" variant="ink" disabled={busy||!accepted} onClick={()=>void remove()}>{t('确认直接删除')}</Button></>}>
   <div className="identity-confirm"><div className="identity-delete-warning"><span aria-hidden="true">!</span><strong>{t('警告：此操作不可撤销，不会创建新备份。')}</strong></div>{deletion==='current'?<><div className="identity-confirm-keep"><span>{t('删除范围')}</span><strong>{[t('登录与身份数据'),...removed].join(' · ')}</strong></div><div className="identity-confirm-keep"><span>{t('留在工作区')}</span><strong>{kept.join(' · ')||t('无')}</strong></div></>:<strong>{deletion?.name}</strong>}{typeof deletion==='object'&&deletion&&state?.browser.bound_ids.includes(deletion.id)&&<Checkbox label={t('同时永久删除关联浏览器资料')} checked={deleteBrowser} onCheckedChange={value=>setDeleteBrowser(value===true)}/>}<Checkbox checked={accepted} onCheckedChange={setAccepted} label={t('我知道此操作无法撤销')}/></div>
  </AppDialog>
  <AppDialog open={browserReview!==null} onOpenChange={open=>{if(!open&&!busy)setBrowserReview(null);}} title={t(browserReview?'联动浏览器':'关闭浏览器联动')} description={t('将正常关闭并重开浏览器，请先保存网页任务。')} footer={<><Button variant="secondary" disabled={busy} onClick={()=>setBrowserReview(null)}>{t('取消')}</Button><Button variant="ink" disabled={busy} loading={busy} onClick={()=>void configureBrowser()}>{t('确认')}</Button></>}>
   <div className="identity-confirm"><p>{t(browserReview?'每个身份使用独立网页资料和稳定指纹，切换时自动恢复。首次仍需登录网站。':'恢复使用原有通用浏览器资料。各身份的独立资料会保留。')}</p>{browserReview&&<Checkbox label={t('将现有浏览器资料关联给当前身份')} checked={useExisting} disabled={busy} onCheckedChange={value=>setUseExisting(value===true)}/>}<p>{t('默认创建新资料，现有资料不会删除。资料隔离不会改变出口 IP。')}</p>{error&&<p role="alert">{t(error)}</p>}</div>
  </AppDialog>
  <AppDialog open={!!rename} onOpenChange={open=>{if(!open&&!busy){setRename(null);setError('');}}} title={t('重命名身份')} footer={<><Button variant="secondary" onClick={()=>{setRename(null);setError('');}}>{t('取消')}</Button><Button variant="ink" disabled={busy||!renameValue.trim()} onClick={()=>void saveName()}>{t('保存')}</Button></>}><Input aria-label={t('身份名称')} maxLength={64} value={renameValue} onChange={e=>setRenameValue(e.target.value)}/>{error&&<p role="alert">{t(error)}</p>}</AppDialog>
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
