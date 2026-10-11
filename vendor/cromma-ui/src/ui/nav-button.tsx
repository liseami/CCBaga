/**
 * [INPUT]: 依赖 .cn，pressable，sidebar-button 的 sidebarButtonClass（raised 选中态）。
 * [OUTPUT]: 提供 NavButton。NavButton是icon、active: boolean | 'raised'、indent、trailing。
 * [POS]: cromma-ui的侧栏导航行 Laper VerticalNavbar 移植：页面底色与侧栏卡上都用它，其余卡片里的可选行用 SidebarButton。raised 给正开着的应用。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { ButtonHTMLAttributes, HTMLAttributes, ReactNode } from 'react';
import { cn } from '../cn';
import { Pressable } from './pressable';
import { sidebarButtonClass } from './sidebar-button';

type Props = {
  icon: ReactNode;
  /** true = 导航选中（sidebar-accent 平铺）；'raised' = 浮起白卡（SidebarButton 的选中皮，给正开着的应用） */
  active?: boolean | 'raised';
  /** 缩进一级（属于上面那一行）：图标对齐父级文字的起点，没有竖线 */
  indent?: boolean;
  /** 行尾（计数之类） */
  trailing?: ReactNode;
  children: ReactNode;
} & Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'children'> &
  Omit<HTMLAttributes<HTMLDivElement>, 'children'>;

export function NavButton({ icon, active = false, indent = false, trailing, className, children, ...rest }: Props) {
  // 常驻透明 border：与 raised 的浮起白卡同宽，两种选中态来回切不跳位
  const skin = active === 'raised' ? sidebarButtonClass({ active: true }) : cn('border border-transparent', active ? 'bg-sidebar-accent' : 'hover:bg-sidebar-accent/66');
  return (
    <Pressable
      {...rest}
      className={cn(
        'group flex h-8 w-full items-center gap-1.5 rounded-xl pr-3 text-left transition-colors',
        skin,
        'rounded-xl', // raised 皮自带 rounded-2xl，这里统一回导航行的圆角
        indent ? 'pl-10' : 'pl-2', // 子级：图标落在父级文字的起点下（8 + 24 + 8 = 40，Linear 的树）
        className,
      )}
    >
      <span className="relative flex size-6 shrink-0 items-center justify-center">
        {/* 原图标：悬停时收束退场 */}
        <span className={cn('absolute inset-0 flex items-center justify-center transition-[opacity,transform] group-hover:-translate-x-[5px] group-hover:-rotate-[8deg] group-hover:scale-[0.56] group-hover:opacity-0', active ? 'text-foreground' : 'text-muted-foreground')}>{icon}</span>
        {/* Pika ChevronBigRight 的原始曲线：悬停时沿路径描绘出来 */}
        <svg aria-hidden="true" width="20" height="20" viewBox="0 0 24 24" fill="none" className={cn('absolute inset-0 m-auto translate-x-[5px] scale-[0.78] opacity-0 transition-[opacity,transform] group-hover:translate-x-0 group-hover:scale-100 group-hover:opacity-100', active ? 'text-foreground' : 'text-muted-foreground')}>
          <path d="M9 18a30.6 30.6 0 0 0 5.817-5.49.8.8 0 0 0 0-1.02A30.6 30.6 0 0 0 9 6" pathLength={1} stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" className="[stroke-dasharray:1] [stroke-dashoffset:1] transition-[stroke-dashoffset] group-hover:[stroke-dashoffset:0]" />
        </svg>
      </span>
      <span className="min-w-0 flex-1 truncate text-base leading-5 text-foreground">{children}</span>
      {trailing && <span className="shrink-0 text-xs text-muted-foreground/70">{trailing}</span>}
    </Pressable>
  );
}
