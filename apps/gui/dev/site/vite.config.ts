import {defineConfig,mergeConfig} from 'vite';
import {fileURLToPath} from 'node:url';
import base from '../../vite.config';
export default mergeConfig(base,defineConfig({
  base:'./',
  resolve:{alias:[
    {find:'@/lib/backend-context',replacement:fileURLToPath(new URL('../lab/backend-context.ts',import.meta.url))},
    {find:'@/views/FileView',replacement:fileURLToPath(new URL('./file-view.ts',import.meta.url))},
  ]},
  plugins:[{name:'site-demo-entry',transformIndexHtml:{order:'pre',handler:html=>html.replace('src="/src/main.tsx"','src="/dev/site/bootstrap.ts"')}}],
}));
