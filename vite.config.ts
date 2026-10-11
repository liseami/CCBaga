/**
 * [INPUT]: 依赖 Vite 静态构建与 Tailwind 4 的同源组件样式编译与构建期 YAML 解析。
 * [OUTPUT]: 提供工具箱与浏览器起始页两个构建入口，资源使用相对路径；YAML 文案编译为静态对象，解析器不进入运行包。
 * [POS]: 前端与 Tauri 宿主之间的构建入口。
 * [PROTOCOL]: 变更时更新此头部，然后检查 CLAUDE.md
 */
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import {parse} from 'yaml';
import tailwindcss from '@tailwindcss/vite';
export default defineConfig({base:"./",plugins:[{name:'yaml-catalog',transform(source,id){if(!id.endsWith('.yaml'))return;const data=parse(source);if(!data||typeof data!=='object'||Object.values(data).some(value=>typeof value!=='string'))throw new Error('YAML catalog must map strings to strings');return {code:'export default '+JSON.stringify(data),map:null};}},react(),tailwindcss()],clearScreen:false,server:{port:1420,strictPort:true},build:{target:'safari15',rollupOptions:{input:{main:'index.html',browser:'browser.html'},output:{manualChunks(id){if(id.includes('/src/locales/')&&id.endsWith('.yaml'))return 'locales';}}}}});
