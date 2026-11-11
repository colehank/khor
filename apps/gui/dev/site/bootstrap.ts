import {createLabFixture} from '../lab/fixture';
import {configureLabBackend} from '../lab/backend-context';
import {scenes,sessionIds,type Scene} from '../lab/scenes';
import {scenario} from './scenario';
import {setSessionSide,setSideWidth} from '../../src/hooks/use-session-side';

// This entry uses the production App with browser-local, synthetic data only.
configureLabBackend('frontend');
const query = new URLSearchParams(location.search);
const requested = query.get('scene') ?? 'gui';
const hero=query.get('hero')==='1';
if (!(requested in scenes)) throw new Error('Unknown demo scene');
const scene = scenes[requested as Scene];
const active = scene.session ?? sessionIds[0];
const identity = `site-v2/${hero?'hero':requested}`;
if(localStorage.getItem('khor.site.identity')!==identity){
  localStorage.setItem('khor.site.identity',identity);
  localStorage.setItem('khor.lang','zh');
  localStorage.setItem('khor.sessions.tabs',JSON.stringify({tabs:[active,...sessionIds.filter(id=>id!==active)].slice(0,3),active}));
  const landing=['workspace','workspaces'].includes(requested)?'workspaces':requested==='devices'?'devices':'sessions';
  localStorage.setItem('khor.place',JSON.stringify({landing,screen:'list'}));
}
localStorage.setItem('khor.theme',query.get('theme')==='dark'?'dark':'light');
const fixture=createLabFixture(true,{...scenario,reply:`Let's try Khor, [download it here.](${location.origin}/download/)`,origin:location.origin,contentText:'项目文件：/preview/project/README.md\n\n[打开项目网页]('+location.origin+'/lab/example)',settings:true,content:!hero&&['content','page'].includes(requested),page:requested==='page',items:3,machines:3});
if(hero){
  const readme={machine:'MacBook Pro',path:'/preview/project/README.md'};
  const before=fixture.request('read_text',readme) as {digest:string};
  fixture.request('write_text',{...readme,was:before.digest,text:'# Website\n\n## 首页适配\n\n让桌面和手机上的工作衔接起来。\n\n- [x] 查看现有首页\n- [x] 调整布局\n- [x] 预览修改\n\n## 接着做\n\n在会话中说明下一步，文件与修改记录都留在这里。\n'});
  for(const [view,url,title] of [['workspace','/preview/project','project'],['file','/preview/project/notes.md','notes.md'],['file','/preview/project/README.md','README.md']]){
    fixture.request('tabs_open',{kind:'content',session:active,view,url,title});
  }
  setSessionSide(active,{open:true,seat:'workspace'});
  setSideWidth('workspace',innerWidth*.38);
}
document.addEventListener('click',event=>{
  const link=(event.target as Element).closest<HTMLElement>('[data-md-link]');
  if(link?.dataset.mdLink!==location.origin+'/download/')return;
  event.preventDefault();event.stopImmediatePropagation();
  if(parent!==window)parent.postMessage({type:'khor-demo-download'},location.origin);
  else location.assign('/download/');
},true);
const original=window.fetch.bind(window);
window.fetch=async(input,init)=>{
  const request=new Request(input,init),url=new URL(request.url);
  if(url.origin!==location.origin||!url.pathname.startsWith('/api/'))return original(input,init);
  if(request.signal.aborted)throw new DOMException('Aborted','AbortError');
  try{
    const command=url.pathname.slice('/api/'.length);
    const body=command==='chat_put'?Object.fromEntries(url.searchParams):await request.json();
    const answer=fixture.request(command,body);
    if(['chat_wait','term_wait','term_stream','dirs_wait'].includes(command))await new Promise(resolve=>setTimeout(resolve,120));
    return Response.json(answer);
  }catch(error){return Response.json(String(error),{status:400});}
};
window.addEventListener('pagehide',()=>fixture.dispose(),{once:true});
const enter=()=>{
  if(scene.row){
    const row=[...document.querySelectorAll<HTMLElement>('[data-list] [data-row-open]')].find(el=>el.closest('[data-row]')?.getAttribute('data-row')===scene.session);
    if(row){row.click();if(hero)setSessionSide(active,{open:true,seat:'workspace'});observer.disconnect();}
  }else if(requested==='workspace'){
    const file=document.querySelector<HTMLElement>('[data-file-path="/preview/project/README.md"]');
    if(file){file.click();observer.disconnect();return;}
    const row=[...document.querySelectorAll<HTMLElement>('[data-list] [data-ws-row]')].find(el=>el.textContent?.includes('project'));
    if(row&&!workspaceOpened){workspaceOpened=true;row.click();}
  }
};
let workspaceOpened=false;
const observer=new MutationObserver(enter);
observer.observe(document.body,{childList:true,subtree:true});
setTimeout(()=>observer.disconnect(),15000);
if(requested==='page'){
  const pageObserver=new MutationObserver(()=>{
    const link=document.querySelector<HTMLElement>('[data-md-link=\"'+location.origin+'/lab/example\"]');
    if(link){pageObserver.disconnect();link.click();}
  });
  pageObserver.observe(document.body,{childList:true,subtree:true});
  setTimeout(()=>pageObserver.disconnect(),15000);
}
Object.defineProperty(window,'__KHOR_SITE_DEMO__',{value:Object.freeze({scene:requested,synthetic:true})});
if(hero&&parent!==window){
  const ready=new MutationObserver(()=>{
    if(!document.querySelector('[placeholder="发消息…"]')||!document.querySelector('[data-session-content]'))return;
    ready.disconnect();parent.postMessage({type:'khor-demo-ready'},location.origin);
  });
  ready.observe(document.body,{childList:true,subtree:true});
}
await import('../../src/main');
