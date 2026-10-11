/**
 * [INPUT]: 依赖 Motion 的布局与无障碍动效，motion-tokens 的通用三档与 media-viewer-tokens 的专用媒体参数。
 * [OUTPUT]: 提供 MOTION、MOTION_EASE、PUSH、MotionRoot、useLayoutTransition（显式布局过渡也尊重减弱动态效果），转出 motion、AnimatePresence、LayoutGroup、useIsPresent、useReducedMotion 与类型 Transition。MOTION是web 动效系统的预设。PUSH是推入推出的交叉滑编舞。MotionRoot是根上的 MotionConfig。
 * [POS]: web 动效的唯一真相源：一条曲线（键盘曲线 7的贝塞尔，搜索态那一条）× 三档时长（手机全局三档 × web的 speed）。见 docs/09「动效」CSS 过渡读同值的 --motion-quick、--motion-base、--motion-panel 与 ease-motion，JS 动画只取这里的 MOTION.*，不另写时长与曲线。Motion 只经本文件进 cromma-ui的入口。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { createElement, type ReactNode } from 'react';
import { MotionConfig, useReducedMotion, type Transition } from 'motion/react';
import { MOTION_TOKENS as T } from './motion-tokens';
import { MEDIA_VIEWER as MEDIA } from './media-viewer-tokens';

export { AnimatePresence, LayoutGroup, motion, useIsPresent, useReducedMotion } from 'motion/react';
export type { Transition } from 'motion/react';

/** web 唯一的曲线：快起、长尾、零过冲（y ≤ 1），CSS 侧是 ease-motion */
export const MOTION_EASE = T.ease;

const tween = (duration: number, ease: Transition['ease'] = MOTION_EASE): Transition => ({ type: 'tween', duration, ease });

/**
 * 预设：挑档不挑数。
 * - panel：位置与尺寸的形变（搜索态升降、资料栏开合、列宽让位、侧栏宽窄、推入推出）——Motion 的 layout 与 x / width 都用它
 * - base：内容换页的交叉淡化、浮层与模态的进出
 * - quick：菜单、提示、小控件与悬停
 * - fadeIn / fadeOut：只动透明度的那一层（结果层、页头一行）：进线性 quick、出 easeInOut base，与搜索态同一副编舞
 */
export const MOTION = {
  panel: tween(T.panel),
  /** Laper 专用媒体锚点形变；不改变普通布局的三档。 */
  mediaOpen: tween(MEDIA.openDuration, [...MEDIA.ease]),
  mediaClose: tween(MEDIA.closeDuration, [...MEDIA.ease]),
  /** 壳布局与父子换页：与 base 同时长，可打断，不追着拖拽跑。 */
  layout: tween(T.base),
  base: tween(T.base),
  quick: tween(T.quick),
  fadeIn: tween(T.quick, 'linear'),
  fadeOut: tween(T.base, 'easeInOut'),
  /** 不动：无动画的退出（打开搜索结果）与首帧 */
  instant: { duration: 0 } as Transition,
} as const;

/** 推入推出时页移的那一小段（px）：web 的页在卡里换，不像手机整屏推，只留一个方向感 */
const PUSH_SHIFT = 24;

/**
 * 推入推出的交叉滑（Motion variants，custom = 方向：1 推入 · -1 退回）：新页从行进方向的后沿移入并淡入，旧页向前沿让位并淡出；
 * covered 是被压在栈下、仍挂着的页（保住滚动与草稿）。资料栏的一摞页与朋友圈的二级视图同用（ui/push-swap 的 PushSwap 是它的现成外壳）
 */
export const PUSH = {
  enter: (dir: number) => ({ x: dir * PUSH_SHIFT, opacity: 0 }),
  center: { x: 0, opacity: 1, transition: { x: MOTION.layout, opacity: MOTION.fadeIn } },
  exit: (dir: number) => ({ x: -dir * PUSH_SHIFT, opacity: 0, transition: { x: MOTION.layout, opacity: MOTION.quick } }),
  covered: { x: -PUSH_SHIFT, opacity: 0, transition: { x: MOTION.layout, opacity: MOTION.quick } },
} as const;

/**
 * 根上挂一次：缺省过渡 = layout（layout 动画不写 transition 也同一个手感），系统「减弱动态效果」打开时位移与形变直接到位、只留透明度。
 * 壳的 main 与每个 Cromma App 的根各包一层；CSS 侧的减弱动效在 theme.css 把 --motion-* 归零
 */
export function MotionRoot({ children }: { children: ReactNode }) {
  return createElement(MotionConfig, { reducedMotion: 'user', transition: MOTION.layout }, children);
}

/** 显式 width / 位移动画也尊重系统减弱动态效果，不只依赖 MotionConfig 的 transform 处理。 */
export function useLayoutTransition(): Transition {
  return useReducedMotion() ? MOTION.instant : MOTION.layout;
}
