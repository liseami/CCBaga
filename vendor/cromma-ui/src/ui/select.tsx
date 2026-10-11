/**
 * [INPUT]: 依赖 radix-ui 的 Select（键盘、首字母跳转、读屏语义），cn，control 的几何，glyphs 的 CheckGlyph、ChevronGlyph，menu-row 的 MenuRow，popover 的菜单层皮。
 * [OUTPUT]: 提供 SelectItem 类型与 Select。Select 含 items、value、onValueChange、placeholder、size，并补充 id 与 aria-label 透传。触发器长得像 Input field。列表是 MenuRow。
 * [POS]: cromma-ui 的选择器。Kumo Select 迁来。禁止裸写 select。行为归 Radix。长相只用已有的菜单层与 MenuRow，不另起一套菜单。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { ReactNode } from 'react';
import { Select as SelectPrimitive } from 'radix-ui';
import { cn } from '../cn';
import { CONTROL_DEFAULT_SIZE, CONTROL_HEIGHT, CONTROL_RADIUS, type ControlSize } from './control';
import { CheckGlyph, ChevronGlyph } from './glyphs';
import { MenuRow } from './menu-row';
import { menuLayerClass, menuLayerStyle } from './popover';
import { SurfaceGlow } from './surface';

export interface SelectItem {
  value: string;
  label: ReactNode;
  disabled?: boolean;
}

const TYPE = 'text-xl text-foreground sm:text-base';

// Radix 把空串留给「清空选择」，不许当某一项的值；而「未设置」这一项常以空串表示：进出 Radix 时换成一个不会撞上的记号
const EMPTY = '\u0000empty';
const enc = (v: string | undefined) => (v === '' ? EMPTY : v);
const dec = (v: string) => (v === EMPTY ? '' : v);

export function Select({
  items,
  value,
  defaultValue,
  onValueChange,
  placeholder,
  size = CONTROL_DEFAULT_SIZE,
  disabled,
  className,
  id,
  "aria-label": ariaLabel,
}: {
  id?: string;
  "aria-label"?: string;
  items: SelectItem[];
  value?: string;
  defaultValue?: string;
  onValueChange?: (value: string) => void;
  placeholder?: ReactNode;
  size?: ControlSize;
  disabled?: boolean;
  className?: string;
}) {
  return (
    <SelectPrimitive.Root value={enc(value)} defaultValue={enc(defaultValue)} onValueChange={(v) => onValueChange?.(dec(v))} disabled={disabled}>
      <SelectPrimitive.Trigger
        id={id}
        aria-label={ariaLabel}
        className={cn(
          'flex w-full items-center justify-between gap-2 border border-border bg-input px-3 outline-none',
          CONTROL_HEIGHT[size],
          CONTROL_RADIUS,
          TYPE,
          'focus:border-primary focus:shadow-glow-input disabled:cursor-not-allowed disabled:opacity-60',
          className,
        )}
      >
        <span className="min-w-0 truncate">
          <SelectPrimitive.Value placeholder={<span className="text-muted-foreground">{placeholder}</span>} />
        </span>
        <SelectPrimitive.Icon asChild>
          <ChevronGlyph className="text-muted-foreground" />
        </SelectPrimitive.Icon>
      </SelectPrimitive.Trigger>
      <SelectPrimitive.Portal>
        {/* popper：列表贴在触发器下方、与它同宽（与 Popover 的菜单同一种出场），不走 macOS 式的覆盖选中项 */}
        <SelectPrimitive.Content position="popper" align="start" sideOffset={6} className={cn(menuLayerClass, 'max-h-(--radix-select-content-available-height) w-(--radix-select-trigger-width)')} style={menuLayerStyle}>
          <SurfaceGlow />
          <SelectPrimitive.Viewport className="flex flex-col gap-0.5">
            {items.map((it) => (
              <SelectPrimitive.Item key={it.value} value={enc(it.value)!} disabled={it.disabled} asChild>
                <MenuRow as="div" justify="between" className="data-[state=checked]:bg-accent data-[state=checked]:font-medium">
                  <SelectPrimitive.ItemText>
                    <span className="truncate">{it.label}</span>
                  </SelectPrimitive.ItemText>
                  <SelectPrimitive.ItemIndicator>
                    <CheckGlyph className="text-muted-foreground" />
                  </SelectPrimitive.ItemIndicator>
                </MenuRow>
              </SelectPrimitive.Item>
            ))}
          </SelectPrimitive.Viewport>
        </SelectPrimitive.Content>
      </SelectPrimitive.Portal>
    </SelectPrimitive.Root>
  );
}
