/**
 * [INPUT]: 依赖 clsx 与 tailwind-merge 的 extendTailwindMerge。
 * [OUTPUT]: 提供 cn(...)。cn(...)是条件类名合并并去重冲突（shadcn 约定）。
 * [POS]: cromma-ui的 className 工具，壳与所有 Cromma App的组件拼接统一走这里。认得 tokens.css 自定的圆角名，rounded-panel 与 rounded-2xl 才会互相覆盖。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { clsx, type ClassValue } from 'clsx';
import { extendTailwindMerge } from 'tailwind-merge';

// tailwind-merge 只认 T 恤尺码的圆角；panel（Laper 面板的角）与 capsule 是 tokens.css 自定的名字
const twMerge = extendTailwindMerge({ extend: { theme: { radius: ['panel', 'capsule'] } } });

export function cn(...inputs: ClassValue[]): string {
  return twMerge(clsx(inputs));
}
