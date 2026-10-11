/**
 * [INPUT]: 依赖 radix-ui 的 Checkbox（空格切换、不定态、checkbox 语义），react 的 createContext、useContext、forwardRef，cn，glyphs 的 CheckGlyph、MinusGlyph。
 * [OUTPUT]: 提供 CheckboxProps、Checkbox、Checkbox.Group、Checkbox.Item。
 * [POS]: cromma-ui 的勾选。Kumo Checkbox 迁来。行为归 Radix。方盒走 token。选中是前景实底。不定态画减号。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { createContext, forwardRef, useContext, type ButtonHTMLAttributes, type ReactNode } from 'react';
import { Checkbox as CheckboxPrimitive } from 'radix-ui';
import { cn } from '../cn';
import { CheckGlyph, MinusGlyph } from './glyphs';

export interface CheckboxProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'onChange' | 'checked' | 'defaultChecked' | 'value'> {
  label?: ReactNode;
  checked?: boolean;
  defaultChecked?: boolean;
  indeterminate?: boolean;
  onCheckedChange?: (checked: boolean) => void;
}

const CheckboxBase = forwardRef<HTMLButtonElement, CheckboxProps>(function Checkbox({ className, label, checked, defaultChecked, indeterminate, disabled, onCheckedChange, ...rest }, ref) {
  const control = (
    <CheckboxPrimitive.Root
      ref={ref}
      checked={indeterminate ? 'indeterminate' : checked}
      defaultChecked={defaultChecked}
      disabled={disabled}
      onCheckedChange={(v) => onCheckedChange?.(v === true)}
      className={cn(
        'flex size-4 shrink-0 items-center justify-center rounded-md border border-border bg-background text-background outline-none',
        'data-[state=checked]:border-foreground data-[state=checked]:bg-foreground data-[state=indeterminate]:border-foreground data-[state=indeterminate]:bg-foreground',
        'focus-visible:ring-2 focus-visible:ring-primary/40 disabled:opacity-50',
        className,
      )}
      {...rest}
    >
      <CheckboxPrimitive.Indicator>{indeterminate ? <MinusGlyph /> : <CheckGlyph />}</CheckboxPrimitive.Indicator>
    </CheckboxPrimitive.Root>
  );

  if (!label) return control;
  return (
    <label className={cn('inline-flex items-start gap-2 text-base', disabled ? 'cursor-not-allowed opacity-60' : 'cursor-pointer')}>
      <span className="mt-0.5 inline-flex">{control}</span>
      <span>{label}</span>
    </label>
  );
});

const GroupCtx = createContext<{ values: string[]; toggle: (v: string) => void; disabled?: boolean } | null>(null);

function CheckboxGroup({
  legend,
  value,
  onValueChange,
  disabled,
  className,
  children,
}: {
  legend?: string;
  value: string[];
  onValueChange: (value: string[]) => void;
  disabled?: boolean;
  className?: string;
  children: ReactNode;
}) {
  const toggle = (v: string) => {
    if (disabled) return;
    onValueChange(value.includes(v) ? value.filter((x) => x !== v) : [...value, v]);
  };
  return (
    <GroupCtx.Provider value={{ values: value, toggle, disabled }}>
      <fieldset disabled={disabled} className={cn('flex flex-col gap-2', className)}>
        {legend ? <legend className="mb-1 text-base font-medium text-foreground">{legend}</legend> : null}
        {children}
      </fieldset>
    </GroupCtx.Provider>
  );
}

function CheckboxItem({ value, label, className }: { value: string; label: ReactNode; className?: string }) {
  const ctx = useContext(GroupCtx);
  if (!ctx) throw new Error('Checkbox.Item must live inside Checkbox.Group');
  return <Checkbox className={className} label={label} checked={ctx.values.includes(value)} disabled={ctx.disabled} onCheckedChange={() => ctx.toggle(value)} />;
}

export const Checkbox = Object.assign(CheckboxBase, { Group: CheckboxGroup, Item: CheckboxItem });
