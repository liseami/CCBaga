/**
 * [INPUT]: cromma-ui 的 ModalShell 与 Radix Dialog/AlertDialog 的受控状态、焦点和读屏语义与共享 YAML 无障碍文案。
 * [OUTPUT]: AppDialog，连接页面状态与 Cromma 原版模态外壳，普通/警告模态共享外壳；警告默认聚焦取消并阻止遮罩误关闭，记录并恢复原焦点。
 * [POS]: 清理预览和应用设置共用的模态边界；样式由复制的组件库提供。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {useId,useRef,type ReactNode} from 'react';
import {Dialog as DialogPrimitive, AlertDialog as AlertPrimitive} from 'radix-ui';
import {ModalShell} from 'cromma-ui';
import {useI18n} from './i18n';
export function AppDialog({open,onOpenChange,title,description,children,footer,width=560,className,alert=false}:{open:boolean;onOpenChange:(open:boolean)=>void;title:string;description?:string;children:ReactNode;footer?:ReactNode;width?:number;className?:string;alert?:boolean}){
  const {t}=useI18n();
  const titleId=useId();
  const descriptionId=useId();
  const Primitive=alert?AlertPrimitive:DialogPrimitive;
  const content=useRef<HTMLDivElement>(null);
  const previousFocus=useRef<HTMLElement|null>(null);
  return <Primitive.Root open={open} onOpenChange={onOpenChange}>
    <Primitive.Portal>
      <Primitive.Overlay className="app-dialog-overlay"/>
      <Primitive.Content ref={content} className="app-dialog-position" aria-labelledby={titleId} aria-describedby={description?descriptionId:undefined}
        onOpenAutoFocus={event=>{
          previousFocus.current=document.activeElement instanceof HTMLElement?document.activeElement:null;
          event.preventDefault();
          const safe=alert?content.current?.querySelector<HTMLElement>('[data-dialog-cancel]'):null;
          (safe||content.current)?.focus();
        }}
        onCloseAutoFocus={event=>{
          event.preventDefault();previousFocus.current?.focus();
        }}>
        <ModalShell title={<Primitive.Title id={titleId} asChild><span>{title}</span></Primitive.Title>} description={description?<Primitive.Description id={descriptionId} asChild><span>{description}</span></Primitive.Description>:undefined} onClose={()=>onOpenChange(false)} width={width} className={className} closeLabel={t("关闭弹窗")} footer={footer?<div className="confirm-actions">{footer}</div>:undefined}>
          {children}
        </ModalShell>
      </Primitive.Content>
    </Primitive.Portal>
  </Primitive.Root>;
}
