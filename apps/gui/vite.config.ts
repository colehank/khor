import path from "node:path";
import { readFileSync, readdirSync } from "node:fs";
import { defineConfig, mergeConfig, type Plugin } from "vite";
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
const preloadEveryChunk: Plugin = {
  name: "khor-preload-chunks",
  enforce: "post" as const,
  transformIndexHtml: {
    order: "post" as const,
    handler(html, ctx) {
      const bundle = ctx.bundle;
      if (!bundle) return html;
      const files = Object.values(bundle);
      const office = [];
      const links = [];
      for (const file of files) {
        const href = `/${file.fileName}`;
        if (file.type === "chunk" && !file.isEntry) {
          if (file.moduleIds.length > 0 && file.moduleIds.every(id => id.includes("/@silurus/ooxml/"))) {
            office.push({ href, rel: "modulepreload" });
          } else {
            links.push(`<link rel="modulepreload" crossorigin href="${href}">`);
          }
        } else if (/\/(docx|pptx|xlsx)_parser_bg-[\w-]+\.wasm$/.test(href)) {
          office.push({ href, rel: "preload", as: "fetch" });
        } else if (/\/(render-worker-|(?:docx|pptx|xlsx)-source-worker-|pdf\.worker\.min-)/.test(href)) {
          office.push({ href, rel: "modulepreload" });
        }
      }
      // The Windows WebView may stop fetching from its bundled protocol
      // after startup. Other clients fetch the selected renderer on demand.
      const warmOffice = office.length === 0 ? "" : `<script>
        if (window.__KHOR_WINDOW_TRANSPORT__) {
          for (const attrs of ${JSON.stringify(office)}) {
            const link = document.createElement("link");
            Object.assign(link, { crossOrigin: "anonymous" }, attrs);
            document.head.append(link);
          }
        }
      </script>`;
      return html.replace("</head>", `${links.join("\n")}\n${warmOffice}\n</head>`);
    },
  },
};

const previewAssets: Plugin = {
  name: "khor-preview-assets",
  generateBundle() {
    for (const name of ["LICENSE", "THIRD_PARTY_NOTICES.md"]) {
      this.emitFile({
        type: "asset",
        fileName: `licenses/silurus-ooxml/${name}`,
        source: readFileSync(new URL(`./node_modules/@silurus/ooxml/${name}`, import.meta.url)),
      });
    }
    const pdf = new URL("./node_modules/pdfjs-dist/", import.meta.url);
    this.emitFile({ type: "asset", fileName: "licenses/pdfjs/LICENSE", source: readFileSync(new URL("LICENSE", pdf)) });
    for (const directory of ["cmaps", "standard_fonts", "wasm", "iccs"]) {
      for (const file of readdirSync(new URL(directory + "/", pdf), { withFileTypes: true })) {
        if (!file.isFile()) continue;
        const relative = directory + "/" + file.name;
        this.emitFile({ type: "asset", fileName: "assets/pdfjs/" + relative, source: readFileSync(new URL(relative, pdf)) });
      }
    }
  },
};

// 1430, not tauri's usual 1420: mandala's dev server owns 1420 on this
// machine and the two must be able to run side by side.
export const sharedConfig = defineConfig({
  plugins: [react(), tailwindcss(), katexWoff2Only, previewAssets],
  resolve: { alias: {
    "@": path.resolve(import.meta.dirname, "src"),
    "@office-parser": path.resolve(import.meta.dirname, "node_modules/@silurus/ooxml/dist"),
  } },
  server: { port: 1430, strictPort: true },
  // Browser gates exercise the app. Experimental lab entry points may
  // reference retired APIs and must not restart that app mid-gesture.
  optimizeDeps: {
    // Vite 7's dev prebundle moves the viewer's worker assets away from
    // their import.meta.url paths. Build-time asset resolution is separate.
    exclude: ["@silurus/ooxml"],
    ...(process.env.KHOR_FACE_GATE ? { entries: ["index.html"] } : {}),
  },
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

export default mergeConfig(sharedConfig, { plugins: [preloadEveryChunk] });
