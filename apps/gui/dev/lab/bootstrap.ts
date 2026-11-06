import {createLabFixture} from './fixture';
import {scenes, sessionIds, type Scene} from './scenes';

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
if (runtime.mode === 'frontend' && native) throw new Error('Frontend fixtures cannot replace the native backend');
if (query.has('bridge')) throw new Error('Lab does not connect to external API bridges');
const scene = scenes[runtime.scene];
const items = Number(query.get('items') ?? 5), machines = Number(query.get('machines') ?? 3);
if (![2,3,5].includes(items) || ![0,1,3,6].includes(machines)) throw new Error('Lab data supports items=2/3/5 and machines=0/1/3/6');
const identity = `${runtime.tree}/${runtime.scene}/${runtime.revision}/${items}/${machines}`;
if (runtime.mode === 'frontend') {
  if (localStorage.getItem('khor.lab.identity') !== identity) {
    for (const key of Object.keys(localStorage)) if (key.startsWith('khor.')) localStorage.removeItem(key);
    localStorage.setItem('khor.lab.identity', identity);
    localStorage.setItem('khor.lang', 'zh');
    const active = scene.session ?? sessionIds[0];
    const tabs = [active,...sessionIds.filter(id => id !== active)].slice(0,items);
    localStorage.setItem('khor.sessions.tabs', JSON.stringify({tabs,active}));
    const landing = ['workspace','workspaces'].includes(runtime.scene) ? 'workspaces' : runtime.scene === 'devices' ? 'devices' : runtime.scene === 'browser' ? 'browser' : runtime.scene === 'quota' ? 'quota' : 'sessions';
    localStorage.setItem('khor.place', JSON.stringify({landing, screen: 'list'}));
  }
  const fixture = createLabFixture(true, {origin: location.origin, content: ['content', 'page'].includes(runtime.scene), items, machines, page:runtime.scene==='page'});
  const original = window.fetch.bind(window);
  window.fetch = async (input, init) => {
    const request = new Request(input, init);
    const url = new URL(request.url);
    if (url.origin !== location.origin || !url.pathname.startsWith('/api/')) return original(input, init);
    if (request.signal.aborted) throw new DOMException('Aborted', 'AbortError');
    try {
      const command = url.pathname.slice('/api/'.length);
      const answer = fixture.request(command, await request.json());
      // Polling fixtures must yield so production loops cannot starve rendering.
      if (['chat_wait', 'term_wait', 'term_stream', 'dirs_wait'].includes(command)) await new Promise(r => setTimeout(r, 120));
      return Response.json(answer);
    } catch (error) {
      console.error('[Lab fixture]', url.pathname, error);
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
    const row = [...document.querySelectorAll<HTMLElement>('[data-list] [data-row-open]')].find(el => el.textContent?.includes(scene.row!));
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
  let opened = false;
  const enter = () => {
    const file = document.querySelector<HTMLElement>('[data-file-path="/preview/project/README.md"]');
    if (file) {observer.disconnect();file.click();return;}
    const row = [...document.querySelectorAll<HTMLElement>('[data-list] [data-ws-row]')].find(el => el.textContent?.includes('project'));
    if (row && !opened) {opened=true;row.click();}
  };
  const observer = new MutationObserver(enter);
  observer.observe(document.body, {childList:true,subtree:true});
  setTimeout(() => observer.disconnect(),15000);
}
Object.defineProperty(window, '__KHOR_LAB__', {value: Object.freeze(runtime)});
await import('../../src/main');
