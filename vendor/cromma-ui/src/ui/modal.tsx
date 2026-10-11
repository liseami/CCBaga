/**
 * [INPUT]: 依赖 react-dom 的 createPortal，motion 的 MOTION、motion、AnimatePresence（进出：遮罩淡、弹层 0.9 ↔ 1 缩放叠淡，quick 档），panel 的 Panel、PanelHeader、PanelAction，portal-owner。
 * [OUTPUT]: 提供 modalStore、openModal、closeModal、closeAllModals、ModalProvider、ModalShell 及类型；closeLabel 允许消费方本地化关闭按钮。
 * [POS]: cromma-ui 的居中模态。Laper ModalProvider 同款。外部 store 让开关零重渲染。ESC、遮罩关顶层、焦点陷阱、zIndex 叠放。自带质感的卡直接交给 openModal。ModalShell 的背景和页头由 Panel、PanelHeader 提供，与壳面板共用。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { memo, useCallback, useEffect, useRef, useSyncExternalStore, type CSSProperties, type ReactNode } from 'react';
import { createPortal } from 'react-dom';
import { cn } from '../cn';
import { AnimatePresence, MOTION, motion } from '../motion';
import { Panel, PanelHeader, PanelAction } from './panel';
import { createPortalOwnership } from './portal-owner';

// ═══════════════════════════════════════════════════════════════════════
//  外部 store · 完全脱离 React 渲染周期
// ═══════════════════════════════════════════════════════════════════════

export interface ModalOptions {
  /** 点遮罩 / ESC 可关（默认 true） */
  maskClosable?: boolean;
  onClose?: (() => void) | null;
}

export type ModalContent = ReactNode | ((context: { close: () => void }) => ReactNode);

export interface ModalState {
  id: string;
  content: ModalContent;
  zIndex: number;
  options: { maskClosable: boolean; onClose: (() => void) | null };
}

type Listener = () => void;

const BASE_Z = 1000;

function createModalStore() {
  let modals = new Map<string, ModalState>();
  const listeners = new Set<Listener>();
  const notify = () => listeners.forEach(l => l());

  const closeModal = (id?: string) => {
    if (!id) {
      const keys = [...modals.keys()];
      if (keys.length === 0) return;
      id = keys[keys.length - 1];
    }
    const modal = modals.get(id!);
    if (!modal) return;
    modal.options.onClose?.();
    // 立刻出 store：退场动画由 ModalPortal 的 AnimatePresence 放完再卸，store 不记「关闭中」
    const next = new Map(modals);
    next.delete(id!);
    modals = next;
    notify();
  };

  const openModal = (content: ModalContent, options: ModalOptions = {}): (() => void) => {
    const id = `modal_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`;
    modals = new Map(modals).set(id, {
      id,
      content,
      zIndex: BASE_Z + modals.size * 10,
      options: { maskClosable: options.maskClosable ?? true, onClose: options.onClose ?? null },
    });
    notify();
    return () => closeModal(id);
  };

  const closeAllModals = () => [...modals.keys()].forEach(id => closeModal(id));

  return {
    subscribe: (l: Listener) => {
      listeners.add(l);
      return () => {
        listeners.delete(l);
      };
    },
    getSnapshot: () => modals,
    openModal,
    closeModal,
    closeAllModals,
  };
}

export const modalStore = createModalStore();
export const { openModal, closeModal, closeAllModals } = modalStore;

// ═══════════════════════════════════════════════════════════════════════
//  ModalProvider · 零状态：children 永不因 modal 状态变化而重渲染
// ═══════════════════════════════════════════════════════════════════════

/** 同一页只该有一个 Portal，且由最后挂上的 Provider 画（模态是壳发起的、要读壳的上下文；登录门那棵树先挂、只透传） */
const useOwnsPortal = createPortalOwnership();

export const ModalProvider = memo(function ModalProvider({ children }: { children: ReactNode }) {
  const owns = useOwnsPortal();
  return (
    <>
      {children}
      {owns && <ModalPortal />}
    </>
  );
});

function ModalPortal() {
  const modals = useSyncExternalStore(modalStore.subscribe, modalStore.getSnapshot, modalStore.getSnapshot);
  const list = [...modals.values()];
  if (typeof document === 'undefined') return null;
  const top = list[list.length - 1]?.id;
  return createPortal(
    <AnimatePresence>
      {list.map(m => (
        <ModalItem key={m.id} modal={m} isTop={m.id === top} />
      ))}
    </AnimatePresence>,
    document.body,
  );
}

const POP_FROM = { opacity: 0, scale: 0.9 };
const POP_TO = { opacity: 1, scale: 1 };

const FOCUSABLE = 'a[href], button:not([disabled]), textarea:not([disabled]), input:not([disabled]):not([type="hidden"]), select:not([disabled]), [tabindex]:not([tabindex="-1"])';

