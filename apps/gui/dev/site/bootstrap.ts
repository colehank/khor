import {createSiteFixture} from './fixture';
import {configureLabBackend} from '../lab/backend-context';
import {scenes,type Scene} from '../lab/scenes';
import {projects,tasks} from './world';
import {constrainDemoInteractions} from './interaction';
import {siteRoot,downloadUrl} from './urls';
import {setSessionSide,setSideWidth} from '../../src/hooks/use-session-side';
import controls from './macos-controls.svg?url';
import './desktop.css';
import {WorkspaceNavigationStore} from '../../src/views/workspace/workspace-navigation-storage';
import {workspaceTabId} from '../../src/views/workspace/workspace-tabs';
import {terminalTour,type TourScene} from './tour-scenario';

// This entry uses the production App with browser-local, synthetic data only.
configureLabBackend('frontend');
const query = new URLSearchParams(location.search);
const tour:TourScene=['p2p','agent','terminal'].includes(query.get('tour')??'')?query.get('tour') as TourScene:null;
const requested = query.get('scene') ?? 'gui';
const hero=query.get('hero')==='1';
if (!(requested in scenes)) throw new Error('Unknown demo scene');
const scene = scenes[requested as Scene];
const active = tour==='terminal'?tasks[4].id:hero||requested==='gui'||requested==='content'?tasks[0].id:scene.session??tasks[0].id;
const identity = `site-v7/${hero?'hero':requested}/${tour??'default'}`;
if(localStorage.getItem('khor.site.identity')!==identity){
  localStorage.setItem('khor.site.identity',identity);
  localStorage.setItem('khor.lang','zh');
  localStorage.setItem('khor.sessions.tabs',JSON.stringify({tabs:[active,...tasks.map(task=>task.id).filter(id=>id!==active)].slice(0,3),active}));
  const landing=['workspace','workspaces'].includes(requested)?'workspaces':requested==='devices'?'devices':'sessions';
  localStorage.setItem('khor.place',JSON.stringify({landing,screen:'list'}));
}
const theme=query.get('theme')==='dark'?'dark':query.get('theme')==='light'?'light':'auto';
localStorage.setItem('khor.theme',theme);
localStorage.setItem('khor.preview-ground','auto');
const fixture=createSiteFixture(siteRoot,tour);
const workspacePaths=['README.md','src/export.ts','src/export.test.ts'].map(path=>({kind:'file' as const,path:projects[0].root+'/'+path}));
new WorkspaceNavigationStore(()=>localStorage,'MacBook Pro',projects[0].root,path=>({kind:'file',path})).save({at:projects[0].root,historyOpen:false,content:{items:workspacePaths,active:workspaceTabId(workspacePaths[0]),preview:null}});
// Discard a previous demo's pinned theme; the production theme listener
// follows the browser live while its preference is auto.
fixture.request('set_config',{name:'theme',value:theme});
if(hero){
  document.documentElement.dataset.siteMacos='';
  // Reuse the production window's single translucent background plane.
  // Browser frost is painted by the parent window, behind this iframe;
  // the fixture remains HTTP-only and never enables a native backend.
  document.documentElement.dataset.windowTransparent='';
  document.documentElement.dataset.siteMaterial='frost';
  document.documentElement.style.setProperty('--window-opacity','40%');
  const lights=document.createElement('img');
  lights.src=controls;lights.alt='';lights.className='site-macos-controls';lights.setAttribute('aria-hidden','true');
  document.body.append(lights);
  setSessionSide(active,{open:tour!=='agent'&&tour!=='terminal',seat:'workspace'});
  setSideWidth('workspace',innerWidth*(tour==='p2p'?.55:.38));
}
if((hero||requested==='content')&&tour!=='agent'&&tour!=='terminal'){
  const devices=fixture.request('devices') as {id:string;name:string}[];
  for(const task of tasks.slice(0,1)){
    const project=projects.find(project=>project.root===task.root)!;
    const paths=tour==='p2p'?[terminalTour.url]:['README.md','src/export.ts','src/export.test.ts'];
    let first='';
    for(const path of paths){
      const id=fixture.request('tabs_open',{kind:'content',session:task.id,device:devices[task.machine].id,view:tour==='p2p'?'page':'file',url:tour==='p2p'?path:project.root+'/'+path,title:tour==='p2p'?'Atlas Web':path.split('/').pop()}) as string;
      if(!first)first=id;
    }
    fixture.request('tabs_focus',{kind:'content',id:first});
  }
}
if(tour==='p2p')setSessionSide(active,{open:true,seat:'workspace'});
constrainDemoInteractions(tour==='terminal'?active:undefined);
document.addEventListener('click',event=>{
  const link=(event.target as Element).closest<HTMLElement>('[data-md-link]');
  if(link?.dataset.mdLink!==downloadUrl)return;
  event.preventDefault();event.stopImmediatePropagation();
  if(parent!==window)parent.postMessage({type:'khor-demo-download'},location.origin);
  else location.assign(downloadUrl);
},true);
const original=window.fetch.bind(window);
window.fetch=async(input,init)=>{
  const request=new Request(input,init),url=new URL(request.url);
  if(url.origin!==location.origin||!url.pathname.startsWith('/api/'))return original(input,init);
  if(request.signal.aborted)throw new DOMException('Aborted','AbortError');
  try{
    const command=url.pathname.slice('/api/'.length);
    const body=command==='chat_put'?Object.fromEntries(url.searchParams):await request.json();
    if((command==='chat_wait'&&body.id!==tasks[0].id)||['term_wait','term_stream','dirs_wait'].includes(command)){
      return await new Promise<Response>((_,reject)=>request.signal.addEventListener('abort',()=>reject(new DOMException('Aborted','AbortError')),{once:true}));
    }
    const answer=fixture.request(command,body);
    if(command==='chat_wait')await new Promise(resolve=>setTimeout(resolve,300));
    return Response.json(answer);
  }catch(error){return Response.json(String(error),{status:400});}
};
window.addEventListener('pagehide',()=>fixture.dispose(),{once:true});
const enter=()=>{
  if(scene.row){
    const row=[...document.querySelectorAll<HTMLElement>('[data-list] [data-row-open]')].find(el=>el.closest('[data-row]')?.getAttribute('data-row')===active);
    if(row){row.click();if(hero)setSessionSide(active,{open:tour!=='agent'&&tour!=='terminal',seat:'workspace'});observer.disconnect();}
  }else if(requested==='workspace'){
    const file=document.querySelector<HTMLElement>('[data-file-path="'+projects[0].root+'/README.md"]');
    if(file){file.click();observer.disconnect();return;}
    const row=[...document.querySelectorAll<HTMLElement>('[data-list] [data-ws-row]')].find(el=>el.textContent?.includes(projects[0].name));
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
    // Any mounted App page is interactive. A restored devices/settings page
    // has no composer and must not remain trapped behind the static poster.
    if(!document.getElementById('root')?.childElementCount)return;
    ready.disconnect();parent.postMessage({type:'khor-demo-ready'},location.origin);
  });
  ready.observe(document.body,{childList:true,subtree:true});
}
await import('../../src/main');
