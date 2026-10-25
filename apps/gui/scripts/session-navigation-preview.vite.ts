import {readFileSync,realpathSync} from 'node:fs';
import {dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {defineConfig,mergeConfig} from 'vite';
import {aiElementsPreview} from './ai-elements-preview.vite';

// Isolated data, keyboard and phone fixtures around the production App.
const tokens=readFileSync(new URL('../src/styles/tokens.css',import.meta.url),'utf8');
const coarse:string[]=[];
for(const match of tokens.matchAll(/@media\s*\(pointer:\s*coarse\)\s*\{/g)){
  let cursor=match.index!+match[0].length,depth=1,end=cursor;
  while(depth&&end<tokens.length){if(tokens[end]==='{')depth++;if(tokens[end]==='}')depth--;end++;}
  coarse.push(tokens.slice(cursor,end-1));
}
export default mergeConfig(aiElementsPreview(true),defineConfig({
  plugins:[{
    name:'gui-session-layout-lab',enforce:'pre',
    transform(code,id){
      if(id.endsWith('/src/views/TerminalPane.tsx'))return code.replace(/import\s*\{\s*useCoarse\s*\}\s*from\s*['"]@\/hooks\/use-coarse['"];?/,"import {useSessionPreviewCoarse as useCoarse} from '/scripts/session-navigation-coarse';");
    },
    transformIndexHtml(_html,ctx){
      if(ctx.path!=='/'&&ctx.path!=='/index.html')return;
      return [{tag:'script',injectTo:'head-prepend',children:`
        {
          const q=new URLSearchParams(location.search),session=q.get('labSession')||'gui/ai-elements-review';
          const sessions=['gui/ai-elements-review','gui/ai-elements-plan','gui/ai-elements-stream','gui/codex-review','wide/all','shell/layout-lab'];
          const requested=q.get('labTabs')?.split(',').filter(id=>sessions.includes(id));
          const tabs=requested?.includes(session)?requested:sessions;
          localStorage.setItem('khor.sessions.tabs',JSON.stringify({tabs,active:session}));
          localStorage.setItem('khor.place',JSON.stringify({landing:'sessions',screen:'list'}));
          window.addEventListener('DOMContentLoaded',()=>{
            const root=document.documentElement,phone=q.get('phone')==='1';
            if(q.get('labSession')==='wide/all'&&q.has('approval'))fetch('/__ai_preview/khor-approval',{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify({pending:q.get('approval')==='1'})});
            root.dataset.layoutLab='';
            if(q.get('composer')!=='classic')root.dataset.labUnified='';
            if(phone){
              root.setAttribute('data-ios','');
              const style=document.createElement('style');
              style.textContent=${JSON.stringify(coarse.join('\n'))}+'\\n:root[data-layout-lab]{--safe-top:'+(innerWidth>innerHeight?0:47)+'px}:root[data-layout-lab]:not([data-keyboard]){--safe-bottom:'+(innerWidth>innerHeight?21:34)+'px}';
              document.head.append(style);
            }
            if(q.get('keyboard')==='1'&&window.visualViewport){
              const cover=Math.min(300,Math.round(innerHeight*.4));
              Object.defineProperty(window.visualViewport,'height',{configurable:true,get:()=>innerHeight-cover});
              window.visualViewport.dispatchEvent(new Event('resize'));
              const key=document.createElement('div');key.dataset.labKeyboard='';key.textContent='键盘区域（模拟）';
              key.style.cssText='position:fixed;inset:auto 0 0;height:'+cover+'px;display:grid;place-items:center;background:var(--fill-strong);color:var(--muted-foreground);font-size:var(--text-callout);border-top:var(--hair) solid var(--border)';
              document.body.append(key);
            }
            const names=['AI Elements · 组件总览','AI Elements · 计划与确认','AI Elements · 流式响应','Codex · 工具与补丁','khor','终端 · 布局试验'];
            const title=names[sessions.indexOf(session)]||names[0];
            const open=()=>{
              if(document.querySelector('[data-session-screen]')){observer.disconnect();return;}
              const row=[...document.querySelectorAll('[data-list] [data-row-open]')].find(el=>el.textContent?.includes(title));
              if(row){row.click();observer.disconnect();}
            };
            const observer=new MutationObserver(open);observer.observe(document.body,{childList:true,subtree:true});open();
            if(q.get('attachment')==='1'){
              const attach=()=>{
                const input=document.querySelector('[data-chat-file-input], [data-file-input]');
                if(!input)return;
                filesObserver.disconnect();const files=new DataTransfer();files.items.add(new File(['# 布局讨论\\n这是 Lab 的示例附件。'],'设计说明.md',{type:'text/markdown'}));
                input.files=files.files;input.dispatchEvent(new Event('change',{bubbles:true}));
              };
              const filesObserver=new MutationObserver(attach);filesObserver.observe(document.body,{childList:true,subtree:true});attach();
            }
          },{once:true});
        }
      `}];
    },
  }],
  optimizeDeps:{entries:['index.html','lab/gui-session-layout.html']},
  server:{host:'0.0.0.0',port:23805,strictPort:true,fs:{allow:[fileURLToPath(new URL('../../..',import.meta.url)),dirname(realpathSync(new URL('../node_modules',import.meta.url)))]}},
}));
