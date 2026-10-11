/**
 * [INPUT]: PikaIconProps 的尺寸与 SVG 属性。
 * [OUTPUT]: cromma-web 同款 Refresh 图标。
 * [POS]: 从本机 Pika 生成产物复制的图形，不改变源 SVG 路径。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
// 由 cromma-ui/scripts/gen-icons.ts 生成，不手改（源：icons/pika）
import type { PikaIconProps } from '../types';
export default function Refresh({
  size = 24,
  color,
  className,
  ariaLabel,
  ...props
}: PikaIconProps) {
  const iconStyle = {
    color: color || 'currentColor'
  };
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" className={className} role="img" aria-label={ariaLabel ?? 'refresh icon'} style={iconStyle} {...props}><path stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M17.5 2.474A15 15 0 0 1 18.548 6.2a.48.48 0 0 1-.297.515l-.181.07M6.5 21.527A15 15 0 0 1 5.45 17.8a.48.48 0 0 1 .297-.515l.182-.07M14.5 7.67a15 15 0 0 0 3.57-.884m0 0a8 8 0 0 0-13.912 6.797m15.75-2.79A8 8 0 0 1 5.93 17.215m3.57-.885a15 15 0 0 0-3.57.884" fill="none" /></svg>;
}