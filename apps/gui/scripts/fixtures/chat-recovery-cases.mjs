import assert from 'node:assert/strict';
import {recoveryActionCases, exerciseRecoveryActions} from './recovery-actions-ui.mjs';

// User-visible capabilities retained from the retired History/HistoryEnd suite.
export const chatRecoveryCases = [
  'failed-empty', 'failed-partial', 'cancelled-delivery', 'empty-error',
  'successful-empty', 'failure-then-success', 'live-interleaved',
  'ordinary-control-error', 'successful-result', 'gone-reconnect', 'gone-flag-reconnect',
  'retry-mid-capture', 'successful-capture-before-gone',
];

// Transport facts only: real ChatView, Capture, source, reader and paint execute.
export function installRecoveryTransport() {
  const empty = {frames: [], next: 0, gone: false, lost: false, sending: 0, sending_of: 0, stall: null};
  const entry = (binding, index, text) => ({key: `${binding}:${index}:0`, starts_message:false, record:null, frame: {kind: 'note', update: {update: {
    sessionUpdate: 'agent_message_chunk', content: {type: 'text', text},
  }}}});
  const sources = new Map([['canonical-1', ['Complete known history.']]]);
  const jobs = new Map();
  let binding = 'canonical-1', serial = 0;
  const cursor = (source, offset) => JSON.stringify([source, offset]);
  const wire = window.__chatWire = {
    calls: [], pending: [], held: [], unknown: [], cancellations: [], failNext: null, holdNext: false,
    reply(index, change = {}) {
      const poll = wire.pending[index];
      if (!poll || poll.completed) throw new Error('missing or completed fixture poll');
      poll.completed = true; poll.resolve({...empty, ...change});
    },
    source(texts) {binding = `canonical-${sources.size + 1}`;sources.set(binding, [...texts]);},
    append(...texts) {sources.get(binding).push(...texts);},
    captured(token) {
      return {kind: 'captured', token, result: {kind: 'available', binding, cursor: cursor(binding, sources.get(binding).length)}};
    },
    release(index) {
      const job = wire.held[index];
      if (!job || job.released) throw new Error('missing or released fixture page');
      job.released = true; job.resolve(job.answer);
    },
    leases: () => [...jobs].map(([key, job]) => ({key, ...job})),
  };
  const invoke = async (cmd, args = {}) => {
    const call = {cmd, args, at: Date.now()}; wire.calls.push(call);
    const admission = window.__attachmentLeases(cmd, args);
    if (admission.handled) return admission.value;
    if (cmd === 'chat_open') return true;
    if (cmd === 'chat_capture') return null;
    if (cmd === 'chat_wait' || cmd === 'chat_poll') return new Promise((resolve, reject) => wire.pending.push({resolve, reject, call, completed: false}));
    if (cmd === 'devices' || cmd === 'workspaces') return [];
    if (cmd === 'plugin:event|listen') return 0;
    if (cmd === 'plugin:event|unlisten') return null;
    if (cmd === 'history_begin') {
      if (!sources.has(args.binding)) throw new Error('unknown canonical binding');
      const key = `read-${++serial}`; jobs.set(key, {binding: args.binding, claimed: false, stopped: false}); return key;
    }
    if (cmd === 'history_cancel') {
      wire.cancellations.push(args.requestKey);
      const job = jobs.get(args.requestKey);
      if (job) {job.stopped = true;if (!job.claimed) jobs.delete(args.requestKey);}
      return null;
    }
    if (cmd === 'history_page') {
      const job = jobs.get(args.requestKey);
      if (!job || job.claimed || job.stopped || job.binding !== args.binding) throw new Error('invalid or reused read lease');
      job.claimed = true;
      try {
        const selected = sources.get(args.binding);
        if (!selected) throw new Error('unknown canonical source');
        const [source, offset] = args.cursor === null ? [args.binding, selected.length] : JSON.parse(args.cursor);
        if (source !== args.binding || !Number.isInteger(offset) || offset < 0 || offset > selected.length) throw new Error('invalid source cursor');
        const texts = args.newer ? selected.slice(offset) : selected.slice(0, offset);
        const start = args.newer ? offset : 0;
        const answer = {entries: texts.map((text, index) => entry(args.binding, start + index, text)), references: [], older: null,
          newer: cursor(args.binding, args.newer ? selected.length : offset), has_more: false, reset: false, encoded_bytes: 0};
        answer.encoded_bytes = new TextEncoder().encode(JSON.stringify(answer)).length + 20;
        const failure = wire.failNext; wire.failNext = null;
        if (failure !== null) throw new Error(failure);
        if (wire.holdNext) {
          wire.holdNext = false;
          return await new Promise(resolve => wire.held.push({key: args.requestKey, answer, resolve, released: false}));
        }
        return answer;
      } finally {
        // A claimed request remains owned while held, including after Cancel.
        jobs.delete(args.requestKey);
      }
    }
    wire.unknown.push(cmd);throw new Error(`unknown recovery fixture API: ${cmd}`);
  };
  window.__TAURI_INTERNALS__ = {
    metadata: {currentWindow: {label: 'main'}, currentWebview: {label: 'main', windowLabel: 'main'}},
    transformCallback: () => 0, unregisterCallback: () => {}, convertFileSrc: path => path, invoke,
  };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = {unregisterListener: () => {}};
}

