/**
 * [INPUT]: Vite YAML 构建插件。
 * [OUTPUT]: YAML 平面文案目录的 TypeScript 类型。
 * [POS]: 编译边界；解析器仅用于构建，不进入应用运行时。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
declare module '*.yaml' { const catalog:Record<string,string>;export default catalog; }
