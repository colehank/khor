import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFileSync} from 'node:fs';

const upstream = new URL('./tauri-2.11.5/', import.meta.url);
const hashes = {
  'core.js': '71056442a732cca1f03572ccae96d68e16104c3a591c64de9618cd304e063070',
  'event-mod.rs': 'dfcb7cf11d7674903f71196e64ae0fa95286757ac816bfbc3139ebdedcb25dea',
  'event-init.js': '8946488895cc9879f2bf8fee98be454d0f21949b946c264b47d2a2fc84eb1df0',
};

export function nativeEventRuntime(gui) {
  const lock = JSON.parse(readFileSync(new URL('package-lock.json', gui)));
  assert.equal(lock.packages['node_modules/@tauri-apps/api'].version, '2.11.1', 'Review the native-event SDK contract on upgrade');
  const cargo = readFileSync(new URL('../../Cargo.lock', gui), 'utf8');
  assert.match(cargo, /\[\[package\]\]\nname = "tauri"\nversion = "2\.11\.5"\n/, 'Review the native-event runtime contract on upgrade');
  const event = readFileSync(new URL('node_modules/@tauri-apps/api/event.js', gui));
  assert.equal(createHash('sha256').update(event).digest('hex'), 'f00d22c559ce7e58a85d860e35fd60d20826f13d7f534a5f3548a7ef94446e84', 'Use the genuine locked SDK, not an event mock');
  const coreApi = readFileSync(new URL('node_modules/@tauri-apps/api/core.js', gui));
  assert.equal(createHash('sha256').update(coreApi).digest('hex'), 'b2187a1c0c0a25806dc64f8823757ce553c09d3bbe3e409bd05cab32e245f1c9', 'Review the SDK core delegation on upgrade');
  const helperSdk = {
    'window.js': '2b7c5d852dd6ea216d3eef7eed7dbe3000232b3359ec47aa8809945f889104d9',
    'webview.js': '29b803a82322bac10e08a809f257725367ce9200f251563c73b7113722db1c46',
    'dpi.js': '69920668363f105d89b9fbad7db499271fd95b93feeeab84387d43319fbc8f77',
  };
  for (const [name, expected] of Object.entries(helperSdk)) {
    const actual = readFileSync(new URL(`node_modules/@tauri-apps/api/${name}`, gui));
    assert.equal(createHash('sha256').update(actual).digest('hex'), expected, `Review the genuine SDK ${name} helper semantics on upgrade`);
  }
  const sources = Object.fromEntries(Object.entries(hashes).map(([file, hash]) => {
    const content = readFileSync(new URL(file, upstream));
    assert.equal(createHash('sha256').update(content).digest('hex'), hash, file);
    return [file, content.toString()];
  }));
  const literal = name => {
    const body = sources['event-mod.rs'].split(`fn ${name}(`)[1]?.split('\n}\n')[0];
    const match = body?.match(/format!\(\s*"([\s\S]*?)"\s*,?\s*\)/);
    assert(match && !match[1].includes('\\'), `Unsupported upstream Rust format string: ${name}`);
    return match[1].replaceAll('{{', '{').replaceAll('}}', '}');
  };
  const unlisten = literal('unlisten_js_script')
    .replaceAll('{listeners_object_name}', '__internal_unstable_listeners_object_id__')
    .replaceAll('{event_arg}', 'event').replaceAll('{event_id_arg}', 'eventId');
  const initialization = sources['event-init.js'].replace('__RAW_unregister_listener_function__', `(event, eventId) => ${unlisten}`);
  const dispatch = literal('event_initialization_script')
    .replaceAll('{function_name}', '__internal_unstable_listeners_function_id__')
    .replaceAll('{listeners}', '__internal_unstable_listeners_object_id__');
  const core = sources['core.js'].replace('__TEMPLATE_os_name__', '"ios"').replace('__TEMPLATE_protocol_scheme__', '"http"');
  // Only native IPC delivery is controlled. Core callback storage, promises, registry and SDK stay genuine.
  return `if (!new URL(location.href).searchParams.has('web')) {
    window.__TAURI_INTERNALS__ = {};
    window.__eventWire = {messages: [], native: new Map()};
    Object.defineProperty(window.__TAURI_INTERNALS__, 'ipc', {value: message => window.__eventWire.messages.push(message)});
    ${core}
    ${initialization}
    ${dispatch}
    window.__eventRegisterTemplate = ${JSON.stringify(literal('listen_js_script'))};
  }`;
}
