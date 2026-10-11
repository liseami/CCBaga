/**
 * [INPUT]: 依赖 react 的 useMemo，.cn；皮 .cromma-skeleton 在同位的 loader.css，关键帧是 theme.css 的全站 laper-shimmer-sweep。
 * [OUTPUT]: 提供 Loader、Skeleton、SkeletonLine、SkeletonText、useSkeletonRhythm。Loader是圆转。SkeletonLine是随机宽度的文字行。useSkeletonRhythm是扫光节奏，Avatar 的加载位同用。
 * [POS]: cromma-ui的等待原语：列表等接口一律铺骨架（镜像真实布局，落地不跳），Loader 只给尺寸固定的盒子。骨架必须用组件，裸 class 拿不到随机节奏。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useMemo, type CSSProperties, type HTMLAttributes } from 'react';
import { cn } from '../cn';
import './loader.css';

export type LoaderSize = 'small' | 'medium' | 'large';

const SIZE: Record<LoaderSize, string> = {
  small: 'size-4',
  medium: 'size-6',
  large: 'size-8',
};

export function Loader({ size = 'medium', className, 'aria-label': ariaLabel = 'Loading', ...rest }: { size?: LoaderSize; className?: string } & HTMLAttributes<SVGSVGElement>) {
  return (
    <svg viewBox="0 0 24 24" fill="none" role="status" aria-label={ariaLabel} className={cn('animate-spin text-muted-foreground', SIZE[size], className)} {...rest}>
      <circle cx="12" cy="12" r="9" stroke="currentColor" strokeOpacity="0.2" strokeWidth="3" />
      <path d="M21 12a9 9 0 0 0-9-9" stroke="currentColor" strokeWidth="3" strokeLinecap="round" />
    </svg>
  );
}

/** 时长与宽度的随机区间：两档都不同步，整片骨架才像在呼吸而不是机械闪 */
const DURATION = [1.3, 1.7] as const;
const WIDTH = [30, 100] as const;

const between = (min: number, max: number) => Math.random() * (max - min) + min;

/**
 * 每个实例挂载时抽一次，渲染之间稳定。延迟必须是负的：正延迟期间 ::after 还没进关键帧，
 * 高光停在 translateX(0) 压在骨架中央等发令枪，几十条各等各的就是一片乱闪；负延迟让每条挂载即处在扫光中途。
 * 扫光只动 transform（theme.css 的 laper-shimmer-sweep），跑在合成器线程，主线程被大列表堵住也照常流动。
 */
export function useSkeletonRhythm(min = DURATION[0], max = DURATION[1]): CSSProperties {
  return useMemo(() => {
    const duration = between(min, max);
    return {
      ['--skeleton-duration' as string]: `${duration.toFixed(2)}s`,
      ['--skeleton-delay' as string]: `${(-Math.random() * duration).toFixed(2)}s`,
    };
  }, [min, max]);
}

/** 骨架块：尺寸与圆角由调用方给（cn 让传进来的 rounded-* 覆盖这里的缺省） */
export function Skeleton({ className, circle = false, style, ...rest }: { circle?: boolean } & HTMLAttributes<HTMLDivElement>) {
  const rhythm = useSkeletonRhythm();
  return <div aria-hidden="true" style={{ ...rhythm, ...style }} className={cn('cromma-skeleton rounded-lg bg-muted', circle && 'rounded-full', className)} {...rest} />;
}

/**
 * 文字行：宽度在 [minWidth, maxWidth]% 之间随机——真实的一行文字不会齐头齐尾，齐了就像色块不像字。
 * ⚠️ 宽度是**内联样式**，会盖掉 className 里的 w-*：要定宽就用 Skeleton，别用这个。
 * blockHeight 给行高槽（比如 1.5rem 的一行正文），骨架条在槽里垂直居中，段落行距因此与真文字一致。
 */
export function SkeletonLine({
  minWidth = WIDTH[0],
  maxWidth = WIDTH[1],
  blockHeight,
  className,
  style,
}: {
  minWidth?: number;
  maxWidth?: number;
  blockHeight?: number | string;
  className?: string;
  style?: CSSProperties;
}) {
  const rhythm = useSkeletonRhythm();
  const width = useMemo(() => `${Math.round(between(minWidth, maxWidth))}%`, [minWidth, maxWidth]);
  const line = <div aria-hidden="true" style={{ ...rhythm, width, ...style }} className={cn('cromma-skeleton h-2.5 rounded-sm bg-muted', className)} />;
  if (blockHeight === undefined) return line;
  return (
    <div className="flex items-center" style={{ height: typeof blockHeight === 'number' ? `${blockHeight}px` : blockHeight }}>
      {line}
    </div>
  );
}

/** 多行段落：每行宽度各自随机，末行明显更短——段落的最后一行本来就不满行 */
export function SkeletonText({ lines = 3, className, lineClassName }: { lines?: number; className?: string; lineClassName?: string }) {
  return (
    <div className={cn('flex flex-col gap-2', className)}>
      {Array.from({ length: lines }).map((_, i) => (
        <SkeletonLine key={i} minWidth={i === lines - 1 ? 35 : 70} maxWidth={i === lines - 1 ? 65 : 100} className={lineClassName} />
      ))}
    </div>
  );
}