export async function exerciseChatRecovery(scenario, page, polls, preparePeer) {
  const wait = ms => new Promise(resolve => setTimeout(resolve, ms));
  const until = async (check, what) => {
    const end = Date.now() + 8000;
    while (!await check()) {assert(Date.now() < end, what);await wait(10);}
  };
  const calls = command => page.evaluate(command => window.__chatWire.calls.filter(call => call.cmd === command), command);
  const check = async (present, absent = []) => {
    await until(async () => {
      const drawn = await page.locator('[data-chat-column]').innerText();
      return present.every(text => drawn.includes(text)) && absent.every(text => !drawn.includes(text));
    }, `conversation retains ${present.join(', ')} and excludes ${absent.join(', ')}`);
  };
  const nextPoll = () => page.evaluate(() => {
    const wire = window.__chatWire, lease = wire.calls.findLast(call => call.cmd === 'chat_open')?.args.lease;
    return wire.pending.findIndex(poll => !poll.completed && poll.call.args.lease === lease);
  });
  const deliver = async (frames, change = {}, index) => {
    await until(async () => (index ?? await nextPoll()) >= 0, 'actual held poll ready');
    const selected = index ?? await nextPoll();
    await page.evaluate(({selected, frames, change}) => window.__chatWire.reply(selected, {frames, next: selected + 1, ...change}), {selected, frames, change});
    await wait(10);await page.clock.runFor(0);await wait(10);
  };
  const capture = async (extra = []) => {
    await until(async () => (await calls('chat_capture')).length > 0, 'actual view asks for Capture');
    const token = (await calls('chat_capture')).at(-1).args.token;
    const frame = await page.evaluate(token => window.__chatWire.captured(token), token);
    await deliver([frame, ...extra], {gone: extra.some(frame => frame.kind === 'gone')});
  };
  const wake = () => deliver([{kind: 'content_changed'}]);
  const mutate = texts => page.evaluate(texts => window.__chatWire.source(texts), texts);
  const recapture = async (texts, extra = []) => {
    await mutate(texts);
    const before = (await calls('chat_capture')).length;
    await deliver([], {lost: true});
    await until(async () => (await calls('chat_capture')).length === before + 1, 'lost control cursor requests a fresh Capture');
    await capture(extra);
  };
  const reconnect = async () => {
    const before = (await calls('chat_capture')).length;
    await page.evaluate(() => window.__chatReact.reload());
    await until(async () => (await calls('chat_capture')).length === before + 1, 'real tab action opens a new owner and captures');
  };
  const control = error => ({kind: 'control_result', token: 'fixture-control', operation: 'set_mode', data: null, error});

  // The entry case begins with an ended owner; Capture is possible only after Ready.
  if(scenario!=='automatic-entry-and-later-exit'){await capture();await check(['Complete known history.']);}
  if (recoveryActionCases.includes(scenario)) {
    await exerciseRecoveryActions(scenario,page,{calls,capture,check,preparePeer});
  } else if (scenario === 'failed-empty' || scenario === 'failed-partial' || scenario === 'empty-error') {
    if (scenario !== 'failed-empty') await page.evaluate(() => window.__chatWire.append('Uncommitted fragment.'));
    const before = (await calls('history_page')).length;
    await page.evaluate(scenario => {window.__chatWire.failNext = scenario === 'empty-error' ? '' : 'fixture read failure';}, scenario);
    await wake();await until(async () => (await calls('history_page')).length === before + 1, 'canonical read failure reached actual reader');
    await check(['Complete known history.'], ['Uncommitted fragment.']);
  } else if (scenario === 'cancelled-delivery') {
    await page.evaluate(() => {window.__chatWire.append('Uncommitted fragment.');window.__chatWire.holdNext = true;});
    await wake();await until(() => page.evaluate(() => window.__chatWire.held.length === 1), 'canonical read held');
    const key = await page.evaluate(() => window.__chatWire.held[0].key);
    await mutate(['Complete known history.']);await reconnect();await capture();
    await until(() => page.evaluate(key => window.__chatWire.cancellations.includes(key), key), 'retired reader cancelled its exact held read');
    assert(await page.evaluate(key => window.__chatWire.leases().some(job => job.key === key && job.stopped), key), 'cancel does not release the still-running read');
    await page.evaluate(() => window.__chatWire.release(0));
    await check(['Complete known history.'], ['Uncommitted fragment.']);
    await until(() => page.evaluate(key => !window.__chatWire.leases().some(job => job.key === key), key), 'IO completion drops its claimed lease');
  } else if (scenario === 'successful-empty') {
    await recapture([]);await check([], ['Complete known history.']);
  } else if (scenario === 'failure-then-success') {
    await page.evaluate(() => {window.__chatWire.append('Failed fragment.');window.__chatWire.failNext = 'fixture read failure';});
    const before = (await calls('history_page')).length;await wake();
    await until(async () => (await calls('history_page')).length === before + 1, 'first read actually failed');
    await check(['Complete known history.'], ['Failed fragment.']);
    await recapture(['Successful replacement.']);await check(['Successful replacement.'], ['Complete known history.', 'Failed fragment.']);
    await recapture([]);await check([], ['Successful replacement.']);
  } else if (scenario === 'live-interleaved') {
    await page.evaluate(() => {window.__chatWire.append('Live before failure.');window.__chatWire.failNext = 'fixture read failure';});
    const before = (await calls('history_page')).length;
    await deliver([{kind: 'note', update: {update: {sessionUpdate: 'agent_message_chunk', content: {type: 'text', text: 'Failed fragment.'}}}}, {kind: 'content_changed'}]);
    await until(async () => (await calls('history_page')).length === before + 1, 'failed canonical read reached reader');
    await page.evaluate(() => window.__chatWire.append('Live after failure.'));await wake();
    await check(['Complete known history.', 'Live before failure.', 'Live after failure.'], ['Failed fragment.']);
  } else if (scenario === 'ordinary-control-error' || scenario === 'successful-result') {
    await recapture(['Successful replacement.'], [control(scenario === 'ordinary-control-error' ? 'Mode denied' : null)]);
    await check(['Successful replacement.'], ['Complete known history.']);
  } else if (scenario === 'gone-reconnect' || scenario === 'gone-flag-reconnect') {
    await deliver(scenario === 'gone-reconnect' ? [{kind: 'gone'}] : [], {gone: true});
    await check(['Complete known history.']);
    await mutate(['New attachment history.']);await reconnect();await capture();
    await check(['New attachment history.'], ['Complete known history.']);
  } else if (scenario === 'retry-mid-capture') {
    const stale = await nextPoll(), before = (await calls('chat_capture')).length;
    const staleToken = (await calls('chat_capture')).at(-1).args.token;
    const staleLease = (await calls('chat_open')).at(-1).args.lease;
    const oldPolls = (await polls()).filter(call => call.args.lease === staleLease).length;
    await mutate(['New attachment history.']);await reconnect();
    const current = await page.evaluate(() => window.__chatWire.pending.findLastIndex(poll => !poll.completed));
    const frame = await page.evaluate(token => window.__chatWire.captured(token), (await calls('chat_capture')).at(-1).args.token);
    await deliver([frame], {}, current);await check(['New attachment history.'], ['Complete known history.']);
    const late = await page.evaluate(token => {
      window.__chatWire.source(['Late abandoned owner.']);return window.__chatWire.captured(token);
    }, staleToken);
    await deliver([late, control('late old owner')], {gone: true, lost: true}, stale);
    await page.clock.runFor(5000);
    assert.equal((await calls('chat_capture')).length, before + 1, 'old owner cannot ask for another capture');
    assert.equal((await polls()).filter(call => call.args.lease === staleLease).length, oldPolls, 'old lease never restarts polling');
    await check(['New attachment history.'], ['Complete known history.', 'Late abandoned owner.']);
  } else if (scenario === 'successful-capture-before-gone') {
    await recapture(['Completed before disconnect.'], [{kind: 'gone'}]);
    await check(['Completed before disconnect.'], ['Complete known history.']);
  } else throw new Error(`Missing current recovery case: ${scenario}`);

  assert.equal((await calls('chat_replay')).length, 0, 'no retired Replay API is invoked');
  assert.deepEqual(await page.evaluate(() => window.__chatWire.unknown), []);
  const leaveBefore=await page.evaluate(()=>({chat:window.__chatWire.calls.filter(c=>c.cmd==='chat_leave').length,terminal:window.__recoveryWire.calls.filter(c=>c.cmd==='term_leave').length,hasTerminal:!!document.querySelector('[data-kept-terminal][data-active] [data-terminal]'),hasChat:window.__chatWire.calls.some(open=>open.cmd==='chat_open'&&!window.__chatWire.calls.some(leave=>leave.cmd==='chat_leave'&&leave.args.lease===open.args.lease))}));
  await page.evaluate(() => window.__chatReact.close());
  await until(async () => (await calls('chat_leave')).length >= 1, 'actual component leaves its view');
  await until(()=>page.evaluate(before=>before.hasTerminal?window.__recoveryWire.calls.filter(c=>c.cmd==='term_leave').length>before.terminal:!before.hasChat||window.__chatWire.calls.filter(c=>c.cmd==='chat_leave').length>before.chat,leaveBefore),'current production view leaves its own admitted resource');
  await page.evaluate(()=>{const wire=window.__recoveryWire;for(let i=0;i<wire.offers.length;i++)if(!wire.offers[i].done)wire.releaseOffer(i,wire.offers[i].answer);});
  const actionsBefore=await page.evaluate(()=>window.__recoveryWire.calls.filter(c=>c.cmd==='recover_session'||c.cmd==='new_shell_run').length);
  const before = (await polls()).length;
  await page.clock.runFor(60000);await wait(20);
  assert.equal(await page.evaluate(()=>window.__recoveryWire.calls.filter(c=>c.cmd==='recover_session'||c.cmd==='new_shell_run').length),actionsBefore,'late read-only offers cannot revive execution after unmount');
  assert(await page.evaluate(()=>window.__recoveryWire.actions.every(io=>io.done)&&window.__recoveryWire.drops.every(io=>io.done)&&window.__recoveryWire.waits.every(io=>io.done)),'all controlled action/transfer/wait IO ended in its actual reply or leave');
  assert.equal((await polls()).length, before, 'retired component does not resume polling');
  assert.deepEqual(await page.evaluate(() => window.__chatWire.leases()), [], 'all actual fixture read owners ended');
}
