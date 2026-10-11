/**
 * [INPUT]: 依赖 react 的 forwardRef、cn。
 * [OUTPUT]: 提供 Pressable。形态是 button 或 div。bare 换焦点环为细描边。type 缺省 button。div 形态认 Enter、Space。
 * [POS]: cromma-ui 的可点底座。有固定长相的用 Button、SidebarButton、MenuRow、Chip。长相独一无二的用它。as=div 给内嵌按钮的行。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { forwardRef, type ButtonHTMLAttributes, type HTMLAttributes, type KeyboardEvent, type MouseEvent } from 'react';
import { cn } from '../cn';

type Props = {
  as?: 'button' | 'div';
  /** 换掉默认的 focus-visible 环（表面自己画焦点态时）：只留一道贴外沿的细描边，键盘用户仍看得见焦点 */
  bare?: boolean;
} & ButtonHTMLAttributes<HTMLButtonElement> &
  HTMLAttributes<HTMLDivElement>;

export const Pressable = forwardRef<HTMLButtonElement & HTMLDivElement, Props>(function Pressable({ as = 'button', bare = false, className, children, ...rest }, ref) {
  const classes = cn(
    'cursor-pointer select-none outline-none disabled:cursor-not-allowed aria-disabled:cursor-not-allowed',
    bare ? 'focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-primary/40' : 'focus-visible:ring-2 focus-visible:ring-primary/40',
    className,
  );
  if (as === 'div') {
    const { onKeyDown, onClick, ...divRest } = rest as HTMLAttributes<HTMLDivElement>;
    // role=button 的 div 要像按钮一样认键盘：Enter / Space 等同一次点击（Space 顺手挡掉页面滚动）；只认自己身上的按键，内嵌控件的回车不冒泡成点击
    const keyDown = (e: KeyboardEvent<HTMLDivElement>) => {
      onKeyDown?.(e);
      if (e.defaultPrevented || e.target !== e.currentTarget || (e.key !== 'Enter' && e.key !== ' ')) return;
      if (e.currentTarget.getAttribute('aria-disabled') === 'true') return;
      e.preventDefault();
      onClick?.(e as unknown as MouseEvent<HTMLDivElement>);
    };
    return (
      <div ref={ref} role="button" tabIndex={0} className={classes} onClick={onClick} onKeyDown={keyDown} {...divRest}>
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
