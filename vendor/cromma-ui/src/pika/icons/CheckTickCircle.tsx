/**
 * [INPUT]: PikaIconProps 的尺寸与 SVG 属性。
 * [OUTPUT]: cromma-web 同款 CheckTickCircle 图标。
 * [POS]: 从本机 Pika 生成产物复制的图形，不改变源 SVG 路径。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
// 由 cromma-ui/scripts/gen-icons.ts 生成，不手改（源：icons/pika）
import type { PikaIconProps } from '../types';
export default function CheckTickCircle({
  size = 24,
  color,
  className,
  ariaLabel,
  ...props
}: PikaIconProps) {
  const iconStyle = {
    color: color || 'currentColor'
  };
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" className={className} role="img" aria-label={ariaLabel ?? 'check-tick-circle icon'} style={iconStyle} {...props}><path stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="m8.5 12.513 2.341 2.338A15 15 0 0 1 15.4 9.915l.101-.069M21.15 12a9.15 9.15 0 1 1-18.3 0 9.15 9.15 0 0 1 18.3 0Z" fill="none" /></svg>;
}