/**
 * [INPUT]: 脱敏的清理样本与页面选择规则。
 * [OUTPUT]: 预设范围、父子预览去重及单项选择保护的验证。
 * [POS]: 纯逻辑回归；不接触本机数据或 Tauri 清理命令。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import {expect,test} from 'bun:test';
import {effectiveItems,deselectItems,isDescendantPath,presetIds,reselectIds,type Item} from './cleaner-model';
const item=(id:string,path:string,risk:string):Item=>({id,path,risk,title:id,group:'test',impact:'',bytes:10,files:1,exact:true,keys:[]});
const samples=[item('cache','/home/Claude/Cache','safe'),item('desktop','/home/Claude','login'),item('history','/home/.claude/history.jsonl','history'),item('settings','/home/.claude/settings.json','config')];
test('默认清理不选择账号、历史与配置，完整预设选择全部可清理项',()=>{
 expect(presetIds(samples,'cache')).toEqual(['cache']);
 expect(presetIds(samples,'reset')).toEqual(['cache','desktop']);
 expect(presetIds(samples,'full')).toEqual(['cache','desktop','history','settings']);
});
test('整目录只执行一次；单独选择子目录不会扩大到父目录',()=>{
 expect(effectiveItems(samples,presetIds(samples,'full')).map(i=>i.id)).toEqual(['desktop','history','settings']);
 expect(effectiveItems(samples,['cache']).map(i=>i.id)).toEqual(['cache']);
 expect(effectiveItems(samples,['unknown'])).toEqual([]);
});

test('排除子项同时取消整目录选择，避免被父目录隐式清理',()=>{
 const remaining=deselectItems(samples,presetIds(samples,'full'),['cache']);
 expect(remaining).toEqual(['history','settings']);
 expect(effectiveItems(samples,remaining).some(i=>i.path.startsWith('/home/Claude'))).toBe(false);
});

test('既有备份只纳入完整清理，缓存和登录预设不扩大范围',()=>{
 const backups=[...samples,item('backup','/home/ClaudeBackups','backup')];
 expect(presetIds(backups,'cache')).toEqual(['cache']);
 expect(presetIds(backups,'reset')).toEqual(['cache','desktop']);
 expect(presetIds(backups,'full')).toContain('backup');
});

test('结束进程后的新快照按原路径恢复选择，不扩大到新项目',()=>{
 const next=[item('new-cache','/home/Claude/Cache','safe'),item('new-desktop','/home/Claude','login'),item('new-data','/home/extra','login')];
 expect(reselectIds(samples,['cache'],next)).toEqual(['new-cache']);
});

test('Windows 反斜杠路径保持父子去重和取消选择语义',()=>{
 const parent='C:\\Users\\fixture\\.claude\\projects';
 const child=parent+'\\org\\session.jsonl';
 const windows=[item('parent',parent,'history'),item('child',child,'history')];
 expect(isDescendantPath(parent,child)).toBe(true);
 expect(effectiveItems(windows,['parent','child']).map(i=>i.id)).toEqual(['parent']);
 expect(deselectItems(windows,['parent','child'],['child'])).toEqual([]);
 expect(deselectItems(windows,['parent','child'],['parent'])).toEqual(['child']);
});

test('同名前缀但非子路径不会被去重',()=>{
 expect(isDescendantPath('/home/Claude/projects','/home/Claude/projects-old')).toBe(false);
 expect(isDescendantPath('C:\\Users\\a\\Claude','C:\\Users\\ab\\Claude')).toBe(false);
});

test('保留 POSIX 大小写、反斜杠与合法文件名空格',()=>{
 expect(isDescendantPath('/home/Claude','/home/claude/data')).toBe(false);
 expect(isDescendantPath('/home/name\\folder','/home/name/folder/child')).toBe(false);
 expect(isDescendantPath('/home/name ','/home/name/child')).toBe(false);
 expect(isDescendantPath('/','/home/child')).toBe(true);
 expect(isDescendantPath('C:\\Users\\Fixture','c:/users/fixture/child')).toBe(true);
});
