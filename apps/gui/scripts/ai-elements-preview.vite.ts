import {defineConfig, mergeConfig, type Plugin} from 'vite';
import base from '../vite.config';
import {createLabFixture} from '../dev/lab/fixture';

function previewBridge(terminal: boolean): Plugin {
  const fixture = createLabFixture(terminal);
  return {
    name: 'ai-elements-isolated-preview',
    transformIndexHtml() {
      return [{tag: 'script', injectTo: 'head-prepend', children: `
        localStorage.setItem('khor.lang', 'zh');
        const theme = new URLSearchParams(location.search).get('theme');
        if (['light','dark','auto'].includes(theme)) localStorage.setItem('khor.theme', theme);
        if (!localStorage.getItem('khor.sessions.tabs')) localStorage.setItem('khor.sessions.tabs', JSON.stringify({tabs:['gui/ai-elements-review'],active:'gui/ai-elements-review'}));
      `}];
    },
    configureServer(server) {
      server.httpServer?.on('close', () => fixture.dispose());
      server.middlewares.use(async (req, res, next) => {
        const url = new URL(req.url ?? '/', 'http://preview');
        const command = url.pathname.slice(1);
        if (req.method !== 'POST' && command !== '__ai_preview/reset') return next();
        res.setHeader('content-type', 'application/json');
        res.setHeader('cache-control', 'no-store');
        try {
          let raw = '';
          for await (const chunk of req) {
            if (['chat_put', 'upload'].includes(command)) continue;
            raw += String(chunk);
            if (raw.length > 1000000) throw new Error('Preview request too large');
          }
          const answer = ['chat_put', 'upload'].includes(command)
            ? `${url.searchParams.get('into') ?? '/preview/files'}/${url.searchParams.get('name') ?? 'attachment'}`
            : fixture.request(command, JSON.parse(raw || '{}'));
          res.end(JSON.stringify(answer));
        } catch (error) { res.statusCode = 400; res.end(JSON.stringify(String(error))); }
      });
    },
  };
}
export function aiElementsPreview(terminal = false) {
  return mergeConfig(base, defineConfig({plugins: [previewBridge(terminal)], server: {host: '127.0.0.1', port: 23776, strictPort: true}}));
}
export default aiElementsPreview(process.env.KHOR_PREVIEW_TERMINAL === '1');
