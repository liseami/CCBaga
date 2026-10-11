/**
 * [INPUT]: 依赖 radix-ui 的 Slot、cn、control 的高度表与圆角。
 * [OUTPUT]: 提供 Button、ButtonVariant、ButtonSize、ButtonProps、ICON_PX、emphasisStyle、VARIANT_STYLES。
 * [POS]: cromma-ui 的按钮原语。强调钮逐字复刻 Laper、Kumo 受光配方。secondary 与 Chip 同一张 Tag 皮。muted 是白卡输入面里的灰底钮。media 是媒体黑玻璃上的白钮。有 href 时渲染 a。强调钮的外环画在盒外，盒子按 EMPHASIS_FIT 收回，与同档白钮外沿齐平。正圆图标钮用 round。media 自带 --media-bar-* 几何，不吃 SIZE 表。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useCallback, useState, type ButtonHTMLAttributes, type CSSProperties, type ReactNode } from 'react';
import { Slot as SlotPrimitive } from 'radix-ui';
import { cn } from '../cn';
import { CONTROL_DEFAULT_SIZE, CONTROL_HEIGHT, CONTROL_RADIUS, type ControlSize } from './control';

export type ButtonVariant = 'primary' | 'ink' | 'info' | 'warning' | 'danger' | 'success' | 'secondary' | 'ghost' | 'outline' | 'selected' | 'icon' | 'dashed' | 'muted' | 'media';
export type ButtonSize = ControlSize;

// ============ 尺寸（Laper SIZE_CONFIG / PADDING_CONFIG / ICON_SIZE_MAP 原样） ============
const SIZE: Record<ButtonSize, string> = {
  small: `${CONTROL_HEIGHT.small} gap-1.5 text-sm font-medium`,
  medium: `${CONTROL_HEIGHT.medium} gap-2 text-base font-medium`,
  large: `${CONTROL_HEIGHT.large} gap-2 text-base font-medium`,
};
const PADDING: Record<ButtonSize, { base: string; left: string; right: string; both: string }> = {
  small: { base: 'px-3', left: 'pl-2.5 pr-3.5', right: 'pl-3.5 pr-2.5', both: 'px-2.5' },
  medium: { base: 'px-4', left: 'pl-3 pr-5', right: 'pl-5 pr-3', both: 'px-3' },
  large: { base: 'px-4 xl:px-5', left: 'pl-3.5 pr-5 xl:pl-4 xl:pr-6', right: 'pl-5 pr-3.5 xl:pl-6 xl:pr-4', both: 'px-3.5 xl:px-4' },
};
/**
 * 强调钮的 1px 外环画在盒子外面，看得见的外沿比同档白钮（发丝画在里面）高出 2px。
 * 盒子上下各收 1px、再用 1px 上下外边距还回占位：外沿与白钮齐平，排版不动（左右不收：w-full 的钮不该溢出容器）
 */
const EMPHASIS_FIT: Record<ButtonSize, string> = { small: 'h-[26px] my-px', medium: 'h-[30px] my-px', large: 'h-[34px] xl:h-[38px] my-px' };
export const ICON_PX: Record<ButtonSize, number> = { small: 14, medium: 16, large: 16 };
/** 图标与文字的间距：单独成表，别从 SIZE 字符串里按位置取——large 多了 xl:h-10 一项就会取错 */
const GAP: Record<ButtonSize, string> = { small: 'gap-1.5', medium: 'gap-2', large: 'gap-2' };

