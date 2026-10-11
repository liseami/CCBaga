/**
 * [INPUT]: 依赖 react 的 useEffect、useState、useSyncExternalStore。
 * [OUTPUT]: 提供 createPortalOwnership。它返回 useOwner。
 * [POS]: cromma-ui 的 Provider 单例规则。同类 Provider（Modal、Toast）同页多挂时只有最后挂上的画 Portal。其余透传。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { useEffect, useState, useSyncExternalStore } from 'react';

export function createPortalOwnership(): () => boolean {
  const stack: number[] = [];
  const listeners = new Set<() => void>();
  let seq = 0;
  // 后来者拥有：web 上登录门先挂、壳后挂，模态内容要读壳的上下文（I18n），必须由壳那棵树来画
  const top = () => stack[stack.length - 1];
  const emit = () => listeners.forEach((l) => l());
  const subscribe = (l: () => void) => {
    listeners.add(l);
    return () => listeners.delete(l);
  };
  return function useOwner(): boolean {
    const [id] = useState(() => ++seq);
    useEffect(() => {
      stack.push(id);
      emit();
      return () => {
        const i = stack.indexOf(id);
        if (i >= 0) stack.splice(i, 1);
        emit();
      };
    }, [id]);
    return useSyncExternalStore(subscribe, () => top() === id, () => false);
  };
}
