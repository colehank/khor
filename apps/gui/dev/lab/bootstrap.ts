import {handOff} from '../../src/lib/composer-handoff';
import {createLabFixture} from './fixture';
import {scenes, sessionIds, independentKhorSession, type Scene} from './scenes';
import {configureLabBackend} from './backend-context';
import {FixtureRefusal} from './navigation';

type Runtime = {mode: 'frontend'|'full'; scene: Scene; tree: string; revision: string};
const version = document.querySelector<HTMLMetaElement>('meta[name="khor-lab-version"]')?.content;
const settings: Runtime = await fetch('/lab/runtime' + (version ? '?version=' + encodeURIComponent(version) : '')).then(r => {
  if (!r.ok) throw new Error('Start this entry with khor-dev lab start');
  return r.json();
});
const query = new URLSearchParams(location.search);
const requested = settings.mode === 'frontend' ? query.get('scene') : null;
if (requested && !(requested in scenes)) throw new Error('Unknown Lab scene');
const runtime = requested ? {...settings, scene: requested as Scene} : settings;
if (!(runtime.scene in scenes)) throw new Error('Unknown Lab scene');
const native = '__TAURI_INTERNALS__' in window;
if (runtime.mode === 'full' && !native) throw new Error('Full Lab requires its isolated native host; use the frontend link for browser review');
configureLabBackend(runtime.mode);
// Native pointer diagnostics stay inside the isolated Lab. Record event state,
// never terminal bytes, destinations or credentials; production has no listener.
if (runtime.mode === 'full') {
  const {faceNote} = await import('../../src/api');
  let remaining = 32;
  for (const type of ['mousedown', 'mouseup']) document.addEventListener(type, event => {
    const mouse = event as MouseEvent;
    const host = (event.target as Element | null)?.closest<HTMLElement>('[data-terminal]');
    const term = (host as HTMLElement & {__xterm?: {hasSelection:()=>boolean;modes:{mouseTrackingMode:string};_core?:{linkifier?:{currentLink?:unknown}}}} | null)?.__xterm;
    if (!term || remaining-- <= 0) return;
    const state = {type,button:mouse.button,selected:term.hasSelection(),tracking:term.modes.mouseTrackingMode,linked:!!term._core?.linkifier?.currentLink,target:(event.target as Element).tagName};
    void faceNote('Lab terminal pointer '+JSON.stringify(state)).catch(()=>{});
  }, true);
}
if (query.has('bridge')) throw new Error('Lab does not connect to external API bridges');
const scene = scenes[runtime.scene];
const khorSession = runtime.scene==='khor'||query.get('khor')==='independent' ? independentKhorSession : undefined;
const items = Number(query.get('items') ?? 5), machines = Number(query.get('machines') ?? 3);
if (![2,3,5].includes(items) || ![0,1,3,6].includes(machines)) throw new Error('Lab data supports items=2/3/5 and machines=0/1/3/6');
const pageSize=query.has('short-pages')?4:128;
let noticeDriver:{ready:()=>boolean;trigger:(burst:boolean)=>string[]}|undefined;
const identity = `${query.get('history')??'default'}/${pageSize}/${query.has('handoff')}/${runtime.tree}/${runtime.scene}/${runtime.revision}/${items}/${machines}/${query.get('file')??''}/${khorSession??''}`;
if (runtime.mode === 'frontend') {
  if (localStorage.getItem('khor.lab.identity') !== identity) {
    for (const key of Object.keys(localStorage)) if (key.startsWith('khor.')) localStorage.removeItem(key);
    localStorage.setItem('khor.lab.identity', identity);
    localStorage.setItem('khor.lang', 'zh');
    const active = scene.session ?? sessionIds[0];
    const available = sessionIds.map(id=>id==='wide/all'&&khorSession?khorSession:id);
    const tabs = [active,...available.filter(id => id !== active)].slice(0,items);
    localStorage.setItem('khor.sessions.tabs', JSON.stringify({tabs,active}));
    const landing = ['workspace','workspaces'].includes(runtime.scene) ? 'workspaces' : runtime.scene === 'devices' ? 'devices' : runtime.scene === 'browser' ? 'browser' : runtime.scene === 'quota' ? 'quota' : 'sessions';
    localStorage.setItem('khor.place', JSON.stringify({landing, screen: 'list'}));
  }
  const fixture = createLabFixture(true, {settings:true,origin: location.origin, content: ['content', 'page'].includes(runtime.scene), items, machines,pageSize, longHistory:query.get('history')==='long', notices:query.get('notices')==='1',noticeBurst:query.get('notices')==='burst', page:runtime.scene==='page',khorSession});
  if(query.get('notices')==='1'||query.get('notices')==='burst')noticeDriver={
    ready:()=>fixture.request('__ai_preview/notification-ready')===true,
    trigger:burst=>fixture.request('__ai_preview/notification',{burst}) as string[],
  };
  const uploadedText = new Map<string, {text: string; total: number; at_ms: number; digest: string; cut: boolean; binary: boolean}>();
  const original = window.fetch.bind(window);
  window.fetch = async (input, init) => {
    const request = new Request(input, init);
    const url = new URL(request.url);
    if (url.origin !== location.origin || !url.pathname.startsWith('/api/')) return original(input, init);
    if (request.signal.aborted) throw new DOMException('Aborted', 'AbortError');
    try {
      const command = url.pathname.slice('/api/'.length);
      let body:Record<string,unknown>;
      if(command==='chat_put'){
        body=Object.fromEntries(url.searchParams);
        const bytes=await request.arrayBuffer();
        const mode=fixture.request('__ai_preview/upload-state');
        if(mode==='fail')return Response.json('上传失败，请移除附件后重新添加',{status:503});
        if(mode==='delay')await new Promise(resolve=>setTimeout(resolve,1200));
        const path=fixture.request(command,body);
        if(typeof path==='string' && path.endsWith('.txt')){
          uploadedText.set(path,{text:new TextDecoder().decode(bytes),total:bytes.byteLength,at_ms:Date.now(),digest:crypto.randomUUID(),cut:false,binary:false});
        }
        return Response.json(path);
      }else body=await request.json();
      if(['chat_poll','chat_wait'].includes(command)&&fixture.request('__ai_preview/poll-state')==='fail')return Response.json('读取暂时中断',{status:503});
      if(command==='app_update_check')await new Promise(resolve=>setTimeout(resolve,900));
      const text = (command === 'read_text' || command === 'stat_text') && uploadedText.get(String(body.path));
      const answer = text || fixture.request(command,body);
      // Polling fixtures must yield so production loops cannot starve rendering.
      if (['chat_wait', 'term_wait', 'term_stream', 'dirs_wait'].includes(command)) await new Promise(r => setTimeout(r, 120));
      return Response.json(answer);
    } catch (error) {
      if (!(error instanceof FixtureRefusal)) console.error('[Lab fixture]', url.pathname, error);
      return Response.json(String(error), {status: 400});
    }
  };
  window.addEventListener('pagehide', () => fixture.dispose(), {once: true});
}
const theme = query.get('theme');
if (theme && ['light', 'dark', 'auto'].includes(theme)) localStorage.setItem('khor.theme', theme);
// Enter through the actual list action; no alternative React tree, layout or gesture host.
if (runtime.mode === 'frontend' && scene.row) {
  const enter = () => {
    if (document.querySelector('[data-session-screen]')) {observer.disconnect();return;}
    const row = [...document.querySelectorAll<HTMLElement>('[data-list] [data-row-open]')].find(el => el.closest('[data-row]')?.getAttribute('data-row')===scene.session);
    if (row) {row.click();observer.disconnect();}
  };
  const observer = new MutationObserver(enter);
  observer.observe(document.body, {childList: true, subtree: true});
  setTimeout(() => observer.disconnect(), 15000);
}
if (runtime.mode === 'frontend' && runtime.scene === 'content') {
  const openFile = () => {
    const link = document.querySelector<HTMLElement>('[data-link="/preview/project/README.md"]');
    if (link) {observer.disconnect();link.click();}
  };
  const observer = new MutationObserver(openFile);
  observer.observe(document.body, {childList: true, subtree: true});
  setTimeout(() => observer.disconnect(), 15000);
}
if (runtime.mode === 'frontend' && runtime.scene === 'page') {
  const openPage = () => {
    const link = document.querySelector<HTMLElement>('[data-md-link="' + location.origin + '/lab/example"]');
    if (link) {observer.disconnect();link.click();}
  };
  const observer = new MutationObserver(openPage);
  observer.observe(document.body, {childList: true, subtree: true});
  setTimeout(() => observer.disconnect(), 15000);
}
if (runtime.mode === 'frontend' && runtime.scene === 'workspace') {
  let opened = false,folderOpened=false;
  const enter = () => {
    const requested=query.get('file');
    const name=requested&&['media/preview.html','media/clip.mp4','media/audio.wav'].includes(requested)?requested:'README.md';
    const file = document.querySelector<HTMLElement>('[data-file-path="/preview/project/'+name+'"]');
    const folder=name.startsWith('media/')?document.querySelector<HTMLElement>('[data-file-path="/preview/project/media"]'):null;
    if(!file&&folder&&!folderOpened){folderOpened=true;folder.click();return;}
    if (file) {observer.disconnect();file.click();return;}
    const row = [...document.querySelectorAll<HTMLElement>('[data-list] [data-ws-row]')].find(el => el.textContent?.includes('project'));
    if (row && !opened) {opened=true;row.click();}
  };
  const observer = new MutationObserver(enter);
  observer.observe(document.body, {childList:true,subtree:true});
  setTimeout(() => observer.disconnect(),15000);
}
if(runtime.mode==='frontend'&&query.has('handoff'))handOff('terminal.txt','$ example\nSynthetic terminal output');
Object.defineProperty(window, '__KHOR_LAB__', {value: Object.freeze(runtime)});
await import('../../src/main');

if(noticeDriver){const {mountNoticeTools}=await import('./notice-tools');mountNoticeTools(noticeDriver);}
