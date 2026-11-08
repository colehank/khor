// Fixed, already-projected scenario DTOs only. No vendor normalization, body
// folding, source identity algorithm, reader or cache replaces production code.
const BODY_NOTES = new Set(['user_message_chunk', 'agent_message_chunk', 'agent_thought_chunk', 'tool_call', 'tool_call_update','plan','compaction_update','compaction_summary_chunk']);
// Match gui-core frame_of: these five become ContentChanged. Plans and
// compaction facts are also recorded, but remain actual live Note receipts.
const PROMOTED_NOTES=new Set(['user_message_chunk','agent_message_chunk','agent_thought_chunk','tool_call','tool_call_update']);
const COMMANDS = new Set(['attachment_begin', 'chat_open', 'chat_capture', 'chat_poll', 'chat_wait', 'chat_leave', 'history_begin', 'history_page', 'history_cancel']);

export function canonicalChatFixture({ids, takeFrames, onOpen = (_id, _lease) => {}, bindingPrefix = 'fixed-fixture',pageSize=128}) {
  const known = new Set(ids);
  const scenarios = new Map(), views = new Map(), reads = new Map();
  let serial = 0;
  const key = () => (++serial).toString(16).padStart(32, '0');
  const scenario = id => {
    if (!scenarios.has(id)) scenarios.set(id, {binding: `${bindingPrefix}:${id}`, entries: [], pending: [], cursor: 0, opened:false, snapshots:new Map()});
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
      const update=frame.kind==='note'?frame.update?.update:null;
      const fact=['agent','settings','capabilities','turning'].includes(frame.kind)?frame.kind
        :['available_commands_update','usage_update','current_mode_update'].includes(update?.sessionUpdate)?update.sessionUpdate:null;
      if(fact)state.snapshots.set(fact,structuredClone(frame));
      if(frame.kind==='turn'||frame.kind==='gone')state.snapshots.delete('turning');
      if(frame.kind==='ask'||frame.kind==='elicit')state.snapshots.set(`${frame.kind}:${frame.ask}`,structuredClone(frame));
      if(frame.kind==='answered'||frame.kind==='elicit_answered'){
        state.snapshots.delete(`ask:${frame.ask}`);state.snapshots.delete(`elicit:${frame.ask}`);
      }
      if (frame.kind === 'history' || frame.kind === 'history_end') throw new Error('retired History DTO in current fixture');
      // These five source facts are explicit in these scenarios. Original SDK
      // payloads stay untouched; real paintHistory performs all body folding.
      if (frame.kind === 'note' && BODY_NOTES.has(frame.update?.update?.sessionUpdate)) {
        state.entries.push({key: `${state.entries.length}:0`, starts_message: false, record: null, frame});
        state.pending.push(PROMOTED_NOTES.has(frame.update.update.sessionUpdate)?{kind:'content_changed'}:frame);
      } else {
        if (frame.kind === 'turn') state.entries.push({key: `${state.entries.length}:0`, starts_message: true, record: null, frame});
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
        owner.claimed = true;onOpen(args.id, args.lease);const state=flush(args.id),fresh=!state.opened;state.opened=true;
        if(!fresh)state.pending.push(...structuredClone([...state.snapshots.values()]));return {value:fresh};
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
        const start = args.newer ? at : Math.max(0, at - pageSize);
        const end = args.newer ? Math.min(state.entries.length, at + pageSize) : at;
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
