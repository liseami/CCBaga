/**
 * [INPUT]: tokens.yaml 的 mediaViewer 段，由 gen-tokens 生成。
 * [OUTPUT]: MEDIA_VIEWER：装框、chrome、控制卡与进度条几何、自动隐藏与键盘步长、倍速档与锚点过渡；长度px，时长秒。
 * [POS]: web与桌面专用媒体查看器同源（几何另以 --media-* 落 tokens.css），不改手机viewer；不手改。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
export const MEDIA_VIEWER = {
  "radius": 32,
  "sourceRadius": 8,
  "openDuration": 0.4,
  "closeDuration": 0.4,
  "controlsDelay": 0.08,
  "ease": [
    0.32,
    0.72,
    0,
    1
  ],
  "stageInsetX": 72,
  "stageInsetY": 64,
  "videoMinWidth": 344,
  "chromeInset": 16,
  "chromeButton": 40,
  "chromeIcon": 18,
  "barInset": 12,
  "barRadius": 20,
  "barPadding": 8,
  "barBlur": 24,
  "barButton": 36,
  "barIcon": 20,
  "barButtonGap": 4,
  "barClusterGap": 8,
  "barMinWidth": 320,
  "barCompactWidth": 400,
  "scrubRow": 20,
  "scrubInset": 8,
  "track": 3,
  "trackHover": 5,
  "thumb": 12,
  "thumbDrag": 14,
  "tipGap": 8,
  "volumeWidth": 64,
  "speedPadH": 8,
  "centerPlay": 64,
  "idleHide": 2.5,
  "revealDistance": 5,
  "bufferingDelay": 0.3,
  "seekStep": 5,
  "seekStepLong": 10,
  "volumeStep": 0.1,
  "rates": [
    0.5,
    1,
    1.25,
    1.5,
    2
  ]
} as const;
