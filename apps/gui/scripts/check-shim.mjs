// The shim gate: **the two APIs a borrowed page must not have, and the
// one call it must still be allowed to make.**
//
// `apps/gui/core/src/page_shim.js` runs at document start in every
// borrowed page. What it takes away is not a feature decision — on
// WebKit, `WebTransport` and WebAuthn leave through the device's own
// network rather than through the borrow, so a page loaded via Tokyo
// quietly reports where the device really is. That failure **does not
// look like a failure**: nothing errors, the page works, and the only
// observable difference is an address in somebody else's log.
//
// Which is exactly why it needs a gate. A shim that stopped working
// would break nothing visible, ever.
//
// # What is asserted, and the arm that keeps it honest
//
// Three removals and **one call that must survive**:
// `credentials.get({password})` never touches `.well-known` and must
// still reach the real implementation. Without that arm, a shim that
// refused every credential call would pass — and 「refuses everything」
// and 「refuses the leak」 produce identical greens on the other three.
//
// # ⚠️ What this gate cannot tell you
//
// **Whether the shim is what a real page actually gets.** It runs the
// script in a `vm` realm furnished by this file, so it answers 「the
// rules are written correctly」. Two things it does not answer:
//
//   - **Whether WebKit hands the page this script at all.** The nearest
//     reachable evidence is the call site itself, asserted below by
//     reading the source — which catches the line being deleted and
//     nothing subtler. A native child webview is out of reach of every
//     browser gate in this repo (`check-face` drives the web face, in
//     Chrome); measuring the real thing means running the app.
//   - **Whether `delete window.WebTransport` even takes on WebKit.**
//     Here the globals are ordinary configurable properties, which is
//     what WebIDL says an interface object is — but that is this file
//     agreeing with my reading of a spec, not with WebKit. Same for
//     whether a user script reaches an `about:blank` iframe.
//
// Both are **未量**, and neither is a thing this gate can grow into.
//
// Run: `node scripts/check-shim.mjs` (static, no browser, no cargo lock).
import { readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import vm from "node:vm";

const repo = new URL("../../../", import.meta.url);
const at = (rel) => fileURLToPath(new URL(rel, repo));

let bad = 0;
const fail = (m) => {
  console.error(m);
  bad += 1;
};

const SHIM = at("apps/gui/core/src/page_shim.js");
const source = readFileSync(SHIM, "utf8");

/**
 * A realm holding the three things the shim reaches for, furnished the
 * way WebIDL describes them: interface objects are configurable, and
 * `get`/`create` live on `CredentialsContainer.prototype`.
 *
 * `reached` records every call that got through to a real
 * implementation — the only way to tell 「allowed」 from 「refused with
 * a resolved promise」.
 */
function realm() {
  const reached = [];
  const container = class CredentialsContainer {};
  container.prototype.get = function (options) {
    reached.push(["get", options]);
    return Promise.resolve("the real answer");
  };
  container.prototype.create = function (options) {
    reached.push(["create", options]);
    return Promise.resolve("the real answer");
  };
  const world = {
    WebTransport: class WebTransport {},
    PublicKeyCredential: class PublicKeyCredential {},
    CredentialsContainer: container,
    DOMException,
    navigator: { credentials: new container() },
  };
  world.navigator.credentials = Object.create(container.prototype);
  vm.createContext(world);
  return { world, reached };
}

/** What a promise did, as one word plus the detail worth printing. */
async function settled(p) {
  try {
    return { how: "kept", value: await p };
  } catch (e) {
    return { how: "refused", name: e && e.name, message: e && e.message };
  }
}

// ── The instrument's own arms, before it is trusted ──────────────────
//
// An unfurnished realm makes every assertion below pass by agreeing
// about nothing: if `WebTransport` was never there, the shim removing
// it is indistinguishable from the shim doing nothing at all. So the
// bare realm is checked for all three, and both credential calls are
// checked to reach the real implementation before anything patches it.
{
  const { world, reached } = realm();
  for (const name of ["WebTransport", "PublicKeyCredential"]) {
    if (typeof world[name] !== "function") {
      fail(`probe dead: a bare realm has no \`${name}\` to take away — every removal below would pass by removing nothing`);
    }
  }
  const before = await settled(world.navigator.credentials.get({ publicKey: {} }));
  if (before.how !== "kept" || reached.length !== 1) {
    fail(
      `probe dead: in a bare realm \`credentials.get({publicKey})\` answered ${JSON.stringify(before)} ` +
        `and reached the implementation ${reached.length} time(s) — the control arm below cannot mean anything`,
    );
  }
}

// ── The shim, run the way a page gets it ─────────────────────────────
const { world, reached } = realm();
vm.runInContext(source, world, { filename: "page_shim.js" });

for (const [name, why] of [
  ["WebTransport", "its QUIC goes out over UDP, which an HTTP proxy does not carry — the datagrams leave from this device"],
  ["PublicKeyCredential", "answering WebAuthn makes WebKit fetch the relying party's `.well-known` outside the borrow"],
]) {
  if (world[name] !== undefined) {
    fail(`\`${name}\` is still on a borrowed page (${typeof world[name]}) — ${why}, so the page came in through one network and says it is on another`);
  }
}

const pk = await settled(world.navigator.credentials.get({ publicKey: { challenge: 1 } }));
if (pk.how !== "refused" || pk.name !== "NotAllowedError") {
  fail(
    `\`credentials.get({publicKey})\` answered ${JSON.stringify(pk)} — it has to refuse with \`NotAllowedError\`, ` +
      `the name a site already handles as 「the person declined」, so its password path runs instead of a broken passkey flow`,
  );
}
const made = await settled(world.navigator.credentials.create({ publicKey: { challenge: 1 } }));
if (made.how !== "refused" || made.name !== "NotAllowedError") {
  fail(`\`credentials.create({publicKey})\` answered ${JSON.stringify(made)} — registering a passkey reaches the same \`.well-known\` fetch as using one`);
}

// **The arm that makes the three above mean something.** A password
// credential never reaches `.well-known`; if this one is refused too,
// the shim is not blocking a leak, it is blocking an API.
const pw = await settled(world.navigator.credentials.get({ password: true }));
if (pw.how !== "kept" || !reached.some(([verb, o]) => verb === "get" && o && o.password)) {
  fail(
    `\`credentials.get({password})\` answered ${JSON.stringify(pw)} and reached the implementation ` +
      `${reached.length} time(s) — a shim that refuses every credential call passes the three assertions above ` +
      `while telling you nothing about whether it can find the leak`,
  );
}

// ── Is anyone asking for it ──────────────────────────────────────────
//
// Separate from everything above, and weaker: the three assertions say
// the rules are right, this one says a webview is built with them. It
// reads source text, so it catches the call being deleted and not much
// else — but a shim nobody injects is the failure this whole file is
// about, and it would otherwise be invisible here.
const wired = [
  ["apps/gui/core/src/web.rs", 'include_str!("page_shim.js")', "the constant that carries the script"],
  [
    "apps/gui/src-tauri/src/lib.rs",
    ".initialization_script_for_all_frames(khor_gui_core::web::PAGE_SHIM)",
    "the page webview asking for it, in every frame rather than the main one",
  ],
];
for (const [rel, needle, what] of wired) {
  if (!readFileSync(at(rel), "utf8").includes(needle)) {
    fail(`${rel} no longer contains \`${needle}\` — that is ${what}; without it the shim is a file nothing reads and a borrowed page keeps both leaks`);
  }
}

console.log(`shim: ran ${SHIM.replace(fileURLToPath(repo), "")} in a furnished realm; checked ${wired.length} call sites by source text`);
if (!bad) {
  console.log("shim gate: WebTransport and PublicKeyCredential gone, publicKey refused as NotAllowedError, password still allowed");
  console.log("--- 未量, and not by this gate: whether WebKit delivers the script, whether the delete takes there, whether an about:blank iframe gets it ---");
}
process.exitCode = bad ? 1 : 0;
