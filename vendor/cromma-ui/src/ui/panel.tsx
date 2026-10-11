/**
 * [INPUT]: 依赖 react 的 forwardRef，.cn，button 的 Button，tooltip 的 Tip，glyphs 的 SearchGlyph，../shell-layout-tokens 的 SHELL_LAYOUT.divider；几何取 tokens.css 的 --panel-*（tokens.yaml panel 段，Laper 面板的数）与 rounded-panel。
 * [OUTPUT]: 提供 Panel、PanelRule、PanelHeader、PanelAction、SearchPanelHeader 与类型 SearchPanelFilter。Panel 是面板的面，有三形：缺省是卡，flat 是 frame 里的一栏，frame 是壳的整张 panel。PanelRule 是栏间的发丝竖线。PanelHeader 是 leading、title、subtitle、actions、divider。PanelAction 是页头右端的 web 圆钮（secondary 白钮，可换 muted 灰底）。SearchPanelHeader 是特制搜索页的页头。
 * [POS]: cromma-ui 的面板壳。一切「面板样」的东西的底与页头只从这里来。与桌面 components/panel.rs 的 Panel 同名同义。壳是 Linear 式：侧栏右侧一张 frame（平滑圆角、1px border 色发丝边、panel 底、无影）；列表栏、dashboard、并排资料栏、设置页与朋友圈定宽列是 frame 里的栏（flat）；栏间是 PanelRule。不变量：栏不自铺底；底与圆角只由 frame 一处画；frame 以同一圆角裁切内容，栏里任何子元素的方底不露出圆角外。卡 = Laper SoftPanel（panel 底、1px panel-outline 边、shadow-sm、24 squircle），给覆盖的资料栏、应用卡与模态。PanelHeader = Laper PanelHeader（48 高含分隔线、标题在左、字号读 panel.headerTitleSize）。页头右端的钮由调用方给，图标钮一律 PanelAction（Button secondary small round），字钮是 Button secondary small。SearchPanelHeader 与 PanelHeader 同一行几何（HEADER_ROW：高、内距、下沿发丝）：那一行就是搜索框，左放大镜落在标题的内缩线上，无边输入占满剩余宽，右端仍是 actions；其下可挂一排筛选胶囊（Button small，选中 selected 灰底，未选中 secondary 白底描边）。特制搜索页一律用它，不在普通页头上魔改。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { forwardRef, type ComponentProps, type HTMLAttributes, type KeyboardEvent, type ReactNode } from 'react';
import { cn } from '../cn';
import { SHELL_LAYOUT } from '../shell-layout-tokens';
import { Button } from './button';
import { SearchGlyph } from './glyphs';
import { Tip } from './tooltip';

type PanelProps = HTMLAttributes<HTMLDivElement> & {
  /** 栏：frame 里的一栏，不铺底、无边、无影、无圆角——底与圆角只由 frame 画，栏若自铺方底会从圆角外露出；栏间的线是 PanelRule */
  flat?: boolean;
  /** 壳的整张 panel（Linear 式）：rounded-panel 平滑圆角、1px border 色发丝边、panel 底、无影；overflow-hidden 按同一圆角裁切栏与栏里的一切 */
  frame?: boolean;
};

/**
 * 面板的面：底、边、影与圆角只在这一处；内容自己排（页头 + 可滚的身体），溢出裁在面里；面里的软边（ChatEdge）承面板底色。
 * frame 的裁切：border-radius 加 corner-shape: squircle（theme.css 全局给 rounded-*）下，overflow: hidden 的裁切框是 padding 盒的内圆角，
 * 与边同一条超椭圆；不支持 corner-shape 时 --radius-panel 退成 --panel-radius-fallback 的正圆角，裁切随之。边是 frame 自己的 border，子元素盖不住它。
 * position: fixed 的后代（覆盖的资料栏、模态）以视口为包含块，不被裁——frame 因此不加 contain / transform。
 */