// ============ 变体（Laper VARIANT_STYLES 原样） ============
// 强调钮 = Laper / Kumo 配方逐字：底色 token 混 30% 白、1px ring 画在盒子**外面**（token 混 10% 黑）、shadow-xs，
// 立体感由 EmphasisEffect 那层自上而下的渐变 + 顶部 1px 内高光给。
// 不用「纯色块 + ring-inset」：inset 的描线压在底色上，在 squircle 拐角被抗锯齿吃掉，
// 边缘发虚、没有那一线深边，和 Laper 并排一眼就不一样。外环是 box-shadow 不占布局，并排照样齐平（与 Surface 同一副边）
const EMPHASIS = 'group relative overflow-hidden bg-(--btn-emphasis-bg) ring ring-(--btn-emphasis-ring) shadow-xs border-0';
export const VARIANT_STYLES: Record<ButtonVariant, string> = {
  primary: `${EMPHASIS} text-primary-foreground`,
  /** 墨色强调钮：一整块前景色（ChatGPT「升级至 Pro」那种黑），与 primary 同一套受光与悬停 */
  ink: `${EMPHASIS} text-background`,
  info: `${EMPHASIS} text-info-foreground`,
  warning: `${EMPHASIS} text-warning-foreground`,
  danger: `${EMPHASIS} text-destructive-foreground`,
  success: `${EMPHASIS} text-success-foreground`,
  secondary: 'cromma-tag border-0 relative overflow-hidden', // 白钮 = Tag 同一张皮（theme.css 的 .cromma-tag：发丝内边 + 内高光 + 极淡外投影，悬停叠淡墨），不自画 border / shadow
  ghost: 'bg-transparent shadow-none! border-0 text-foreground hover:bg-accent',
  outline: 'bg-transparent text-foreground border border-border hover:border-primary hover:text-primary shadow-none!',
  selected: 'bg-sidebar-accent text-foreground border-0 shadow-none!',
  icon: 'bg-transparent shadow-none! p-0 w-7 h-7 text-foreground hover:text-primary',
  dashed: 'bg-transparent text-muted-foreground border border-dashed border-border shadow-none! hover:border-primary/50 hover:text-foreground disabled:hover:border-border disabled:hover:text-muted-foreground',
  /** 灰底钮：白卡输入面里的次级圆（对话输入胶囊的「+」与麦克风）——白卡上的白钮看不出来，退一级明度成 muted 底，无边无影 */
  muted: 'bg-muted text-foreground border-0 shadow-none! hover:bg-accent',
  /** 白色控件：只坐在媒体查看器的黑玻璃上（docs/59），静止只有白字形、悬停铺 control、按下铺 control-hover；色取 media-* 工具色（日夜同值），不随明暗翻面 */
  media: 'bg-transparent text-media-foreground border-0 shadow-none! rounded-full hover:bg-media-control active:bg-media-control-hover focus-visible:ring-media-foreground/60',
};
/** media 钮的几何（像 icon 一样不吃 SIZE 表）：圆钮边长 --media-bar-button；字样钮（倍速、重试）同高、最窄同宽、左右内距 --media-speed-pad-h、13 号半粗 */
const MEDIA_GEOMETRY = { round: 'size-(--media-bar-button) p-0', text: 'h-(--media-bar-button) min-w-(--media-bar-button) px-(--media-speed-pad-h) text-base font-semibold' };

// ============ 强调钮的颜色（Laper constants.emphasisStyle 逐字：全走 token，oklch 里混纯白 / 纯黑） ============
//   ring = token 混 10% 黑（比底色深一线的外边）· bg = token 混 30% 白（底色，也是顶部 1px 内高光、悬停时渐变的起点）
//   from → to = token 混 15% 白 → token（EmphasisEffect 自上而下的受光渐变）
// lift 是「混多少白」：有色钮照 Laper 30 / 15；墨钮（Laper 没有这一枚）只抬 12 / 6——黑混 30% 白就成了灰，不再是那块黑
const EMPHASIS_TOKENS: Partial<Record<ButtonVariant, { token: string; lift: [number, number] }>> = {
  primary: { token: 'var(--primary)', lift: [30, 15] },
  ink: { token: 'var(--foreground)', lift: [12, 6] },
  info: { token: 'var(--info)', lift: [30, 15] },
  warning: { token: 'var(--warning)', lift: [30, 15] },
  danger: { token: 'var(--destructive)', lift: [30, 15] },
  success: { token: 'var(--success)', lift: [30, 15] },
};
export function emphasisStyle(variant: ButtonVariant): CSSProperties | undefined {
  const e = EMPHASIS_TOKENS[variant];
  if (!e) return undefined;
  const [bg, from] = e.lift;
  return {
    '--btn-emphasis-ring': `color-mix(in oklch, ${e.token}, black 10%)`,
    '--btn-emphasis-bg': `color-mix(in oklch, ${e.token}, white ${bg}%)`,
    '--btn-emphasis-from': `color-mix(in oklch, ${e.token}, white ${from}%)`,
    '--btn-emphasis-to': e.token,
  } as CSSProperties;
}

/** 强调钮的受光层（Laper EmphasisEffect 逐字）：自上而下渐变 + 顶部 1px 内高光，悬停时渐变起点抬到底色（整体提亮）；铺在内容层之下 */
function EmphasisEffect() {
  return <span aria-hidden="true" className="pointer-events-none absolute inset-0 rounded-[inherit] bg-linear-to-b from-(--btn-emphasis-from) to-(--btn-emphasis-to) shadow-[inset_0_1px_0_0_var(--btn-emphasis-bg)] transition-colors group-hover:from-(--btn-emphasis-bg)" />;
}

