# vendor/cromma-ui/src/
> L2 | 父级: ../CLAUDE.md

成员清单
cn.ts: Tailwind 类名合并入口，冲突优先级由 tailwind-merge 统一处理。
index.ts: 应用组件与 TooltipProvider 导出入口；从原版库提取实际使用控件与其依赖闭包。
media-viewer-tokens.ts: 原版媒体动效参数，作为 motion.ts 的静态依赖保持同款预设。
motion-tokens.ts: 原版动效时长与曲线，与主题 CSS 同源。
motion.ts: MotionRoot 与动效预设，统一系统减少动态效果行为。
pika/: Pika 图标入口与原版 SVG。
shell-layout-tokens.ts: Panel 分隔线等原版壳布局参数。
theme.css: 复制的全站主题，负责控件皮肤、令牌引入、圆角回退与减少动态效果。
tokens.css: 复制的生成令牌，颜色、尺寸、字体和动效的统一视觉契约。
ui/: 交互原语与依赖闭包。
[PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
