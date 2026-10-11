/**
 * [INPUT]: 中英文 YAML、共享翻译入口与 TypeScript 源码目录。
 * [OUTPUT]: 双语键/插槽一致性、全部界面源文案覆盖、原生结果翻译及切换后保留原始数据的回归。
 * [POS]: 本地化的离线验证；不启动图形界面，不修改系统语言或用户设置。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {test,expect} from 'bun:test';
import {readFileSync,readdirSync} from 'node:fs';
import {parse} from 'yaml';
import ts from 'typescript';
import {translate,setLocale} from './i18n';
const zh:Record<string,string>=parse(readFileSync(new URL('./locales/zh.yaml',import.meta.url),'utf8'));
const en:Record<string,string>=parse(readFileSync(new URL('./locales/en.yaml',import.meta.url),'utf8'));
const slots=(value:string)=>[...value.matchAll(/\{(\w+)\}/g)].map(match=>match[1]).sort();
test('双语目录完整且命名插槽一致',()=>{
  expect(Object.keys(en).sort()).toEqual(Object.keys(zh).sort());
  for(const key of Object.keys(zh)){
    expect(en[key].length).toBeGreaterThan(0);
    expect(slots(en[key])).toEqual(slots(zh[key]));
    // 应用真实路径与名称保留，其他英文文案不得混入中文。
    expect(/[\u3400-\u9fff]/.test(en[key].replaceAll('CC浏览器.app',''))).toBe(false);
  }
});
test('每个界面源文案与辅助名称都在 YAML 中',()=>{
  for(const name of readdirSync(new URL('.',import.meta.url)).filter(name=>name.endsWith('.tsx'))){
    const source=readFileSync(new URL(name,import.meta.url),'utf8');
    const file=ts.createSourceFile(name,source,ts.ScriptTarget.Latest,true,ts.ScriptKind.TSX);
    function walk(node:ts.Node){
      if(ts.isCallExpression(node)&&node.expression.getText(file)==='t'&&ts.isStringLiteral(node.arguments[0]))expect(Object.hasOwn(zh,node.arguments[0].text)).toBe(true);
      if(ts.isJsxText(node))expect(/[\u3400-\u9fff]/.test(node.text)).toBe(false);
      ts.forEachChild(node,walk);
    }
    walk(file);
  }
});
test('原生错误、参数和原始路径可切换语言，未知消息原样保留',()=>{
  setLocale('en');
  expect(translate('已结束 4 个 Claude 进程，可以清理。')).toBe('Stopped 4 Claude processes. Ready to clean.');
  expect(translate('配置残留 · ~/.claude/settings.json')).toBe('Configuration · ~/.claude/settings.json');
  expect(translate('Error: 清理快照已过期，请重新扫描。')).toBe('Error: Scan snapshot expired. Run another scan.');
  expect(translate('钥匙串删除失败：已取消系统授权；余下凭据未处理。（OSStatus -128）')).toBe('Keychain deletion failed: System authorization cancelled. Remaining credentials left unchanged. (OSStatus -128)');
  expect(translate('本机：America/New_York；浏览器：待测；出口：待测')).toBe('Computer: America/New_York; browser: Not checked; exit: Not checked');
  expect(translate('已处理 2 项；部分完成，请检查失败记录。')).toBe('Processed 2 items; partially complete. Check the error log.');
  expect(translate('出口检测未完成（ipwho.is：HTTP 429；ipapi.co：网络连接失败）。请检查 TUN/代理连接后重试。')).toBe('Exit check failed (ipwho.is: HTTP 429; ipapi.co: Network connection failed). Check your TUN/proxy connection and retry.');
  expect(translate('未知错误 0xABC')).toBe('未知错误 0xABC');
  expect(translate('__proto__')).toBe('__proto__');
  const raw='已发现 12 个可选项目。';
  expect(translate(raw)).toBe('Found 12 selectable items.');
  setLocale('zh');expect(translate(raw)).toBe(raw);
  setLocale('fr');expect(translate('网络环境')).toBe('网络环境');
  setLocale('en');expect(translate('时区已核验。{v0}',{v0:'首选语言与地区格式已保存，部分应用需重启或注销。'})).toBe('Time zone verified. Preferred language and region saved. Some apps require restarting or signing out.');
  setLocale('zh');
});
