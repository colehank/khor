import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {build} from 'esbuild';
import {createElement} from 'react';
import {renderToStaticMarkup} from 'react-dom/server';

const built = await build({
  entryPoints: ['src/components/WorkspaceNavigation.tsx'], bundle: true,
  platform: 'node', format: 'cjs', packages: 'external', jsx: 'automatic',
  loader: {'.css': 'empty'}, write: false,
});
const module = {exports: {}};
Function('require', 'module', 'exports', built.outputFiles[0].text)(createRequire(import.meta.url), module, module.exports);
const {WorkspaceNavigation} = module.exports;
const render = props => renderToStaticMarkup(createElement(WorkspaceNavigation, {branch: '', view: 'files', onView() {}, ...props}));

const plain = render({repo: false, withSessions: true});
assert(!plain.includes('data-workspace-view="history"'), 'A plain folder must not offer Git history');
assert(plain.includes('data-workspace-view="files"') && plain.includes('data-workspace-view="sessions"'), 'Plain folders retain Files and Sessions');
const repo = render({repo: true, withSessions: true});
assert(repo.includes('data-workspace-view="history"'), 'A Git workspace retains history');
assert.equal(render({repo: false}), '', 'The session workspace has no Git-only controls for a plain directory');
console.log('PASS real workspace navigation: plain folders and Git repositories');
