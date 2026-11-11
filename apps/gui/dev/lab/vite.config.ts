import {defineConfig, mergeConfig} from 'vite';
import {readFileSync,readdirSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import base from '../../vite.config';

const previewFiles = fileURLToPath(new URL('./data/workspace/media/', import.meta.url));
const previewPrefix = '/assets/preview-fixtures/media/';
const previewTypes:Record<string,string>={html:'text/html',css:'text/css',js:'text/javascript',json:'application/json',mp4:'video/mp4',wav:'audio/wav'};
const port = Number(process.env.KHOR_LAB_VITE_PORT || 23911);

export default mergeConfig(base, defineConfig({
  base: './',
  resolve: {alias: [{find: '@/lib/backend-context', replacement: fileURLToPath(new URL('./backend-context.ts', import.meta.url))}]},
  optimizeDeps: {entries: ['index.html']},
  plugins: [{
    name: 'complete-app-lab-entry',
    transformIndexHtml: {order: 'pre', handler: html => html.replace('src="/src/main.tsx"', 'src="/dev/lab/bootstrap.ts"')},
    generateBundle() {
      for(const name of readdirSync(previewFiles))this.emitFile({type:'asset',fileName:previewPrefix.slice(1)+name,source:readFileSync(previewFiles+name)});
    },
    configureServer(server) {
      server.middlewares.use(previewPrefix,(request,response,next)=>{
        const name=request.url?.split('?')[0]?.replace(/^\//,'');
        if(!name||!readdirSync(previewFiles).includes(name))return next();
        response.setHeader('Access-Control-Allow-Origin','*');
        response.setHeader('Content-Type',previewTypes[name.split('.').pop()??'']??'application/octet-stream');
        response.end(readFileSync(previewFiles+name));
      });
      const runtime = process.env.KHOR_LAB_RUNTIME;
      if (!runtime) return;
      server.middlewares.use('/lab/runtime', (_request, response) => {
        response.setHeader('Content-Type', 'application/json');
        response.setHeader('Cache-Control', 'no-store');
        response.end(readFileSync(runtime));
      });
      server.middlewares.use('/lab/example', (_request, response) => {
        response.setHeader('Content-Type', 'text/html; charset=utf-8');
        response.end('<!doctype html><meta name="viewport" content="width=device-width,initial-scale=1"><title>Khor Lab page</title><p>Khor browser fixture page.</p>');
      });
    },
  }],
  server: {port, strictPort: true, host: '127.0.0.1', hmr: {protocol: 'ws', host: '127.0.0.1', clientPort: port}},
}));
