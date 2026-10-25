import { defineConfig, mergeConfig } from 'vite';
import sessionPreview from './session-navigation-preview.vite';
import { qualityCommitShow } from './quality-fixture.mjs';

export default mergeConfig(sessionPreview, defineConfig({
  cacheDir: 'node_modules/.vite-quality',
  plugins: [{
    name: 'quality-commit-fixture',
    enforce: 'pre',
    configureServer(server) {
      server.middlewares.use((request, response, next) => {
        request.url = request.url?.replace(/^\/api\//, '/');
        const fixture = {
          lock_state: { on: false, has_pin: false, timeout_min: 0, timeouts: [0, 1, 5, 15, 30], pin_min: 4, system_door: false },
          page_icons: {}, day_strips: {}, face_here: null,
        };
        const action = (request.url ?? '').split('?')[0].slice(1);
        if (request.method === 'POST' && Object.hasOwn(fixture, action)) {
          request.resume();
          response.setHeader('content-type', 'application/json');
          response.setHeader('cache-control', 'no-store');
          response.end(JSON.stringify(fixture[action as keyof typeof fixture]));
          return;
        }
        if (request.method !== 'POST' || !/^\/(?:api\/)?git_commit(?:\?|$)/.test(request.url ?? '')) return next();
        request.resume();
        response.setHeader('content-type', 'application/json');
        response.setHeader('cache-control', 'no-store');
        response.end(JSON.stringify(qualityCommitShow));
      });
    },
  }],
  server: { host: '0.0.0.0', port: 23820, strictPort: true },
}));
