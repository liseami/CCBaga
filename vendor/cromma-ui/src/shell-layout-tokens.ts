/**
 * [INPUT]: 依赖 tokens.yaml 的 shellLayout、rail 段与 panel.border。生成者是 gen-tokens。
 * [OUTPUT]: 提供 SHELL_LAYOUT。它是主从布局的宽度预算、窗底一行高 dock、Apps 应用列表对消息列表的宽比 appsPanelRatio、应用页右侧 AI 面板的宽 makerPanel 与拖宽上下限、sidebar 两档宽与 frame 一道边的宽 frameBorder。
 * [POS]: web 壳的纯数值入口。桌面的 shell_layout、rail 与 panel::BORDER 与它同源。不手改。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
export const SHELL_LAYOUT = {
  "panelMin": 320,
  "panelDefault": 380,
  "dashboardMin": 480,
  "windowMin": 520,
  "divider": 1,
  "gap": 8,
  "titlebar": 40,
  "dock": 40,
  "appsPanelRatio": 0.618,
  "makerPanel": 380,
  "makerPanelMin": 320,
  "makerPanelMax": 640,
  "sidebarExpanded": 224,
  "sidebarCompact": 52,
  "frameBorder": 1
} as const;
