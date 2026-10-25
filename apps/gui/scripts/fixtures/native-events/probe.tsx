import React, {useEffect} from 'react';
import {createRoot} from 'react-dom/client';
import {flushSync} from 'react-dom';
import {listen} from '@tauri-apps/api/event';
import {hear} from '/src/api';

const event = 'q043:ownership';
const root = createRoot(document.getElementById('root')!);
const received: string[] = [];
const payloads: unknown[] = [];
const errors: string[] = [];
const closes = new Map<string, () => void>();
let anchorClose: (() => void) | null = null;
addEventListener('unhandledrejection', e => errors.push(String(e.reason)));
function Owner({id}: {id: string}) {
  useEffect(() => {
    const close = hear(event, payload => { received.push(id); payloads.push(payload); });
    closes.set(id, close);
    return close;
  }, [id]);
  return null;
}
const wire = () => window.__eventWire;
const register = (index: number, id: number) => {
  const message = wire().messages[index];
  const values = {listeners_object_name: '__internal_unstable_listeners_object_id__', serialized_target: JSON.stringify(message.payload.target), event, event_id: id, handler_id: message.payload.handler};
  (0, eval)(window.__eventRegisterTemplate.replace(/\{(listeners_object_name|serialized_target|event|event_id|handler_id)\}/g, (_, key) => values[key]));
};
const reply = (index: number, value: unknown, fail = false) => {
  const message = wire().messages[index];
  window.__TAURI_INTERNALS__.runCallback(fail ? message.error : message.callback, value);
};
window.__eventProbe = {
  async anchor() {
    const pending = listen(event, () => {});
    const index = wire().messages.length - 1;
    wire().native.set(1, wire().messages[index].payload.handler);
    register(index, 1); reply(index, 1);
    anchorClose = await pending;
  },
  mount(ids: string[]) { flushSync(() => root.render(ids.map(id => <Owner key={id} id={id} />))); },
  close(id: string) { closes.get(id)?.(); },
  accept(index: number, id: number, evalFirst = false) {
    wire().native.set(id, wire().messages[index].payload.handler);
    if (evalFirst) register(index, id);
    reply(index, id);
  },
  reject(index: number) { reply(index, 'fixture listen denied', true); },
  register,
  emit(ids: number[]) { window.__internal_unstable_listeners_function_id__({event, payload: null}, ids); },
  settleUnlisten(fail = false) {
    for (const [index, message] of wire().messages.entries()) {
      if (message.cmd !== 'plugin:event|unlisten' || message.settled) continue;
      message.settled = true;
      if (!fail) wire().native.delete(message.payload.eventId);
      reply(index, fail ? 'fixture unlisten denied' : null, fail);
    }
  },
  closeAnchor() { anchorClose?.(); anchorClose = null; },
  state() {
    return {
      received, payloads, errors, callbacks: window.__TAURI_INTERNALS__?.callbacks?.size ?? 0,
      native: wire() ? [...wire().native.keys()] : [],
      messages: wire()?.messages.map(message => ({cmd: message.cmd, handler: message.payload.handler, id: message.payload.eventId, target: message.payload.target})) ?? [],
      handlers: wire()?.messages.filter(message => message.cmd === 'plugin:event|listen').map(message => window.__TAURI_INTERNALS__.callbacks.has(message.payload.handler)) ?? [],
    };
  },
};
window.__eventReady = true;
