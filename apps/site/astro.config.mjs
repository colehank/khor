import { defineConfig } from "astro/config";

export default defineConfig({
  site: "https://khor.cloud",
  // The docs are shell commands and prose. A highlighter would bring a
  // palette of its own onto the page; the design system has one.
  markdown: { syntaxHighlight: false },
  // The app icon and the tokens are read from where they are decided
  // (apps/gui, docs/design-system), not copied in.
  vite: { server: { fs: { allow: ["../.."] } } },
});
