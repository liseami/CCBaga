/**
 * [INPUT]: 依赖 .cn 的 cn。
 * [OUTPUT]: 提供 对内提供线框字形（CheckGlyph、MinusGlyph、Chevron*Glyph、CopyGlyph、Eye*Glyph、PlayGlyph、PauseGlyph、DotGlyph、SearchGlyph、CloseGlyph、ClockGlyph、AtGlyph、PlusGlyph、MicGlyph、ArrowUpGlyph），不进包入口。
 * [POS]: cromma-ui的内部零件——新原语共用这一份 16 视口的 SVG，不引图标库、不进消费方。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { cn } from '../cn';

type GlyphProps = { className?: string };

const svg = 'shrink-0';

export function CheckGlyph({ className }: GlyphProps) {
  return (
    <svg viewBox="0 0 16 16" fill="none" aria-hidden="true" className={cn(svg, 'size-3', className)}>
      <path d="M3.5 8.5 6.5 11.5 12.5 4.5" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function MinusGlyph({ className }: GlyphProps) {
  return (
    <svg viewBox="0 0 16 16" fill="none" aria-hidden="true" className={cn(svg, 'size-3', className)}>
      <path d="M3.5 8h9" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
    </svg>
  );
}

export function ChevronGlyph({ className }: GlyphProps) {
  return (
    <svg viewBox="0 0 16 16" fill="none" aria-hidden="true" className={cn(svg, 'size-3.5', className)}>
      <path d="M4 6.5 8 10.5 12 6.5" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function CopyGlyph({ className }: GlyphProps) {
  return (
    <svg viewBox="0 0 16 16" fill="none" aria-hidden="true" className={cn(svg, 'size-3.5', className)}>
      <rect x="5.5" y="5.5" width="7" height="8" rx="1.2" stroke="currentColor" strokeWidth="1.4" />
      <path d="M3.5 10.5V3.8A1.3 1.3 0 0 1 4.8 2.5h6.2" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
    </svg>
  );
}

export function EyeGlyph({ className }: GlyphProps) {
  return (
    <svg viewBox="0 0 16 16" fill="none" aria-hidden="true" className={cn(svg, 'size-3.5', className)}>
      <path d="M2 8s2.4-4.5 6-4.5S14 8 14 8s-2.4 4.5-6 4.5S2 8 2 8Z" stroke="currentColor" strokeWidth="1.4" />
      <circle cx="8" cy="8" r="1.8" stroke="currentColor" strokeWidth="1.4" />
    </svg>
  );
}

export function EyeOffGlyph({ className }: GlyphProps) {
  return (
    <svg viewBox="0 0 16 16" fill="none" aria-hidden="true" className={cn(svg, 'size-3.5', className)}>
      <path d="M3 4.5 13 13" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
      <path d="M6.2 6.4A3.2 3.2 0 0 0 5.5 8 2.5 2.5 0 0 0 8 10.5c.5 0 1-.15 1.4-.4M8.8 5.55A2.5 2.5 0 0 1 10.5 8" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
      <path d="M2 8s2.4-4.5 6-4.5c1.1 0 2.1.4 2.95 1M14 8s-.6 1.1-1.6 2.15" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" />
    </svg>
  );
}

export function ChevronLeftGlyph({ className }: GlyphProps) {
  return (
    <svg viewBox="0 0 16 16" fill="none" aria-hidden="true" className={cn(svg, 'size-3.5', className)}>
      <path d="M10 3.5 5.5 8 10 12.5" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function ChevronRightGlyph({ className }: GlyphProps) {
  return (
    <svg viewBox="0 0 16 16" fill="none" aria-hidden="true" className={cn(svg, 'size-3.5', className)}>
      <path d="M6 3.5 10.5 8 6 12.5" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function PlayGlyph({ className }: GlyphProps) {
  return (
    <svg viewBox="0 0 16 16" fill="none" aria-hidden="true" className={cn(svg, 'size-3.5', className)}>
      <path d="M5 3.5v9l7.5-4.5z" fill="currentColor" stroke="currentColor" strokeWidth="1.2" strokeLinejoin="round" />
    </svg>
  );
}

export function PauseGlyph({ className }: GlyphProps) {
  return (
    <svg viewBox="0 0 16 16" fill="none" aria-hidden="true" className={cn(svg, 'size-3.5', className)}>
      <path d="M5.5 3.5v9M10.5 3.5v9" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" />
    </svg>
  );
}

export function DotGlyph({ className }: GlyphProps) {
  return <span aria-hidden="true" className={cn('size-1.5 shrink-0 rounded-full bg-current', className)} />;
}

export function SearchGlyph({ className }: GlyphProps) {
  return (
    <svg viewBox="0 0 16 16" fill="none" aria-hidden="true" className={cn(svg, 'size-4', className)}>
      <circle cx="7.2" cy="7.2" r="4.4" stroke="currentColor" strokeWidth="1.5" />
      <path d="m10.5 10.5 3 3" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  );
}

export function ClockGlyph({ className }: GlyphProps) {
  return (
    <svg viewBox="0 0 16 16" fill="none" aria-hidden="true" className={cn(svg, 'size-4', className)}>
      <circle cx="8" cy="8" r="5.6" stroke="currentColor" strokeWidth="1.4" />
      <path d="M8 5v3.2l2 1.3" stroke="currentColor" strokeWidth="1.4" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function CloseGlyph({ className }: GlyphProps) {
  return (
    <svg viewBox="0 0 16 16" fill="none" aria-hidden="true" className={cn(svg, 'size-3', className)}>
      <path d="m4.5 4.5 7 7M11.5 4.5l-7 7" stroke="currentColor" strokeWidth="1.6" strokeLinecap="round" />
    </svg>
  );
}

/** Pika at-mark（24 视口，与手机消息行的 @ 标同一枚） */
export function AtGlyph({ className }: GlyphProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" aria-hidden="true" className={cn(svg, 'size-3.5', className)}>
      <path d="M16 8v5.5a2.5 2.5 0 0 0 2.5 2.5c2.18 0 2.5-2.267 2.5-4a9 9 0 1 0-9 9c1.052 0 2.062-.18 3-.512M16 12a4 4 0 1 1-8 0 4 4 0 0 1 8 0Z" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function PlusGlyph({ className }: GlyphProps) {
  return (
    <svg viewBox="0 0 16 16" fill="none" aria-hidden="true" className={cn(svg, 'size-4', className)}>
      <path d="M8 3v10M3 8h10" stroke="currentColor" strokeWidth="1.5" strokeLinecap="round" />
    </svg>
  );
}

/** Pika mic-microphone（24 视口，与壳语义层的 Mic 同一枚）：对话输入胶囊的听写钮 */
export function MicGlyph({ className }: GlyphProps) {
  return (
    <svg viewBox="0 0 24 24" fill="none" aria-hidden="true" className={cn(svg, 'size-4', className)}>
      <path d="M16.655 11.561a4.28 4.28 0 1 0-4.216-4.218m.002-.022 4.238 4.238-9.908 8.865a2.263 2.263 0 0 1-3.195-3.195z" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}

export function ArrowUpGlyph({ className }: GlyphProps) {
  return (
    <svg viewBox="0 0 16 16" fill="none" aria-hidden="true" className={cn(svg, 'size-4', className)}>
      <path d="M8 13V3.5M3.75 7.5 8 3.25l4.25 4.25" stroke="currentColor" strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" />
    </svg>
  );
}