function ModalItem({ modal, isTop }: { modal: ModalState; isTop: boolean }) {
  const ref = useRef<HTMLDivElement>(null);
  const close = useCallback(() => modalStore.closeModal(modal.id), [modal.id]);

  // ESC 关闭（仅顶层且 maskClosable）
  useEffect(() => {
    if (!isTop || !modal.options.maskClosable) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        e.preventDefault();
        e.stopPropagation();
        close();
      }
    };
    document.addEventListener('keydown', onKey, true);
    return () => document.removeEventListener('keydown', onKey, true);
  }, [isTop, modal.options.maskClosable, close]);

  // 焦点陷阱 + 初始聚焦 + 关闭还焦（仅顶层）
  useEffect(() => {
    if (!isTop) return;
    const el = ref.current;
    if (!el) return;
    const previous = document.activeElement as HTMLElement | null;
    const focusable = () =>
      [...el.querySelectorAll<HTMLElement>(FOCUSABLE)].filter(n => {
        const st = getComputedStyle(n);
        return st.display !== 'none' && st.visibility !== 'hidden';
      });
    requestAnimationFrame(() => {
      const list = focusable();
      (list[0] ?? el).focus();
    });
    const onTab = (e: KeyboardEvent) => {
      if (e.key !== 'Tab') return;
      const list = focusable();
      if (list.length === 0) return void e.preventDefault();
      const first = list[0]!;
      const last = list[list.length - 1]!;
      if (e.shiftKey && document.activeElement === first) {
        e.preventDefault();
        last.focus();
      } else if (!e.shiftKey && document.activeElement === last) {
        e.preventDefault();
        first.focus();
      }
    };
    el.addEventListener('keydown', onTab);
    return () => {
      el.removeEventListener('keydown', onTab);
      if (previous && document.activeElement && el.contains(document.activeElement)) previous.focus();
    };
  }, [isTop]);

  const content = typeof modal.content === 'function' ? modal.content({ close }) : modal.content;
  const wrapperStyle: CSSProperties = { zIndex: modal.zIndex, cursor: modal.options.maskClosable ? 'pointer' : 'default' };

  return (
    // 退场中整层不接点按（exit 一开始就落下 pointer-events）
    <motion.div
      className="fixed inset-0 flex items-center justify-center p-4"
      style={wrapperStyle}
      exit={{ pointerEvents: 'none' }}
      onClick={e => {
        if (e.target === e.currentTarget && isTop && modal.options.maskClosable) close();
      }}
    >
      {/* 遮罩：页底色 85%，无模糊（Laper 居中模态同款） */}
      <motion.div aria-hidden="true" className="pointer-events-none absolute inset-0 bg-background/85" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={MOTION.quick} />
      {/* 弹层：绝对居中，scale .9↔1 叠 opacity，进出同一组参数。absolute 会 shrink-wrap：自定义卡必须自带明确宽度，w-full max-w-* 会塌成内容宽 */}
      <motion.div
        ref={ref}
        role="dialog"
        aria-modal="true"
        tabIndex={-1}
        className="absolute top-1/2 left-1/2 cursor-default outline-none"
        style={{ x: '-50%', y: '-50%' }}
        initial={POP_FROM}
        animate={POP_TO}
        exit={POP_FROM}
        transition={MOTION.quick}
        onClick={e => e.stopPropagation()}
      >
        {content}
      </motion.div>
    </motion.div>
  );
}

// ═══════════════════════════════════════════════════════════════════════
//  ModalShell · 统一外壳（素卡）
// ═══════════════════════════════════════════════════════════════════════

function CloseIcon() {
  return (
    <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" aria-hidden="true">
      <path d="M18 6 6 18M6 6l12 12" />
    </svg>
  );
}

export function ModalShell({
  title,
  description,
  children,
  onClose,
  footer,
  hideClose = false,
  closeLabel = "Close",
  width = 500,
  className,
  contentClassName,
}: {
  title?: ReactNode;
  description?: ReactNode;
  children: ReactNode;
  onClose?: () => void;
  footer?: ReactNode;
  hideClose?: boolean;
  closeLabel?: string;
  /** px · 上限 90vw，高上限 90vh，正文自动滚动 */
  width?: number;
  className?: string;
  contentClassName?: string;
}) {
  const showClose = !hideClose && Boolean(onClose);
  return (
    <Panel className={cn('max-h-dialog max-w-dialog', className)} style={{ width }}>
      {(title || showClose) && (
        <PanelHeader title={title} actions={showClose ? <PanelAction label={closeLabel} icon={<CloseIcon />} onClick={onClose} /> : undefined} />
      )}
      <div className={cn('min-h-0 flex-1 overflow-y-auto px-6 py-4', contentClassName)}>
        {description && <div className="mb-3 text-sm text-muted-foreground">{description}</div>}
        {children}
      </div>
      {footer && <div className="shrink-0 border-t border-border px-6 py-4">{footer}</div>}
    </Panel>
  );
}
