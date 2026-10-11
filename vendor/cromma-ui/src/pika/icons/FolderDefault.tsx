/**
 * [INPUT]: PikaIconProps 的尺寸与 SVG 属性。
 * [OUTPUT]: cromma-web 同款 FolderDefault 图标。
 * [POS]: 从本机 Pika 生成产物复制的图形，不改变源 SVG 路径。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
// 由 cromma-ui/scripts/gen-icons.ts 生成，不手改（源：icons/pika）
import type { PikaIconProps } from '../types';
export default function FolderDefault({
  size = 24,
  color,
  className,
  ariaLabel,
  ...props
}: PikaIconProps) {
  const iconStyle = {
    color: color || 'currentColor'
  };
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" className={className} role="img" aria-label={ariaLabel ?? 'folder-default icon'} style={iconStyle} {...props}><path stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M2 9.4c0-2.24 0-3.36.436-4.216a4 4 0 0 1 1.748-1.748C5.04 3 6.16 3 8.4 3h.316c.47 0 .704 0 .917.065a1.5 1.5 0 0 1 .517.276c.172.142.302.337.563.728l.575.862c.26.391.39.586.562.728a1.5 1.5 0 0 0 .517.276c.213.065.448.065.917.065H15.6c2.24 0 3.36 0 4.216.436a4 4 0 0 1 1.748 1.748C22 9.04 22 10.16 22 12.4v2.2c0 2.24 0 3.36-.436 4.216a4 4 0 0 1-1.748 1.748C18.96 21 17.84 21 15.6 21H8.4c-2.24 0-3.36 0-4.216-.436a4 4 0 0 1-1.748-1.748C2 17.96 2 16.84 2 14.6z" fill="none" /></svg>;
}