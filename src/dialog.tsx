/**
 * [INPUT]: cromma-ui 的 ModalShell 与 Radix Dialog 的受控状态、焦点和读屏语义与共享 YAML 无障碍文案。
 * [OUTPUT]: AppDialog，连接页面状态与 Cromma 原版模态外壳，记录并恢复打开前焦点，关闭名称与底部操作按当前语言呈现。
 * [POS]: 清理预览和应用设置共用的模态边界；样式由复制的组件库提供。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {useId,useRef,type ReactNode} from 'react';
import {Dialog as DialogPrimitive} from 'radix-ui';
import {ModalShell} from 'cromma-ui';
import {useI18n} from './i18n';
export function AppDialog({open,onOpenChange,title,description,children,footer,width=560,className}:{open:boolean;onOpenChange:(open:boolean)=>void;title:string;description?:string;children:ReactNode;footer?:ReactNode;width?:number;className?:string}){
  const {t}=useI18n();
  const titleId=useId();
  const content=useRef<HTMLDivElement>(null);
  const previousFocus=useRef<HTMLElement|null>(null);
  return <DialogPrimitive.Root open={open} onOpenChange={onOpenChange}>
    <DialogPrimitive.Portal>
      <DialogPrimitive.Overlay className="app-dialog-overlay"/>
      <DialogPrimitive.Content ref={content} className="app-dialog-position" aria-labelledby={titleId} aria-describedby={undefined}
        onOpenAutoFocus={event=>{
          previousFocus.current=document.activeElement instanceof HTMLElement?document.activeElement:null;
          event.preventDefault();content.current?.focus();
        }}
        onCloseAutoFocus={event=>{
          event.preventDefault();previousFocus.current?.focus();
        }}>
        <ModalShell title={<DialogPrimitive.Title id={titleId} asChild><span>{title}</span></DialogPrimitive.Title>} description={description} onClose={()=>onOpenChange(false)} width={width} className={className} closeLabel={t("关闭弹窗")} footer={footer?<div className="confirm-actions">{footer}</div>:undefined}>
          {children}
        </ModalShell>
      </DialogPrimitive.Content>
    </DialogPrimitive.Portal>
  </DialogPrimitive.Root>;
}
