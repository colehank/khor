import path from "node:path";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// KaTeX's sheet names three files per font (woff2, woff, ttf) and the
// bundler would ship all three — about 0.8 MB the binary carries and no
// engine this app runs in ever reads. Only the woff2 stays (lane
// md-render, `lib/md/math`).
const katexWoff2Only = {
  name: "katex-woff2-only",
  enforce: "pre" as const,
  transform(code: string, id: string) {
    if (!/katex(\.min)?\.css$/.test(id)) return null;
    return code.replace(/,\s*url\([^)]*\.(?:woff|ttf)\)\s*format\("(?:woff|truetype)"\)/g, "");
  },
};

/**
 * **Fetch every split chunk during the first load, parse it only when needed.**
 *
 * The window fetches its page over Tauri's asset protocol, and that protocol
 * stops answering after the startup burst (2026-11-16: a `fetch('/assets/…')`
 * from the running page never settles, exactly like the IPC `fetch` it shares a
 * transport with). A lazily imported view therefore never arrives: the terminal
 * panel's own chunk is requested minutes after launch and simply hangs, which
 * showed up as an empty pane and no `term_*` command, with no error anywhere.
 *
 * Inlining every dynamic import fixed that and cost too much — one 14 MB bundle
 * made first-contentful-paint 1280 ms and the app feel heavy, because the whole
 * graph had to parse before anything showed.
 *
 * This is the middle road: keep the split, and announce every chunk with
 * `modulepreload` in the document's head. The browser then starts those
 * requests **while the protocol still answers** — they are part of the first
 * load, not a later one — so the module is already in memory when a `lazy()`
 * finally asks for it, and no request has to succeed after the stall. First
 * paint only waits for the entry graph, and the deferred parse happens on the
 * interaction that needs it.
 *
 * The cost is the same bytes as the inline bundle, fetched once on loopback and
 * on the embedded protocol rather than parsed up front. If Tauri's asset
 * transport is ever fixed upstream, delete this plugin and the note with it.
 */
const preloadEveryChunk = {
  name: "khor-preload-chunks",
  enforce: "post" as const,
  transformIndexHtml: {
    order: "post" as const,
    handler(html: string, ctx: { bundle?: Record<string, { type: string; fileName: string; isEntry?: boolean }> }) {
      const bundle = ctx.bundle;
      if (!bundle) return html;
      const links = Object.values(bundle)
        .filter((f) => f.type === "chunk" && !f.isEntry)
        .map((f) => `<link rel="modulepreload" crossorigin href="/${f.fileName}">`)
        .join("\n");
      return links ? html.replace("</head>", `${links}\n</head>`) : html;
    },
  },
};

// 1430, not tauri's usual 1420: mandala's dev server owns 1420 on this
// machine and the two must be able to run side by side.
export default defineConfig({
  plugins: [react(), tailwindcss(), katexWoff2Only, preloadEveryChunk],
  resolve: { alias: { "@": path.resolve(import.meta.dirname, "src") } },
  server: { port: 1430, strictPort: true },
  // Browser gates exercise the app. Experimental lab entry points may
  // reference retired APIs and must not restart that app mid-gesture.
  optimizeDeps: process.env.KHOR_FACE_GATE ? { entries: ["index.html"] } : undefined,
  // **One chunk, because a second one cannot be fetched on Windows**
  // (measured 2026-11-16). The desktop window loads its page over Tauri's
  // custom asset protocol, and that protocol stops answering after the startup
  // burst: a `fetch('/assets/<anything>')` from the running page never settles,
  // exactly like the IPC `fetch` it shares a transport with. Code-splitting
  // therefore leaves most of the app unreachable — the terminal panel is lazy
  // (`views/DetailPane` `lazy(() => import("@/views/TerminalPane"))`), so
  // opening a session drew an empty pane and no `term_open` was ever sent,
  clearScreen: false,
});
