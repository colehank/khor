import {defineConfig, mergeConfig} from 'vite';
import base from '../../vite.config';

export default mergeConfig(base, defineConfig({
  base: './',
  plugins: [{
    name: 'complete-app-lab-entry',
    transformIndexHtml: {order: 'pre', handler: html => html.replace('src="/src/main.tsx"', 'src="/dev/lab/bootstrap.ts"')},
  }],
  server: {port: Number(process.env.KHOR_LAB_VITE_PORT || 23911), strictPort: true, host: '127.0.0.1'},
}));
