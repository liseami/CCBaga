/**
 * [INPUT]: PikaIconProps 的尺寸与 SVG 属性。
 * [OUTPUT]: cromma-web 同款 HomeDefault 图标。
 * [POS]: 从本机 Pika 生成产物复制的图形，供侧栏导航使用，不改变源 SVG 路径。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
// 由 cromma-ui/scripts/gen-icons.ts 生成，不手改（源：icons/pika）
import type { PikaIconProps } from '../types';
export default function HomeDefault({
  size = 24,
  color,
  className,
  ariaLabel,
  ...props
}: PikaIconProps) {
  const iconStyle = {
    color: color || 'currentColor'
  };
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" className={className} role="img" aria-label={ariaLabel ?? 'home-default icon'} style={iconStyle} {...props}><path stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M3 12.759c0-1.017 0-1.526.119-2.002a4 4 0 0 1 .513-1.19c.265-.414.634-.763 1.374-1.461l2.6-2.456c1.546-1.46 2.32-2.19 3.201-2.466a4 4 0 0 1 2.386 0c.882.275 1.655 1.006 3.201 2.466l2.6 2.456c.74.698 1.11 1.047 1.374 1.46a4 4 0 0 1 .513 1.191c.119.476.119.985.119 2.002v3.574c0 1.085 0 1.628-.12 2.073a3.5 3.5 0 0 1-2.474 2.475c-.445.119-.988.119-2.073.119-.31 0-.465 0-.592-.034a1 1 0 0 1-.707-.707C15 20.132 15 19.977 15 19.667V17c0-.465 0-.697-.03-.891a2.5 2.5 0 0 0-2.079-2.078C12.697 14 12.464 14 12 14s-.697 0-.891.03a2.5 2.5 0 0 0-2.079 2.08C9 16.303 9 16.535 9 17v2.667c0 .31 0 .465-.034.592a1 1 0 0 1-.707.707C8.132 21 7.977 21 7.667 21c-1.085 0-1.628 0-2.073-.12a3.5 3.5 0 0 1-2.475-2.474C3 17.96 3 17.418 3 16.333z" fill="none" /></svg>;
}
