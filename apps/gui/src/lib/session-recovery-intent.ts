// This device keeps one unresolved caller payload per execution target,
// at most 32 total. This is not a second execution ledger.
// Save before dispatch; ambiguous replies retry these exact bytes. Node owns
// whether the intent was accepted, executed or retired.
import type { RecoveryToken } from "@/api";
import { heardFrom, tellOthers } from "@/lib/other-windows";

const DATABASE = "khor-session-recovery";
const STORE = "caller";
const MAX_PENDING = 32;
const TOLD = "khor.told.session-recovery";
const CHANGED = "khor:session-recovery-intent";
const MAX_BYTES = 8 * 1024;
export type RecoveryIntent = {
  home: string;
  id: string;
  operation: "native" | "new_shell";
  expected_run: string | null;
  token: RecoveryToken;
  cwd: string | null;
};
export class RecoveryIntentError extends Error {
  constructor(readonly code: "storage" | "invalid" | "occupied" | "capacity") { super(code); }
}
const object = (v: unknown): v is Record<string, unknown> => !!v && typeof v === "object" && !Array.isArray(v);
const text = (v: unknown, cap: number): v is string => typeof v === "string" && v.length > 0 && v.length <= cap;
const nullable = (v: unknown, cap: number) => v === null || text(v, cap);
const sessionId = (v: unknown): v is string => text(v, 4096) && new TextEncoder().encode(v).byteLength <= 4096;
const keys = (v: Record<string, unknown>, names: string[]) => Object.keys(v).sort().join(",") === names.sort().join(",");

/** Bounds apply before JSON materialization. These are caller storage checks;
    epoch/nonce validity and all authority are checked again by Node. */
export function decodeRecoveryIntent(raw: string | null): RecoveryIntent | null {
  if (raw === null) return null;
  if (raw.length > MAX_BYTES || new TextEncoder().encode(raw).byteLength > MAX_BYTES) throw new RecoveryIntentError("invalid");
  let v: unknown;
  try { v = JSON.parse(raw); } catch { throw new RecoveryIntentError("invalid"); }
  if (!object(v) || !keys(v, ["home", "id", "operation", "expected_run", "token", "cwd"]) ||
      typeof v.home !== "string" || !/^[0-9a-f]{64}$/.test(v.home) || !sessionId(v.id) ||
      (v.operation !== "native" && v.operation !== "new_shell") || !nullable(v.expected_run, 1024) ||
      !nullable(v.cwd, 4096) || !object(v.token) || !keys(v.token, ["epoch", "nonce"]) ||
      typeof v.token.epoch !== "string" || !/^(0|[1-9][0-9]{0,19})$/.test(v.token.epoch) ||
      typeof v.token.nonce !== "string" || !/^[0-9a-f]{32}$/.test(v.token.nonce) ||
      (v.operation === "native" && v.cwd !== null) ||
      (v.operation === "new_shell" && v.expected_run !== v.id)) throw new RecoveryIntentError("invalid");
  return v as RecoveryIntent;
}
export function sameRecoveryIntent(a: RecoveryIntent, b: RecoveryIntent): boolean {
  return a.home === b.home && a.id === b.id && a.operation === b.operation &&
    a.expected_run === b.expected_run && a.cwd === b.cwd &&
    a.token.epoch === b.token.epoch && a.token.nonce === b.token.nonce;
}
function open(): Promise<IDBDatabase> {
  return new Promise((resolve, reject) => {
    let refused = false;
    const request = indexedDB.open(DATABASE, 1);
    const no = () => { refused = true; reject(new RecoveryIntentError("storage")); };
    request.onupgradeneeded = () => request.result.createObjectStore(STORE);
    request.onerror = no;
    request.onblocked = no;
    request.onsuccess = () => {
      if (refused) { request.result.close(); return; }
      request.result.onversionchange = () => request.result.close();
      resolve(request.result);
    };
  });
}
function target(home: string, id: string): [string, string] {
  if (!/^[0-9a-f]{64}$/.test(home) || !sessionId(id)) throw new RecoveryIntentError("invalid");
  return [home, id];
}
/** One unresolved payload per compound target key. Native readwrite transactions
    serialize only the small read/modify step across windows, never the RPC. */
async function edit(home: string, id: string, mode: IDBTransactionMode, change: (old: RecoveryIntent | null, store: IDBObjectStore, fail: (error: unknown) => void) => RecoveryIntent | null): Promise<RecoveryIntent | null> {
  const key = target(home, id);
  const db = await open().catch(() => { throw new RecoveryIntentError("storage"); });
  try {
    return await new Promise((resolve, reject) => {
      const tx = db.transaction(STORE, mode);
      const store = tx.objectStore(STORE);
      const read = store.get(key);
      let answer: RecoveryIntent | null = null;
      let failure: unknown;
      const fail = (error: unknown) => { failure = error; tx.abort(); };
      read.onsuccess = () => {
        try {
          if (read.result !== undefined && typeof read.result !== "string") throw new RecoveryIntentError("invalid");
          const old = decodeRecoveryIntent(read.result ?? null);
          if (old && (old.home !== home || old.id !== id)) throw new RecoveryIntentError("invalid");
          answer = change(old, store, fail);
        } catch (error) { fail(error); }
      };
      tx.oncomplete = () => resolve(answer);
      tx.onabort = () => reject(failure ?? new RecoveryIntentError("storage"));
      tx.onerror = () => { failure ??= new RecoveryIntentError("storage"); };
    });
  } finally { db.close(); }
}
export function readRecoveryIntent(home: string, id: string): Promise<RecoveryIntent | null> {
  return edit(home, id, "readonly", old => old);
}
function changed() { window.dispatchEvent(new Event(CHANGED)); tellOthers(TOLD); }
export async function rememberRecoveryIntent(intent: RecoveryIntent): Promise<void> {
  const raw = JSON.stringify(intent);
  decodeRecoveryIntent(raw);
  await edit(intent.home, intent.id, "readwrite", (old, store, fail) => {
    if (old && !sameRecoveryIntent(old, intent)) throw new RecoveryIntentError("occupied");
    const key = target(intent.home, intent.id);
    if (old) store.put(raw, key);
    else {
      const count = store.count();
      count.onsuccess = () => {
        try {
          if (count.result >= MAX_PENDING) { fail(new RecoveryIntentError("capacity")); return; }
          store.put(raw, key);
        } catch (error) { fail(error); }
      };
    }
    return intent;
  });
  changed();
}
/** A delayed reply cannot clear a newer caller payload. */
export async function settleRecoveryIntent(intent: RecoveryIntent): Promise<void> {
  await edit(intent.home, intent.id, "readwrite", (old, store) => {
    if (old && sameRecoveryIntent(old, intent)) { store.delete(target(intent.home, intent.id)); return null; }
    return old;
  });
  changed();
}
export function hearRecoveryIntent(run: () => void): () => void {
  window.addEventListener(CHANGED, run);
  const drop = heardFrom(TOLD, run);
  return () => { window.removeEventListener(CHANGED, run); drop(); };
}
