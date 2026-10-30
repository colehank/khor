// Fixed, already-projected scenario DTOs only. No vendor normalization, body
// folding, source identity algorithm, reader or cache replaces production code.
const BODY_NOTES = new Set(['user_message_chunk', 'agent_message_chunk', 'agent_thought_chunk', 'tool_call', 'tool_call_update']);
const COMMANDS = new Set(['attachment_begin', 'chat_open', 'chat_capture', 'chat_poll', 'chat_wait', 'chat_leave', 'history_begin', 'history_page', 'history_cancel']);

export function canonicalChatFixture({ids, takeFrames, onOpen = () => {}, bindingPrefix = 'fixed-fixture'}) {
  const known = new Set(ids);
  const scenarios = new Map(), views = new Map(), reads = new Map();
  let serial = 0;
  const key = () => (++serial).toString(16).padStart(32, '0');
  const scenario = id => {
    if (!scenarios.has(id)) scenarios.set(id, {binding: `${bindingPrefix}:${id}`, entries: [], pending: [], cursor: 0});
    return scenarios.get(id);
  };
  const cursor = offset => `fixture-offset:${offset}`;
  const offset = (value, fallback) => {
    if (value === null) return fallback;
    if (typeof value !== 'string' || !/^fixture-offset:\d+$/.test(value)) throw new Error('not a fixture cursor');
    const number = Number(value.slice('fixture-offset:'.length));
    if (!Number.isSafeInteger(number)) throw new Error('fixture cursor overflow');
    return number;
  };
  const flush = id => {
    const state = scenario(id);
    for (const frame of takeFrames(id)) {
      if (frame.kind === 'history' || frame.kind === 'history_end') throw new Error('retired History DTO in current fixture');
      // These five source facts are explicit in these scenarios. Original SDK
      // payloads stay untouched; real paintHistory performs all body folding.
      if (frame.kind === 'note' && BODY_NOTES.has(frame.update?.update?.sessionUpdate)) {
        state.entries.push({key: `${state.entries.length}:0`, frame});
        state.pending.push({kind: 'content_changed'});
      } else {
        if (frame.kind === 'turn') state.entries.push({key: `${state.entries.length}:0`, frame});
        state.pending.push(frame);
      }
    }
    return state;
  };
  const view = args => {
    const owner = views.get(args.lease);
    if (!owner || owner.id !== args.id || !owner.claimed || owner.closed) throw new Error('invalid fixture attachment');
    return owner;
  };
  return {
    handles: command => COMMANDS.has(command),
    answer(command, args) {
      if (!COMMANDS.has(command) || !known.has(args.id)) return null;
      if (command === 'attachment_begin') {
        if (args.kind !== 'chat') return null;
        if (typeof args.id !== 'string' || !args.id) throw new Error('fixture attachment requires id');
        const lease = key();views.set(lease, {id: args.id, claimed: false, closed: false});
        return {value: lease};
      }
      if (command === 'chat_open') {
        const owner = views.get(args.lease);
        if (!owner || owner.id !== args.id || owner.claimed || owner.closed) throw new Error('fixture claim is not pending');
        owner.claimed = true;onOpen(args.id);return {value: true};
      }
      if (command === 'chat_leave') {
        const owner = views.get(args.lease);
        if (!owner || owner.id !== args.id) throw new Error('fixture leave differs');
        owner.closed = true;return {value: null};
      }
      if (command.startsWith('chat_')) {
        view(args);
        const state = flush(args.id);
        if (command === 'chat_capture') {
          if (typeof args.token !== 'string' || !/^[a-f0-9]{32}$/i.test(args.token)) throw new Error('invalid fixture capture token');
          state.pending.push({kind: 'captured', token: args.token, result: {kind: 'available', binding: state.binding, cursor: cursor(state.entries.length)}});
          return {value: null};
        }
        const frames = state.pending.splice(0);state.cursor += frames.length;
        return {value: {frames, next: state.cursor, gone: false, lost: false, sending: 0, sending_of: 0, stall: null}};
      }
      const state = scenario(args.id);
      if (state.binding !== args.binding) throw new Error('fixture binding differs');
      if (command === 'history_begin') {
        const request = key();reads.set(request, {id: args.id, binding: args.binding, claimed: false, stopped: false});return {value: request};
      }
      const read = reads.get(args.requestKey);
      if (command === 'history_cancel') {
        if (read) {
          if (read.id !== args.id || read.binding !== args.binding) throw new Error('fixture cancel differs');
          read.stopped = true;if (!read.claimed) reads.delete(args.requestKey);
        }
        return {value: null};
      }
      if (!read || read.id !== args.id || read.binding !== args.binding || read.claimed || read.stopped) throw new Error('invalid fixture read claim');
      read.claimed = true;
      const finish = () => reads.delete(args.requestKey);
      try {
        const at = offset(args.cursor, state.entries.length);
        if (at < 0 || at > state.entries.length) throw new Error('fixture cursor outside static entries');
        const start = args.newer ? at : Math.max(0, at - 128);
        const end = args.newer ? Math.min(state.entries.length, at + 128) : at;
        const value = {entries: state.entries.slice(start, end), references: [], older: start ? cursor(start) : null,
          newer: cursor(end), has_more: !!args.newer && end < state.entries.length, reset: false, encoded_bytes: 0};
        value.encoded_bytes = new TextEncoder().encode(JSON.stringify(value)).length + 32;
        if (value.encoded_bytes > 512 * 1024) throw new Error('static fixture projection exceeds page budget');
        return {value, finish};
      } catch (error) {finish();throw error;}
    },
  };
}

export async function routeCanonicalChat(page, options) {
  const fixture = canonicalChatFixture(options);
  await page.route(/\/(attachment_begin|chat_open|chat_capture|chat_poll|chat_wait|chat_leave|history_begin|history_page|history_cancel)(?:\?.*)?$/, async route => {
    const command = new URL(route.request().url()).pathname.split('/').at(-1);
    const args = route.request().postDataJSON() ?? {};
    let reply;
    try {
      reply = fixture.answer(command, args);
      if (reply === null) return await route.fallback();
      await route.fulfill({contentType: 'application/json', body: JSON.stringify(reply.value)});
    } finally {
      // Cancel never releases a claimed reply early; this same reply's actual
      // fulfill settlement retires its request. There is no replacement worker.
      reply?.finish?.();
    }
  });
  return fixture;
}