export const Panel = forwardRef<HTMLDivElement, PanelProps>(function Panel({ flat = false, frame = false, className, ...rest }, ref) {
  return (
    <div
      ref={ref}
      className={cn(
        'relative flex min-h-0 flex-col overflow-hidden text-panel-foreground [--edge-tone:var(--panel)]',
        frame ? 'rounded-panel border-(length:--panel-border) border-border bg-panel' : !flat && 'rounded-panel border-(length:--panel-border) border-panel-outline bg-panel shadow-sm',
        className,
      )}
      {...rest}
    />
  );
});

/** 栏间的发丝竖线：shellLayout.divider 宽、border 色、撑满高（壳的拖宽把手 Gutter 同一道线） */
export function PanelRule({ className }: { className?: string }) {
  return <div aria-hidden="true" className={cn('h-full shrink-0 bg-border', className)} style={{ width: SHELL_LAYOUT.divider }} />;
}

type HeaderProps = {
  /** 标题前的钮（资料栏的 ✕ / ←、收起时的 PanelLead）：与右端同一颗圆钮，外沿离卡边 actionInset */
  leading?: ReactNode;
  /** 标题：恒在左，一行截断 */
  title?: ReactNode;
  /** 标题下一行灰字（可无） */
  subtitle?: ReactNode;
  /** 右端的钮：图标钮用 PanelAction，字钮用 Button secondary small；钮距 actionGap */
  actions?: ReactNode;
  /** 页头下的分隔线（Laper 缺省有）；页头下紧接自带边界的带（搜索栏、分段）时可关 */
  divider?: boolean;
  className?: string;
};

/**
 * 页头：高 --panel-header-height（含分隔线）。左右外沿取 actionInset（钮的外沿落在这里），标题再内缩到 headerPadH——
 * 没有 leading 时标题离卡边 headerPadH，钮的图标与标题同一条内缩线（Laper 的 px-4 + -mr-1.5）
 */
/** 页头那一行：高（含分隔线）、左右外沿、下沿发丝——PanelHeader 与 SearchPanelHeader 同读 */
const HEADER_ROW = 'flex h-(--panel-header-height) shrink-0 items-center px-(--panel-action-inset)';
const HEADER_DIVIDER = 'border-b-(length:--panel-header-divider) border-panel-outline';
/** 标题（与搜索页头的放大镜）再内缩到 headerPadH */
const HEADER_TEXT_INSET = 'px-[calc(var(--panel-header-pad-h)_-_var(--panel-action-inset))]';

export function PanelHeader({ leading, title, subtitle, actions, divider = true, className }: HeaderProps) {
  return (
    <header className={cn(HEADER_ROW, divider && HEADER_DIVIDER, className)}>
      {leading ? <div className="flex shrink-0 items-center gap-(--panel-action-gap)">{leading}</div> : null}
      <div className={cn('flex min-w-0 flex-1 flex-col justify-center gap-0.5', HEADER_TEXT_INSET)}>
        {title ? <h2 className="truncate text-(length:--panel-header-title-size) leading-none tracking-normal [font-weight:var(--panel-header-title-weight)] text-panel-foreground">{title}</h2> : null}
        {subtitle ? <p className="truncate text-sm leading-none text-muted-foreground">{subtitle}</p> : null}
      </div>
      {actions ? <div className="flex shrink-0 items-center gap-(--panel-action-gap)">{actions}</div> : null}
    </header>
  );
}

type ActionProps = {
  /** 提示与无障碍名 */
  label: string;
  /** 皮：secondary 白钮（缺省）；muted 灰底钮（侧栏顶部的搜索钮） */
  variant?: 'secondary' | 'muted';
  /** 图标（尺寸由这里钉成 --panel-action-icon） */
  icon: ReactNode;
  /** false：不包 Tip，交给调用方自己包（菜单触发器要先包 Tip 再包 Trigger） */
  tip?: boolean;
} & Omit<ComponentProps<typeof Button>, 'variant' | 'size' | 'round' | 'children'>;

