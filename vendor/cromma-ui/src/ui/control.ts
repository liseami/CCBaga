/**
 * [INPUT]: 无依赖，纯常量。
 * [OUTPUT]: 提供 ControlSize、CONTROL_HEIGHT、CONTROL_RADIUS、CONTROL_DEFAULT_SIZE、ICON_TILE。CONTROL_HEIGHT是三档高度，Laper SIZE_CONFIG 原值。ICON_TILE是图标砖的圆角方。
 * [POS]: cromma-ui的控件几何唯一真相源：Button、Input、Select、Chip、Meter 等共用，同 size 外沿必然齐平同弧，消费方不再手写 h-*、rounded-*。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */

export type ControlSize = 'small' | 'medium' | 'large';

/**
 * 三档高度（Laper SIZE_CONFIG 原样：large 在 xl 断点长一格）。
 * 高度说的是**看得见的外沿**，不是盒子：边画在盒内的（白皮的发丝、输入框的边）盒子就是这么高；
 * 边画在盒外的（强调钮的 1px 外环）盒子要收回去、再用外边距还回占位——同档并排时外沿齐平，布局不动（button.tsx 的 EMPHASIS_FIT）。
 * 新原语若把边或投影画出盒外，照同一个办法收；消费方不许用 h-* / w-* 凑高度，否则收边被绕开
 */
export const CONTROL_HEIGHT: Record<ControlSize, string> = {
  small: 'h-7',
  medium: 'h-8',
  large: 'h-9 xl:h-10',
};

/** 控件共用圆角（Button 自 Laper 起就是 rounded-2xl；输入框同弧才同构） */
export const CONTROL_RADIUS = 'rounded-2xl';

/** 缺省档 · 与 Button 一致（Laper 默认 small） */
export const CONTROL_DEFAULT_SIZE: ControlSize = 'small';

/** 图标砖（应用图标、连接器标）的圆角方：写成 class，theme.css 的 squircle 才吃得到；40% 才是 iOS 图标那种饱满的连续圆角。
 *  头像不用它——一切露脸走 Avatar 的舷窗 */
export const ICON_TILE = 'rounded-[40%]';
