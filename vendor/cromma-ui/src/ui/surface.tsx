/**
 * [INPUT]: 依赖 react 的 forwardRef、cn。
 * [OUTPUT]: 提供 Surface、surfaceClass、surfaceStyle、SurfaceGlow。Surface 的 elevation 是 panel、menu、raised。
 * [POS]: cromma-ui 的厚表面。Button 的受光配方换成卡片色。菜单与官网提问面共用。壳的面板卡另走 Panel。零件给 Radix Content 这类自带元素的场合。缺省圆角是独立的 Laper 面板圆角 rounded-panel。菜单经 menuSurfaceClass 换回小档。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { forwardRef, type CSSProperties, type HTMLAttributes } from 'react';
import { cn } from '../cn';

export type SurfaceElevation = 'panel' | 'menu' | 'raised';

/** Button 的 emphasisStyle 换成卡片色：同一套变量名，同一层受光；投影走 theme.css 的 --shadow-surface-* */
export function surfaceStyle(elevation: SurfaceElevation = 'panel'): CSSProperties {
  return {
    // raised：描边加深到 22%（panel 的 10% 在同色的浅底上几乎看不见）——应用页那张卡坐在 bg-panel 上用它
    '--btn-emphasis-ring': elevation === 'raised' ? 'color-mix(in oklch, var(--card), var(--foreground) 22%)' : 'color-mix(in oklch, var(--card), var(--foreground) 10%)',
    '--btn-emphasis-bg': 'color-mix(in oklch, var(--card), var(--inverse) 30%)',
    '--btn-emphasis-from': 'color-mix(in oklch, var(--card), var(--inverse) 15%)',
    '--btn-emphasis-to': 'var(--card)',
    '--surface-shadow': elevation === 'panel' ? 'var(--shadow-surface-panel)' : 'var(--shadow-surface-menu)',
  } as CSSProperties;
}

/** 底 + ring + 投影（投影经变量走 class，让外部 box-shadow 仍能覆盖） */
export const surfaceClass = 'relative isolate overflow-hidden border-0 bg-(--btn-emphasis-bg) ring ring-(--btn-emphasis-ring) shadow-(--surface-shadow)';

/** 受光层：-z-1 在 isolate 容器里压在底色之上、内容之下 */
export function SurfaceGlow() {
  return <span aria-hidden="true" className="pointer-events-none absolute inset-0 -z-1 rounded-[inherit] bg-linear-to-b from-(--btn-emphasis-from) to-(--btn-emphasis-to) shadow-[inset_0_1px_0_0_var(--laper-etch-highlight)]" />;
}

type Props = { elevation?: SurfaceElevation } & HTMLAttributes<HTMLDivElement>;

// 圆角不随 elevation：坐在 dashboard 里的大卡也用 menu 的皮，形状却是面板；菜单的小圆角由 menuSurfaceClass 覆盖

export const Surface = forwardRef<HTMLDivElement, Props>(function Surface({ elevation = 'panel', className, style, children, ...rest }, ref) {
  return (
    <div ref={ref} className={cn(surfaceClass, 'rounded-panel', className)} style={{ ...surfaceStyle(elevation), ...style }} {...rest}>
      <SurfaceGlow />
      {children}
    </div>
  );
});