/** 页头的图标钮：web 圆钮 small（28 外沿、16 图标） */
export function PanelAction({ label, icon, tip = true, variant = 'secondary', className, ...rest }: ActionProps) {
  const button = (
    <Button variant={variant} size="small" round aria-label={label} className={cn('[&_svg]:size-(--panel-action-icon)', className)} {...rest}>
      {icon}
    </Button>
  );
  return tip ? <Tip label={label}>{button}</Tip> : button;
}

export interface SearchPanelFilter {
  value: string;
  label: ReactNode;
}

type SearchHeaderProps = {
  value: string;
  onValueChange: (value: string) => void;
  /** 占位字（原语不写字面量） */
  placeholder: string;
  /** 输入的无障碍名 */
  label: string;
  autoFocus?: boolean;
  /** 回车等按键（Esc 有字时先清字，再交给外面） */
  onKeyDown?: (e: KeyboardEvent<HTMLInputElement>) => void;
  /** 右端的钮：同 PanelHeader，图标钮用 PanelAction */
  actions?: ReactNode;
  /** 页头下一排筛选胶囊；不给不画 */
  filters?: readonly SearchPanelFilter[];
  filter?: string;
  onFilterChange?: (value: string) => void;
  /** 筛选胶囊一排的无障碍名 */
  filtersLabel?: string;
};

/**
 * 特制搜索页的页头（Linear 式）：页头那一行就是搜索框——左放大镜 + 占满剩余宽的无边输入，右端 actions；下沿同一道发丝。
 * 其下一排筛选胶囊：Button small，选中 selected（灰底），未选中 secondary（白底描边），左外沿与页头钮同一条线。
 */
export const SearchPanelHeader = forwardRef<HTMLInputElement, SearchHeaderProps>(function SearchPanelHeader(
  { value, onValueChange, placeholder, label, autoFocus, onKeyDown, actions, filters, filter, onFilterChange, filtersLabel },
  ref,
) {
  return (
    <div className="flex shrink-0 flex-col">
      <header className={cn(HEADER_ROW, HEADER_DIVIDER)}>
        <label className={cn('flex h-full min-w-0 flex-1 cursor-text items-center gap-2 text-muted-foreground', HEADER_TEXT_INSET)}>
          <SearchGlyph className="size-(--panel-action-icon) shrink-0" />
          <input
            ref={ref}
            type="search"
            value={value}
            autoFocus={autoFocus}
            placeholder={placeholder}
            aria-label={label}
            onChange={(e) => onValueChange(e.target.value)}
            onKeyDown={(e) => {
              onKeyDown?.(e);
              // Esc 先清字，再交给外面
              if (!e.defaultPrevented && e.key === 'Escape' && value) {
                e.preventDefault();
                onValueChange('');
              }
            }}
            className="h-full min-w-0 flex-1 bg-transparent text-(length:--panel-header-title-size) text-panel-foreground outline-none placeholder:text-muted-foreground [&::-webkit-search-cancel-button]:hidden"
          />
        </label>
        {actions ? <div className="flex shrink-0 items-center gap-(--panel-action-gap)">{actions}</div> : null}
      </header>
      {filters?.length ? (
        <div role="toolbar" aria-label={filtersLabel} className="flex shrink-0 gap-2 overflow-x-auto px-(--panel-action-inset) pt-(--panel-list-top) scrollbar-hide">
          {filters.map((f) => {
            const on = f.value === filter;
            return (
              <Button key={f.value} variant={on ? 'selected' : 'secondary'} size="small" aria-pressed={on} onClick={() => onFilterChange?.(f.value)}>
                {f.label}
              </Button>
            );
          })}
        </div>
      ) : null}
    </div>
  );
});
