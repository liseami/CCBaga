/**
 * [INPUT]: 依赖 radix-ui 的 Tooltip，.cn 的 cn。
 * [OUTPUT]: 提供 TooltipProvider/Tooltip/TooltipTrigger/TooltipContent 与 Tip。TooltipProvider/Tooltip/TooltipTrigger/TooltipContent 与 Tip是一行式包装，可给方位与离距。
 * [POS]: cromma-ui的提示原子——Laper的浅色表面 bg-popover + outline-border + shadow-md，600ms 延迟。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { ReactNode } from 'react';
import { Tooltip as TooltipPrimitive } from 'radix-ui';
import { cn } from '../cn';

export const TooltipProvider = TooltipPrimitive.Provider;
export const Tooltip = TooltipPrimitive.Root;
export const TooltipTrigger = TooltipPrimitive.Trigger;

export function TooltipContent({ className, sideOffset = 8, ...props }: TooltipPrimitive.TooltipContentProps) {
  return (
    <TooltipPrimitive.Portal>
      <TooltipPrimitive.Content
        sideOffset={sideOffset}
        className={cn(
          'laper-tooltip-motion z-3 flex max-w-72 flex-col rounded-md bg-popover px-2.5 py-1.5 text-sm text-popover-foreground shadow-md outline outline-border',
          className,
        )}
        {...props}
      />
    </TooltipPrimitive.Portal>
  );
}

/** 一行式：<Tip label="…"><button/></Tip>；offset 是离触发器的距离（缺省 8） */
export function Tip({ label, side = 'bottom', offset, children }: { label: ReactNode; side?: 'top' | 'bottom' | 'left' | 'right'; offset?: number; children: ReactNode }) {
  return (
    <Tooltip>
      <TooltipTrigger asChild>{children}</TooltipTrigger>
      <TooltipContent side={side} sideOffset={offset}>{label}</TooltipContent>
    </Tooltip>
  );
}
