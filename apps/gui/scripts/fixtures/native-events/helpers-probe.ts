import { hear, onPageWord, onFullscreen, onQuitRequested } from '/src/api';
import { onWindowDrop } from '/src/lib/window-drop';
import * as owned from '/src/lib/native-events';
import { PhysicalPosition } from '@tauri-apps/api/dpi';
import { getCurrentWebview } from '@tauri-apps/api/webview';

const received: unknown[] = [], errors: string[] = [];
let close: (() => void) | undefined;
let ready = 0;
addEventListener('unhandledrejection', e => errors.push(String(e.reason)));
const wire = () => window.__eventWire;
let nextId = 1;
const register = (index: number, id: number) => {
  const message = wire().messages[index];
  const values = {listeners_object_name: '__internal_unstable_listeners_object_id__', serialized_target: JSON.stringify(message.payload.target), event: message.payload.event, event_id: id, handler_id: message.payload.handler};
  (0, eval)(window.__eventRegisterTemplate.replace(/\{(listeners_object_name|serialized_target|event|event_id|handler_id)\}/g, (_, key) => values[key]));
};
const reply = (index: number, value: unknown, fail = false) => {
  const message = wire().messages[index];
  message.settled = true;
  window.__TAURI_INTERNALS__.runCallback(fail ? message.error : message.callback, value);
};
const state = () => ({received, ready, errors, callbacks: window.__TAURI_INTERNALS__?.callbacks?.size ?? 0,
  native: [...wire().native.keys()], handlers: wire().messages.filter(m => m.cmd === 'plugin:event|listen').map(m => window.__TAURI_INTERNALS__.callbacks.has(m.payload.handler)),
  messages: wire().messages.map((m, index) => ({index, cmd: m.cmd, event: m.payload.event, target: m.payload.target, id: m.payload.eventId, settled: !!m.settled}))});
window.__helperProbe = {
  mount(mode: string) {
    if (mode === 'sdk-drag') { void getCurrentWebview().onDragDropEvent(event => received.push({ ...event.payload, physical: event.payload.type === 'leave' ? null : event.payload.position instanceof PhysicalPosition })).then(un => { close = un; }); }
    else if (mode === 'peer') {
      const one = hear('q043:helpers-peer', () => { throw new Error('fixture handler failed'); });
      const two = hear('q043:helpers-peer', () => received.push('peer'));
      close = () => { one(); two(); };
    }
    else if (mode.startsWith('page')) close = onPageWord('page:title', 'alpha', text => received.push(text), () => { ready++; if (mode === 'page-ready-close') close?.(); });
    else if (mode === 'fullscreen') close = onFullscreen(full => received.push(full));
    else if (mode.startsWith('quit')) close = onQuitRequested(() => {
      received.push('asked');
      if (mode === 'quit-close-in-handler') queueMicrotask(() => close?.());
      return mode === 'quit-prevent';
    });
    else if (mode === 'window-drop') close = onWindowDrop(drop => received.push(drop));
    else if (mode === 'drag') close = owned.listenNativeDragDrop(event => received.push({ ...event.payload, physical: event.payload.type === 'leave' ? null : event.payload.position instanceof PhysicalPosition }));
  },
  close() { close?.(); },
  accept(index: number, evalFirst = true) {
    const id = nextId++;
    wire().native.set(id, {handler: wire().messages[index].payload.handler, event: wire().messages[index].payload.event});
    if (evalFirst) register(index, id);
    reply(index, id); return id;
  },
  reject(index: number) { reply(index, 'fixture listen denied', true); },
  register,
  reply,
  emit(event: string, payload: unknown) {
    const ids = [...wire().native.entries()].filter(([, listener]) => listener.event === event).map(([id]) => id);
    window.__internal_unstable_listeners_function_id__({event, payload}, ids);
  },
  settleUnlisten(fail = false) {
    for (const [index, message] of wire().messages.entries()) {
      if (message.cmd !== 'plugin:event|unlisten' || message.settled) continue;
      if (!fail) wire().native.delete(message.payload.eventId);
      reply(index, fail ? 'fixture unlisten denied' : null, fail);
    }
  },
  state,
};
window.__helperReady = true;
