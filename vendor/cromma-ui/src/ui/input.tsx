/**
 * [INPUT]: 依赖 react 的 forwardRef，.cn，control 的高度表与圆角。
 * [OUTPUT]: 提供 Input。Input是variant field | bare；size small | medium | large，缺省与 Button 同高。
 * [POS]: cromma-ui的输入原子：field 聚焦 primary 边 + 柔光，bare 给自带表面的容器。要与按钮齐平就传同一个 size。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { forwardRef, type InputHTMLAttributes } from 'react';
import { cn } from '../cn';
import { CONTROL_DEFAULT_SIZE, CONTROL_HEIGHT, CONTROL_RADIUS, type ControlSize } from './control';

type Props = { variant?: 'field' | 'bare'; size?: ControlSize } & Omit<InputHTMLAttributes<HTMLInputElement>, 'size'>;

// 字号：手机 xl（16px，iOS 聚焦不放大）/ 桌面 base（与「登录」按钮同档）。消费方不许再写 text-[Npx]
const TYPE = 'text-xl text-foreground placeholder:text-muted-foreground sm:text-base';
const FIELD = `flex w-full ${CONTROL_RADIUS} border border-border bg-input px-3 ${TYPE} transition-[border-color,box-shadow] focus:border-primary focus:shadow-glow-input`;
const BARE = `w-full bg-transparent ${TYPE}`;

export const Input = forwardRef<HTMLInputElement, Props>(function Input({ variant = 'field', size = CONTROL_DEFAULT_SIZE, className, ...props }, ref) {
  return (
    <input
      ref={ref}
      spellCheck={false}
      autoComplete="off"
      className={cn('outline-none focus:outline-none focus-visible:outline-none disabled:cursor-not-allowed disabled:opacity-60', variant === 'field' ? cn(FIELD, CONTROL_HEIGHT[size]) : BARE, className)}
      {...props}
    />
  );
});
