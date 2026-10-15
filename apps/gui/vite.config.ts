import path from "node:path";
import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

// 1430, not tauri's usual 1420: mandala's dev server owns 1420 on this
// machine and the two must be able to run side by side.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: { alias: { "@": path.resolve(import.meta.dirname, "src") } },
  server: { port: 1430, strictPort: true },
  // Browser gates exercise the app. Experimental lab entry points may
  // reference retired APIs and must not restart that app mid-gesture.
  optimizeDeps: process.env.KHOR_FACE_GATE ? { entries: ["index.html"] } : undefined,
  clearScreen: false,
});
