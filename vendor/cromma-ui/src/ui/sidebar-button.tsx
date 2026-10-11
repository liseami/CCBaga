/**
 * [INPUT]: 依赖 react 的 forwardRef、cn。
 * [OUTPUT]: 提供 SidebarButton、sidebarButtonClass 与类型 SidebarButtonLook。SidebarButton 含 active、disabled、look（card、wash）、as（button、div）。
 * [POS]: cromma-ui 的可选行。对照 Laper SidebarButton。卡里的行项只用它。选中皮分两副：card 是浮起白卡（缺省，卡里的行项），wash 是 sidebar-accent 灰底、无边无影（RailButton 的侧栏键帽）。悬停两副同为 sidebar-accent 66%。class 版只给必须自带元素的场合，例如 dnd 容器。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { forwardRef, type ButtonHTMLAttributes, type HTMLAttributes } from 'react';
import { cn } from '../cn';

const BASE = 'flex items-center rounded-2xl border text-left transition-colors outline-none focus-visible:ring-2 focus-visible:ring-primary/40';

/** 选中皮：card 浮起白卡；wash 灰底，比悬停的 66% 实一档 */
export type SidebarButtonLook = 'card' | 'wash';
const ACTIVE: Record<SidebarButtonLook, string> = { card: 'border-border bg-card shadow-sm', wash: 'border-transparent bg-sidebar-accent' };

export function sidebarButtonClass(opts: { active?: boolean; disabled?: boolean; look?: SidebarButtonLook; className?: string } = {}): string {
  return cn(
    BASE,
    opts.active ? ACTIVE[opts.look ?? 'card'] : 'border-transparent hover:bg-sidebar-accent/66',
    opts.disabled ? 'cursor-not-allowed opacity-50 hover:bg-transparent' : 'cursor-pointer',
    opts.className,
  );
}

type Props = {
  active?: boolean;
  disabled?: boolean;
  look?: SidebarButtonLook;
  /** 行里还嵌着别的按钮（关闭、删除）时用 div：button 里不能再嵌 button */
  as?: 'button' | 'div';
} & Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'disabled'> &
  HTMLAttributes<HTMLDivElement>;

export const SidebarButton = forwardRef<HTMLButtonElement & HTMLDivElement, Props>(function SidebarButton({ active = false, disabled = false, look, as = 'button', className, children, ...rest }, ref) {
  const classes = sidebarButtonClass({ active, disabled, look, className });
  if (as === 'div') {
    return (
      <div ref={ref} role="button" tabIndex={disabled ? -1 : 0} aria-disabled={disabled || undefined} className={classes} {...(rest as HTMLAttributes<HTMLDivElement>)}>
        {children}
      </div>
    );
  }
  return (
    <button ref={ref} type="button" disabled={disabled} className={classes} {...(rest as ButtonHTMLAttributes<HTMLButtonElement>)}>
      {children}
    </button>
  );
});
