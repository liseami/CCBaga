/**
 * [INPUT]: 依赖 radix-ui 的 Popover、cn、surface 的零件。
 * [OUTPUT]: 提供 Popover、PopoverTrigger、PopoverAnchor、PopoverContent、menuSurfaceClass，以及菜单层的皮 menuLayerClass、menuLayerStyle。
 * [POS]: cromma-ui 的浮层原子。菜单表面是 Surface（menu）加 menuSurfaceClass。自绘菜单 UserMenu 也借 menuSurfaceClass。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { Popover as PopoverPrimitive } from 'radix-ui';
import { cn } from '../cn';
import { SurfaceGlow, surfaceClass, surfaceStyle } from './surface';

export const Popover = PopoverPrimitive.Root;
export const PopoverTrigger = PopoverPrimitive.Trigger;
export const PopoverAnchor = PopoverPrimitive.Anchor;

/** 菜单表面的形状（Surface 之外的部分）：UserMenu 这类不经 Radix 的自绘菜单也用它 */
// gap-0.5 让相邻的 hover 行与选中行之间留一线空隙，不留就是两块贴着的灰
export const menuSurfaceClass = 'flex flex-col gap-0.5 rounded-2xl p-1.5 text-popover-foreground';

/**
 * 一切浮起的菜单层（Popover、Select 的列表、Dropdown）同一副皮：Surface（menu）+ 菜单形状 + 入场动效；
 * 层级高于模态（modal 从 1000 起逐层 +10）：模态里弹出的菜单不被自己的模态盖住
 */
export const menuLayerClass = cn('laper-menu-motion z-[1500] outline-none', surfaceClass, menuSurfaceClass);
export const menuLayerStyle = surfaceStyle('menu');

export function PopoverContent({ className, align = 'start', sideOffset = 6, style, children, ...props }: PopoverPrimitive.PopoverContentProps) {
  return (
    <PopoverPrimitive.Portal>
      <PopoverPrimitive.Content align={align} sideOffset={sideOffset} className={cn(menuLayerClass, 'w-72', className)} style={{ ...menuLayerStyle, ...style }} {...props}>
        <SurfaceGlow />
        {children}
      </PopoverPrimitive.Content>
    </PopoverPrimitive.Portal>
  );
}
