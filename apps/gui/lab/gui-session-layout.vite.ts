import {readFileSync,realpathSync} from 'node:fs';
import {dirname} from 'node:path';
import {fileURLToPath} from 'node:url';
import {defineConfig,mergeConfig} from 'vite';
import preview from '../scripts/ai-elements-preview.vite';

// Only this Lab server opts into the experimental layout. App/main stay unchanged.
const tokens=readFileSync(new URL('../src/styles/tokens.css',import.meta.url),'utf8');
const coarse:string[]=[];
for(const match of tokens.matchAll(/@media\s*\(pointer:\s*coarse\)\s*\{/g)){
  let cursor=match.index!+match[0].length,depth=1,end=cursor;
  while(depth&&end<tokens.length){if(tokens[end]==='{')depth++;if(tokens[end]==='}')depth--;end++;}
  coarse.push(tokens.slice(cursor,end-1));
}
export default mergeConfig(preview,defineConfig({
  plugins:[{
    name:'gui-session-layout-lab',enforce:'pre',
    transform(code,id){
      if(id.endsWith('/src/App.tsx')){
        const anchor=/<SessionScreen\s+/;
        if(!anchor.test(code))throw new Error('SessionScreen Lab mount moved');
        code="import '/lab/gui-session-dock.css';\nimport {LabSessionHeader} from '/lab/gui-session-header';\nimport {LabModelHeader} from '/lab/gui-session-compose';\n"+code;
        return code.replace(anchor,`<SessionScreen
      renderHeader={(selectedRow?.kind === 'gui' || isChatTab(selected)) ? ['compose','fan'].includes(new URLSearchParams(location.search).get('deck')??'') ? props=><LabModelHeader {...props}/> : new URLSearchParams(location.search).get('deck') === 'light' ? props=><LabSessionHeader {...props}/> : undefined : undefined}
      combinedDock={new URLSearchParams(location.search).get('deck') === 'combined' && (selectedRow?.kind === 'gui' || isChatTab(selected))}
      deckAt={['bottom','combined'].includes(new URLSearchParams(location.search).get('deck') ?? '') ? 'bottom' : selectedRow?.kind === 'gui' || isChatTab(selected) ? 'top' : 'bottom'}
      `).replace(/onTabs=\{narrow\s*&&\s*selectedRow\s*\?/,"onTabs={narrow&&selectedRow&&new URLSearchParams(location.search).get('deck')!=='combined'&&new URLSearchParams(location.search).get('deck')!=='light'&&new URLSearchParams(location.search).get('deck')!=='compose'&&new URLSearchParams(location.search).get('deck')!=='fan'?").replace(/<Tabs\s+grouped/,`<Tabs tail={<span data-lab-session-settings-slot />} grouped`);
      }
      if(id.endsWith('/src/views/Composer.tsx')){
        code="import {useLabComposeLayout} from '/lab/gui-session-compose';\nimport {useNarrow} from '@/hooks/use-narrow';\n"+code;
        code=code.replace('const cap = useRef<HTMLDivElement>(null);',`const cap = useRef<HTMLDivElement>(null);
        const labNarrow=useNarrow();
        const labCompose=labNarrow&&['compose','fan'].includes(new URLSearchParams(location.search).get('deck')??'')&&new URLSearchParams(location.search).get('composer')!=='classic';
        useLabComposeLayout(cap,labCompose);`);
        if(!code.includes('useLabComposeLayout(cap,labCompose)'))throw new Error('Lab composer ref mount moved');
        code=code.replace('data-chat-box','data-chat-box data-lab-compose-input={labCompose||undefined}');
        return code.replace('{lead}',`{labCompose&&<span data-lab-composer-session-slot/>}{lead}`);
      }
      if(id.endsWith('/src/views/ChatView.tsx')){
        code="import {useLabSessionDraft} from '/lab/gui-session-drafts';\n"+code;
        code=code.replace('const [text, setText] = useState("");','const [text, setText] = useLabSessionDraft("gui:"+id);');

        code=code.replace(/\{home\s*!==\s*null\s*&&\s*(\(?\s*)<SendInto/,'{home !== null && (!labUnified || files.items.length > 0) && $1<SendInto');
        if(!code.includes('(!labUnified || files.items.length > 0)'))throw new Error('Lab attachment destination mount moved');
        code=code.replace('placeholder={gui.chat_input}','placeholder={labUnified ? "发消息…" : gui.chat_input}');
        code=code.replace(/<LabAdd\s+/,`<LabAdd destination={home !== null ? <SendInto machine={machine ?? ""} name={home} into={into} onInto={setInto} /> : null} `);
        return code;
      }
      if(id.endsWith('/src/views/MachineChat.tsx')){
        code=`import {labUnified,KhorHeader,LabAdd,KhorSend} from '/lab/gui-session-composer';\nimport {useLabSessionDraft} from '/lab/gui-session-drafts';\n`+code;
        code=code.replace('const [draft, setDraft] = useState("");','const [draft, setDraft] = useLabSessionDraft("khor:"+id);');
        const running='log?.messages.findLast(m => m.command?.agent && (m.command.status === "running" || m.command.status === "blocked"))?.command ?? null';
        code=code.replace('<BarSeat at="above">',`<BarSeat at="above">{labUnified && <KhorHeader view={agent} onChanged={setAgent} disabled={!!(${running})} updates={log?.messages.flatMap(m => m.command?.agent?.model === agent?.config?.model ? m.command.agent.updates : []) ?? []} />}`);
        code=code.replace(/<div className="flex justify-end pt-in">\s*<KhorAgentSettings([\s\S]*?)<\/div>/,match=>`{!labUnified && (${match})}`);
        code=code.replace(/<PlusSeat data-attach-file aria-label=\{gui.send_file\} onClick=\{attach\} \/>/,'<LabAdd onAttach={attach} options={[]} />');
        code=code.replace(/trail=\{[\s\S]*?ready\s*\?\s*<SendSeat[\s\S]*?\/>\s*:\s*null\s*\}/,`trail={labUnified ? <KhorSend record={${running}} onStopped={refresh} ready={ready} sending={sending} onSend={send} /> : ready ? <SendSeat onSend={send} disabled={sending} keepFocus /> : null}`);
        code=code.replace('placeholder={gui.agent_placeholder}','placeholder={labUnified ? "发消息…" : gui.agent_placeholder}');
        return code;
      }
      if(id.endsWith('/src/views/chat-files.tsx')){
        code=`import {labUnified} from '/lab/gui-session-composer';\n`+code;
        return code.replace('{gui.send_into}','{labUnified ? "上传到" : gui.send_into}').replace('text={into ?? at ?? "~/khor-drop"}','text={labUnified ? name + " · " + (into ?? at ?? "~/khor-drop").split(/[/\\\\]/).filter(Boolean).at(-1) : into ?? at ?? "~/khor-drop"}');
      }
    },
    transformIndexHtml(_html,ctx){
      if(ctx.path!=='/'&&ctx.path!=='/index.html')return;
      return [{tag:'script',injectTo:'head-prepend',children:`
        {
          const q=new URLSearchParams(location.search),session=q.get('labSession')||'gui/ai-elements-review';
          const sessions=['gui/ai-elements-review','gui/ai-elements-plan','gui/ai-elements-stream','gui/codex-review','wide/all'];
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
            const names=['AI Elements · 组件总览','AI Elements · 计划与确认','AI Elements · 流式响应','Codex · 工具与补丁','khor'];
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
  server:{host:'0.0.0.0',port:23795,strictPort:true,fs:{allow:[fileURLToPath(new URL('../../..',import.meta.url)),dirname(realpathSync(new URL('../node_modules',import.meta.url)))]}},
}));
