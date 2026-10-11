// 由 cromma-ui/scripts/gen-tokens.ts 从 tokens/tokens.yaml 生成，不手改；改令牌改 tokens.yaml 再跑 bun run gen
/**
 * [INPUT]: 由 scripts/gen-tokens.ts 读取 tokens/tokens.yaml 的 motion.web（手机全局三档、键盘弹簧的时长、motion.ease 的曲线）生成。
 * [OUTPUT]: 对外提供 MOTION_TOKENS（web 动效系统三档的秒数与那条曲线，与 tokens.css 的 --motion-* 同值）。
 * [POS]: web 动效系统的令牌，只由 ./motion.ts 读成 Motion 预设；产物不手改。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */

export const MOTION_TOKENS = {
  speed: 0.7,
  quick: 0.14,
  base: 0.182,
  panel: 0.35,
  ease: [0.38, 0.7, 0.125, 1] as [number, number, number, number],
} as const;
