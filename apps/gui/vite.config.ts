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

// 1430, not tauri's usual 1420: mandala's dev server owns 1420 on this
// machine and the two must be able to run side by side.
export default defineConfig({
  plugins: [react(), tailwindcss(), katexWoff2Only],
  resolve: { alias: { "@": path.resolve(import.meta.dirname, "src") } },
  server: { port: 1430, strictPort: true },
  // Browser gates exercise the app. Experimental lab entry points may
  // reference retired APIs and must not restart that app mid-gesture.
  optimizeDeps: process.env.KHOR_FACE_GATE ? { entries: ["index.html"] } : undefined,
  clearScreen: false,
});
