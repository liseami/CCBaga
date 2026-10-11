/**
 * [INPUT]: PikaIconProps 的尺寸与 SVG 属性。
 * [OUTPUT]: cromma-web 同款 Translate 图标。
 * [POS]: 从本机 Pika 生成产物复制的图形，不改变源 SVG 路径。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
// 由 cromma-ui/scripts/gen-icons.ts 生成，不手改（源：icons/pika）
import type { PikaIconProps } from '../types';
export default function Translate({
  size = 24,
  color,
  className,
  ariaLabel,
  ...props
}: PikaIconProps) {
  const iconStyle = {
    color: color || 'currentColor'
  };
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" className={className} role="img" aria-label={ariaLabel ?? 'translate icon'} style={iconStyle} {...props}><path stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 5h4m0 0h4a8 8 0 0 1-4 6.93M7 5V3m0 8.93A7.96 7.96 0 0 1 3 13m4-1.07A8.04 8.04 0 0 1 4.07 9M7 11.93A7.96 7.96 0 0 0 11 13m10 8v-.063c0-.993-.105-1.977-.31-2.937M13 21v-.063c0-.993.105-1.977.31-2.937m0 0a14.1 14.1 0 0 1 2.774-5.855 1.173 1.173 0 0 1 1.832 0A14.1 14.1 0 0 1 20.69 18m-7.38 0h7.38" fill="none" /></svg>;
}