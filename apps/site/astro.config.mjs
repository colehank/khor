import { defineConfig } from "astro/config";
import { fileURLToPath } from "node:url";
import {readFile,writeFile} from 'node:fs/promises';
import {deployment,deploymentPath,installUrl} from './deployment.mjs';
import {release} from './src/releases.ts';
import {satteri} from '@astrojs/markdown-satteri';

const deploymentLinks={
  name:'khor-deployment-links',
  link(node,ctx){ctx.setProperty(node,'url',deploymentPath(node.url));},
  definition(node,ctx){ctx.setProperty(node,'url',deploymentPath(node.url));},
  image(node,ctx){ctx.setProperty(node,'url',deploymentPath(node.url));},
  code(node,ctx){ctx.setProperty(node,'value',node.value
    .replaceAll('https://khor.cloud/install.sh',installUrl)
    .replace('https://github.com/colehank/khor/releases/latest/download/install.sh | sh',`https://github.com/colehank/khor/releases/download/${release.version}/install.sh | KHOR_VERSION=${release.version} sh`));},
};

export default defineConfig({
  site: deployment.site,
  base: deployment.base,
  output: 'static',
  devToolbar: {enabled: false},
  // The docs are shell commands and prose. A highlighter would bring a
  // palette of its own onto the page; the design system has one.
  markdown: { syntaxHighlight: false, processor:satteri({mdastPlugins:[deploymentLinks]}) },
  integrations:[{name:'khor-static-install',hooks:{'astro:build:done':async({dir})=>{
    const source=await readFile(new URL('../../scripts/install.sh',import.meta.url),'utf8');
    const original='VERSION="${KHOR_VERSION:-latest}"';
    if(!source.includes(original))throw new Error('Installer default changed; review the website pin before deploying');
    await writeFile(new URL('install.sh',dir),source.replace(original,`VERSION="\${KHOR_VERSION:-${release.version}}"`));
    await writeFile(new URL('.nojekyll',dir),'');
  }}}],
  // The app icon and the tokens are read from where they are decided
  // (apps/gui, docs/design-system), not copied in.
  vite: {
    server: { fs: { allow: ["../.."] } },
    resolve: {alias: {"@": fileURLToPath(new URL("../gui/src",import.meta.url))}},
    esbuild: {jsx: "automatic"},
  },
});
