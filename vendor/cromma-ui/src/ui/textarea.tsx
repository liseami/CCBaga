/**
 * [INPUT]: 依赖 react 的 forwardRef，.cn，control 的 CONTROL_RADIUS。
 * [OUTPUT]: 提供 Textarea。Textarea是variant field | bare。
 * [POS]: cromma-ui的多行输入：与 Input 同一套圆角、边框、柔光与字号，没有高度档。bare 给 Composer 这类自带表面的容器。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { forwardRef, type TextareaHTMLAttributes } from 'react';
import { cn } from '../cn';
import { CONTROL_RADIUS } from './control';

type Props = { variant?: 'field' | 'bare' } & TextareaHTMLAttributes<HTMLTextAreaElement>;

// 字号：手机 xl（16px，iOS 聚焦不放大）/ 桌面 base。提问面、对话 Composer 都走这一档
const TYPE = 'text-xl text-foreground placeholder:text-muted-foreground sm:text-base';
const VARIANT = {
  field: `${CONTROL_RADIUS} border border-border bg-input px-3 py-2 ${TYPE} transition-[border-color,box-shadow] focus:border-primary focus:shadow-glow-input`,
  bare: `bg-transparent ${TYPE}`,
};

export const Textarea = forwardRef<HTMLTextAreaElement, Props>(function Textarea({ variant = 'field', className, ...props }, ref) {
  return <textarea ref={ref} spellCheck={false} className={cn('w-full resize-none outline-none select-text focus:outline-none focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-60', VARIANT[variant], className)} {...props} />;
});
