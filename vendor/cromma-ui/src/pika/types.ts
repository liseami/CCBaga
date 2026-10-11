/**
 * [INPUT]: 依赖 react 的 SVGProps 类型。
 * [OUTPUT]: 提供 PikaIconProps。PikaIconProps是全部 Pika 图标的 props 契约（size、color、className、ariaLabel）。
 * [POS]: icons/pika的类型根，一千多个形状一致的图标只在这里写一次类型。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import type { SVGProps } from 'react';

export interface PikaIconProps extends Omit<SVGProps<SVGSVGElement>, 'color'> {
  size?: number | string;
  color?: string;
  ariaLabel?: string;
}