export interface ButtonProps extends Omit<ButtonHTMLAttributes<HTMLButtonElement>, 'children'> {
  href?: string; // 给了就渲染 <a>（官网的下载 / 锚点），质感与 <button> 完全一致
  target?: string;
  rel?: string;
  variant?: ButtonVariant;
  size?: ButtonSize;
  leftIcon?: ReactNode;
  rightIcon?: ReactNode;
  loading?: boolean;
  asChild?: boolean;
  /** 正圆的图标钮（发送、附件）：直径 = 这一档的高度。别用 className 写 h-* / w-* 去凑——那会绕开强调钮的收边 */
  round?: boolean;
  children?: ReactNode;
}

export function Button({
  variant = 'primary',
  size = CONTROL_DEFAULT_SIZE,
  leftIcon,
  rightIcon,
  loading = false,
  disabled = false,
  asChild = false,
  round = false,
  href,
  target,
  rel,
  className,
  style,
  onClick,
  type = 'button',
  children,
  ...rest
}: ButtonProps) {
  const [shaking, setShaking] = useState(false);
  const isEmphasis = variant in EMPHASIS_TOKENS;
  const isDisabled = disabled || loading;
  const iconOnly = !children && (leftIcon || rightIcon);
  const pad = PADDING[size];
  const padding = iconOnly || (leftIcon && rightIcon) ? pad.both : leftIcon ? pad.left : rightIcon ? pad.right : pad.base;

  const handleClick = useCallback(
    (e: React.MouseEvent<HTMLButtonElement>) => {
      if (isDisabled) {
        // 禁用态点击：摇晃提示而非死寂（Laper）
        e.preventDefault();
        if (disabled && !loading) {
          setShaking(true);
          setTimeout(() => setShaking(false), 500);
        }
        return;
      }
      onClick?.(e);
    },
    [isDisabled, disabled, loading, onClick],
  );

  const classes = cn(
    'isolate inline-flex shrink-0 items-center justify-center whitespace-nowrap transition-[background-color,box-shadow,transform,color,border-color]',
    CONTROL_RADIUS,
    'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-primary/40',
    isDisabled ? 'cursor-not-allowed' : 'cursor-pointer',
    variant !== 'icon' && variant !== 'media' && SIZE[size],
    isEmphasis && EMPHASIS_FIT[size],
    variant === 'media' ? MEDIA_GEOMETRY[round || iconOnly ? 'round' : 'text'] : variant !== 'icon' && (round ? 'aspect-square rounded-full p-0' : padding),
    VARIANT_STYLES[variant],
    // 强调钮：悬停 1.01 / 按压 0.98（theme.css 的 --scale-hover / --scale-press；Laper 是 1.02，这里减半）
    isEmphasis && !isDisabled && 'hover:scale-hover active:scale-press',
    isDisabled && 'opacity-60',
    shaking && 'animate-shake',
    className,
  );
  const mergedStyle = { ...(isEmphasis ? emphasisStyle(variant) : undefined), ...style };
  const iconPx = ICON_PX[size];
  const content = (
    <span className={cn('relative z-2 flex items-center justify-center', variant !== 'icon' && GAP[size])}>
      {loading ? (
        <span className="flex shrink-0 items-center justify-center" aria-hidden="true">
          <svg width={iconPx} height={iconPx} viewBox="0 0 24 24" className="animate-spin" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round">
            <path d="M12 3a9 9 0 1 0 9 9" />
          </svg>
        </span>
      ) : leftIcon ? (
        <span className="flex shrink-0 items-center justify-center">{leftIcon}</span>
      ) : null}
      {/* children 里常常直接放「图标 + 文字」：用 flex 行承接，否则块级 svg 会把文字挤到下一行 */}
      {children !== null && children !== undefined && children !== false && children !== '' && (
        <span className={cn('inline-flex items-center whitespace-nowrap', variant !== 'icon' && GAP[size], loading && 'opacity-50')}>{children}</span>
      )}
      {!loading && rightIcon && <span className="flex shrink-0 items-center justify-center">{rightIcon}</span>}
    </span>
  );

  if (asChild) {
    return (
      <SlotPrimitive.Slot className={classes} style={mergedStyle} {...(rest as Record<string, unknown>)}>
        {children}
      </SlotPrimitive.Slot>
    );
  }
  const inner = (
    <>
      {isEmphasis && <EmphasisEffect />}
      {loading && <span aria-hidden="true" className="laper-shimmer-band z-1" />}
      {content}
    </>
  );
  if (href) {
    return (
      <a href={href} target={target} rel={rel} className={classes} style={mergedStyle} aria-disabled={isDisabled || undefined} onClick={isDisabled ? (e) => e.preventDefault() : undefined}>
        {inner}
      </a>
    );
  }
  return (
    <button type={type} className={classes} style={mergedStyle} aria-disabled={isDisabled || undefined} onClick={handleClick} {...rest}>
      {inner}
    </button>
  );
}
