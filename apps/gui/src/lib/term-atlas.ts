// **The WebGL renderer and its glyph atlas, kept honest** (item 24; the
// owner, 2026-10-04: on entering a session every CJK run drew as fragments
// of other glyphs while ASCII was fine — Mac and iPhone).
//
// xterm's WebGL addon shares one glyph atlas among every terminal on the
// page whose font, size, theme and pixel ratio match, and 0.19.0 (the one
// this face pins) has three defects in it, each reported upstream:
//
// 1. **A clear on one terminal garbles the others** (xterm.js #6014, fixed
//    upstream in #6055 for 7.0): `TextureAtlas.clearTexture` empties the
//    shared pages but does not ask the other renderers to rebuild their
//    models, so they keep drawing from coordinates that now hold other
//    glyphs. This face cleared the atlas on every pane's mount (the
//    symbol face's "fonts are in" redraw), so entering a session garbled
//    every other open one — and coming back to one showed it.
// 2. **A repack leaves stale textures bound** (upstream commits 6fe6fe1,
//    3bcb575): the glyph renderer re-uploads a page only when its version
//    number changes, and after pages merge a new page can sit in a slot
//    with the old page's number.
// 3. **A font that arrives late** leaves rasters drawn with the stand-in.
//
// What this module does about them, without patching the package:
// - the atlas's `clearTexture` also sets the flag a new page sets, so every
//   renderer sharing it rebuilds its model on its next frame (#6055's
//   effect);
// - when the atlas removes a page (a merge or an eviction), every
//   renderer's page textures are marked stale, so all of them upload again;
// - when the page's fonts finish loading, every atlas is cleared and then
//   every terminal redrawn: all the clears first, so no clear lands after
//   a sibling rebuilt (the order the fix in hermes-agent #127052 keeps);
// - a lost WebGL context drops the pane to xterm's DOM renderer at once,
//   WebGL comes back the next time the page is shown, with the atlases
//   cleared, and a pane that loses it again within `LOST_AGAIN_MS` keeps
//   the DOM renderer and says so (`onFallback`).
//
// Internals reached (`_renderer`, `_charAtlas`, `_glyphRenderer`,
// `_requestClearModel`) are 0.19.0's own names, read defensively: a newer
// addon without them simply skips the guard.
import type { Terminal } from "@xterm/xterm";
import { WebglAddon } from "@xterm/addon-webgl";

/** A second context loss this soon after WebGL came back: the pane keeps
    the DOM renderer (ms). */
export const LOST_AGAIN_MS = 60_000;

type Atlas = { clearTexture: () => void; _requestClearModel?: boolean };
type Renderer = {
  _charAtlas?: Atlas;
  _glyphRenderer?: { value?: { setAtlas?: (a: Atlas) => void } };
};
type Held = { term: Terminal; addon: WebglAddon };

const held = new Set<Held>();
const guarded = new WeakSet<object>();
let fontsHeard = false;
let clearQueued = false;

function rendererOf(addon: WebglAddon): Renderer | null {
  return ((addon as unknown as { _renderer?: Renderer })._renderer ?? null) as Renderer | null;
}

/** #6055's effect, put on the atlas class once: a clear asks every sharer
    to rebuild its model. */
function guardAtlas(addon: WebglAddon) {
  const atlas = rendererOf(addon)?._charAtlas;
  if (!atlas) return;
  const proto = Object.getPrototypeOf(atlas) as Atlas;
  if (!proto || guarded.has(proto) || typeof proto.clearTexture !== "function") return;
  guarded.add(proto);
  const clear = proto.clearTexture;
  proto.clearTexture = function (this: Atlas) {
    clear.call(this);
    if ("_requestClearModel" in this) this._requestClearModel = true;
  };
}

/** Every renderer's page textures marked stale: uploaded again next frame. */
function rebindAll() {
  for (const h of held) {
    const r = rendererOf(h.addon);
    const atlas = r?._charAtlas;
    if (atlas) r?._glyphRenderer?.value?.setAtlas?.(atlas);
    h.term.refresh(0, h.term.rows - 1);
  }
}

/** **Every atlas cleared, then every terminal redrawn** — all the clears
    before any redraw. */
export function clearAtlases() {
  const live = [...held];
  for (const h of live) {
    try {
      h.addon.clearTextureAtlas();
    } catch {
      /* a context going away; its pane falls back on its own */
    }
  }
  for (const h of live) h.term.refresh(0, h.term.rows - 1);
}

/** On the next frame, once, however many asked. */
function clearSoon() {
  if (clearQueued) return;
  clearQueued = true;
  requestAnimationFrame(() => {
    clearQueued = false;
    clearAtlases();
  });
}

function hearFonts() {
  if (fontsHeard || typeof document === "undefined" || !document.fonts) return;
  fontsHeard = true;
  // A face that finishes loading after a pane drew — the symbols face, an
  // uploaded terminal font, a CJK fallback the system fetched on demand.
  document.fonts.addEventListener("loadingdone", clearSoon);
  void document.fonts.ready.then(clearSoon);
}

/**
 * **WebGL for one terminal, for as long as it stays honest.** Loads the
 * addon, registers it with the page's other WebGL terminals, and keeps it
 * through context losses as the module head says. `onFallback` is told
 * when the pane gives up WebGL for good. Returns the pane's teardown.
 */
export function keepWebgl(
  term: Terminal,
  hooks: { onLoaded?: () => void; onFallback?: (why: string) => void } = {},
): () => void {
  let mine: Held | null = null;
  let stopped = false;
  let loadedOnce = false;
  /** When WebGL last came back after a loss; 0 before any loss. */
  let backAt = 0;
  const drop = () => {
    if (!mine) return;
    const gone = mine;
    mine = null;
    held.delete(gone);
    gone.addon.dispose();
  };
  const use = () => {
    if (mine || stopped) return;
    try {
      const addon = new WebglAddon();
      addon.onContextLoss(drop);
      // A merge or an eviction in the shared atlas: everybody uploads again.
      addon.onRemoveTextureAtlasCanvas(() => queueMicrotask(rebindAll));
      term.loadAddon(addon);
      mine = { term, addon };
      held.add(mine);
      guardAtlas(addon);
      hearFonts();
      if (loadedOnce) {
        // Back after a loss: whatever the shared atlas went through while
        // this pane was away, start everybody from a clean one.
        backAt = performance.now();
        clearSoon();
      }
      loadedOnce = true;
      hooks.onLoaded?.();
    } catch {
      /* no WebGL here: the DOM renderer stands */
    }
  };
  // **Lost: the DOM renderer now, not after the addon's 3 s wait** (T-23).
  const lost = () => {
    const again = backAt && performance.now() - backAt < LOST_AGAIN_MS;
    drop();
    if (again && !stopped) {
      stopped = true;
      hooks.onFallback?.("webgl context lost again within a minute of coming back");
    }
  };
  const shown = () => {
    if (document.visibilityState === "visible") use();
  };
  use();
  term.element?.addEventListener("webglcontextlost", lost, true);
  document.addEventListener("visibilitychange", shown);
  return () => {
    stopped = true;
    term.element?.removeEventListener("webglcontextlost", lost, true);
    document.removeEventListener("visibilitychange", shown);
    drop();
  };
}

/** Whether a terminal draws through WebGL now (for the gates). */
export function onWebgl(term: Terminal): boolean {
  for (const h of held) if (h.term === term) return true;
  return false;
}
