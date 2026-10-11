/**
 * [INPUT]: 依赖 react 的 forwardRef，.cn。
 * [OUTPUT]: 提供 MenuRow。MenuRow是justify、align、tone、active、as button | div。
 * [POS]: cromma-ui的菜单行：Surface（menu）上行项的唯一长相，嵌进表面的洗色，字取 sm（Laper 菜单项同档）。行距由 menuSurfaceClass的 gap 给，行自己不写 margin。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { forwardRef, type ButtonHTMLAttributes, type HTMLAttributes } from 'react';
import { cn } from '../cn';

type Props = {
  as?: 'button' | 'div';
  justify?: 'start' | 'between';
  align?: 'center' | 'start';
  tone?: 'default' | 'primary';
  active?: boolean; // 当前选中的项（模型选择器、筛选器）：常驻 accent 实底
} & ButtonHTMLAttributes<HTMLButtonElement> &
  HTMLAttributes<HTMLDivElement>;

// 悬停是淡洗、选中是实底：两者深浅不同，相邻时一眼分得清哪个是当前项。
// 键盘高亮与悬停同一副洗色：Radix 菜单标 data-highlighted，cmdk 标 data-selected
const TONE = {
  default: 'hover:bg-accent/60 data-[highlighted]:bg-accent/60 data-[selected=true]:bg-accent/60',
  primary: 'hover:bg-primary/10 data-[highlighted]:bg-primary/10 data-[selected=true]:bg-primary/10',
};

export const MenuRow = forwardRef<HTMLButtonElement & HTMLDivElement, Props>(function MenuRow({ as = 'button', justify = 'start', align = 'center', tone = 'default', active = false, className, children, ...rest }, ref) {
  const classes = cn(
    'flex w-full cursor-pointer gap-2 rounded-xl px-2.5 py-2 text-left text-sm text-foreground outline-none transition-colors disabled:cursor-wait data-[disabled]:cursor-not-allowed data-[disabled]:opacity-50',
    align === 'start' ? 'items-start' : 'items-center',
    justify === 'between' && 'justify-between',
    TONE[tone],
    active && 'bg-accent font-medium hover:bg-accent data-[highlighted]:bg-accent data-[selected=true]:bg-accent',
    className,
  );
  if (as === 'div') {
    return (
      <div ref={ref} className={classes} {...(rest as HTMLAttributes<HTMLDivElement>)}>
        {children}
      </div>
    );
  }
  return (
    <button ref={ref} type="button" className={classes} {...(rest as ButtonHTMLAttributes<HTMLButtonElement>)}>
      {children}
    </button>
  );
});
