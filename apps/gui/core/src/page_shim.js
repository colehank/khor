// The page shim: what a borrowed tab takes away from the site it opens.
//
// Injected at document start, in every frame, of every page webview a
// browsing tab builds (`apps/gui/src-tauri/src/lib.rs`'s `open_page`) —
// and nowhere else. khor's own face never runs this.
//
// # Why anything is taken away
//
// A borrowed tab loads its page through another machine's network: the
// webview's proxy is a local port tunnelled to the exit machine, and the
// name is resolved *there* (`crates/node/src/tunnel.rs`'s `serve_stream`
// connects at the far end). That holds for everything the page fetches —
// except for a few APIs that WebKit sends over the device's own network
// instead.
//
// **They do not fail. They succeed, through the wrong network**, which
// is the exact failure shape docs/NET.md rules out for the exit machine
// («一档接不上不会变成「没有值」,它会变成旁边那一档»). A page that came
// in through Tokyo, quietly saying where the device really is.
//
// Three of them, reported by Mysk on 2026-08-04, all unfixed at the time
// of that report. Two are taken away here; the third cannot be, and that
// is written down below rather than left for someone to discover.
//
// # What is taken away, and why removal rather than refusal
//
// Both are **deleted**, not replaced with something that throws. A site
// that feature-detects (`if (window.PublicKeyCredential)`) then takes
// its other path — a password form, a WebSocket — which is a working
// page. A shim that lets the site start and then fails it mid-flight is
// a broken page instead, and the site cannot tell why.
//
//   - `WebTransport` (iOS 26.4+ per that report). It speaks QUIC over
//     UDP, and an HTTP proxy carries no UDP; the datagrams leave from
//     this device. Deleted, so a site uses its WebSocket path.
//   - `PublicKeyCredential` (iOS 18+). Answering a WebAuthn call makes
//     WebKit fetch the relying party's `.well-known/webauthn` **outside
//     the proxy**. Deleted, and `credentials.get`/`create` refuse the
//     `publicKey` branch as well — a site that skips the detection and
//     calls anyway gets `NotAllowedError`, the same name it gets when a
//     person declines the prompt, so its own cancel path runs.
//
// **Passkeys therefore do not work in a borrowed tab.** That is the
// price and it is the right way round: a passkey login that works while
// reporting this device's address is worth less than a password login
// that does not.
//
// # ⚠️ The third leak is not blocked, and not blockable from here
//
// `<link rel="dns-prefetch">` (iOS 26+) resolves through the device's
// own resolver. **This shim cannot stop it and neither can any other
// script.** WebKit acts on the element the instant it is inserted,
// synchronously, inside the parser — a `MutationObserver` callback is a
// microtask later and the lookup is already gone, and patching
// `setAttribute` only reaches the elements a script inserts, which is
// the smaller half. There is no earlier hook.
//
// What escapes is the **hostname**, not the traffic: which sites this
// device is looking at, and — on a campus or home network — the names of
// internal hosts, handed to whatever resolver this device uses. Closing
// it needs a layer that sees the HTML before WebKit parses it (the
// gateway shape in docs/handoff/001.md), or a WebKit fix.
//
// # ⚠️ What the two above cannot block either
//
// **A page that is trying.** Measured on 2026-09-10 by `npm run wk`,
// which loads the same page twice in a real `WKWebView`, once without
// this file. The script reaches the page, an `about:blank` iframe the
// parser made, a real child frame, an empty iframe a script made, and a
// `srcdoc` iframe once it has loaded — all five answer `undefined` for
// both globals, and all five answer `function` without the shim.
//
// Two realms it does not reach, and both are real ways out:
//
//   - **Workers.** A user script is injected into documents, not into
//     `Worker`/`SharedWorker` globals; measured `typeof WebTransport`
//     there is `function`, shim or no shim. Patching `Worker` itself
//     would break ordinary sites.
//   - **A `srcdoc` iframe read before its load event.** Setting
//     `srcdoc` and appending gives a document the script has not run in
//     yet, and `iframe.contentWindow.WebTransport` is a `function` in
//     that window. After the load event the same frame answers
//     `undefined`.
//
// ⚠️ **The guess this replaced was wrong in a way worth keeping.** The
// note here used to say the open question was `about:blank` — and
// `about:blank` turns out to be covered, both when the parser makes it
// and when a script does. Following that guess would have meant
// hardening a realm that was fine while the two above stayed open.
//
// So this blocks **the leak a site walks into**, not a site that is
// deliberately trying to find out where you are. Saying otherwise would
// be the more expensive kind of wrong.
//
// # No mark is left on the page
//
// Deliberately nothing — no `window.__khor`, no flag. A shim that
// announces itself hands every site a fingerprint that says *this
// client is khor*, which helps precisely the party this exists to stop.
// The observable fact is the absence itself.
(() => {
  "use strict";
  const w = globalThis;

  // Removal is best-effort by construction: on a realm where these are
  // non-configurable the delete is a no-op, and a shim that threw here
  // would take the page down over a leak it merely failed to close.
  try {
    delete w.WebTransport;
  } catch {
    /* left in place; see the note above on what this cannot block */
  }
  try {
    delete w.PublicKeyCredential;
  } catch {
    /* same */
  }

  const why =
    "khor: WebAuthn is off in a borrowed tab — answering it makes WebKit " +
    "fetch the relying party's .well-known outside the borrow, which would " +
    "report this device's real address while the page came in through another.";

  // The `publicKey` branch only. `credentials.get({password})` and the
  // federated branch never touch `.well-known` and are left alone — a
  // shim that refused every credential call would be a shim that cannot
  // tell the leak from the rest of the API.
  const box = w.CredentialsContainer && w.CredentialsContainer.prototype;
  if (box) {
    for (const name of ["get", "create"]) {
      const was = box[name];
      if (typeof was !== "function") continue;
      Object.defineProperty(box, name, {
        configurable: true,
        writable: true,
        value: function (options) {
          if (options && options.publicKey) {
            return Promise.reject(new w.DOMException(why, "NotAllowedError"));
          }
          return was.apply(this, arguments);
        },
      });
    }
  }
})();
