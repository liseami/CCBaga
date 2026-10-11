/**
 * [INPUT]: PikaIconProps 的尺寸与 SVG 属性。
 * [OUTPUT]: cromma-web 同款 EraserDefault 图标。
 * [POS]: 从本机 Pika 生成产物复制的图形，不改变源 SVG 路径。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
// 由 cromma-ui/scripts/gen-icons.ts 生成，不手改（源：icons/pika）
import type { PikaIconProps } from '../types';
export default function EraserDefault({
  size = 24,
  color,
  className,
  ariaLabel,
  ...props
}: PikaIconProps) {
  const iconStyle = {
    color: color || 'currentColor'
  };
  return <svg width={size} height={size} viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg" className={className} role="img" aria-label={ariaLabel ?? 'eraser-default icon'} style={iconStyle} {...props}><path stroke="currentColor" strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="m14.268 17.292-.907.908c-1.27 1.27-1.905 1.905-2.638 2.143-.644.21-1.338.21-1.982 0-.733-.238-1.368-.873-2.638-2.143l-.303-.303c-1.27-1.27-1.905-1.905-2.143-2.638a3.2 3.2 0 0 1 0-1.982c.238-.732.873-1.368 2.143-2.638l.908-.907m7.56 7.56 3.932-3.931c1.27-1.27 1.905-1.905 2.143-2.638.21-.644.21-1.338 0-1.982-.238-.733-.873-1.368-2.143-2.638l-.303-.303c-1.27-1.27-1.905-1.905-2.638-2.143a3.2 3.2 0 0 0-1.982 0c-.732.238-1.368.873-2.638 2.143L6.708 9.732m7.56 7.56-7.56-7.56" fill="none" /></svg>;
}