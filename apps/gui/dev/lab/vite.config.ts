import {defineConfig, mergeConfig} from 'vite';
import {readFileSync} from 'node:fs';
import {fileURLToPath} from 'node:url';
import base from '../../vite.config';

const port = Number(process.env.KHOR_LAB_VITE_PORT || 23911);

export default mergeConfig(base, defineConfig({
  base: './',
  resolve: {alias: [{find: '@/lib/backend-context', replacement: fileURLToPath(new URL('./backend-context.ts', import.meta.url))}]},
  optimizeDeps: {entries: ['index.html']},
  plugins: [{
    name: 'complete-app-lab-entry',
    transformIndexHtml: {order: 'pre', handler: html => html.replace('src="/src/main.tsx"', 'src="/dev/lab/bootstrap.ts"')},
    configureServer(server) {
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
