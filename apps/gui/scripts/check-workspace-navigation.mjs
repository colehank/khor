import assert from 'node:assert/strict';
import {createRequire} from 'node:module';
import {build} from 'esbuild';
import {createElement} from 'react';
import {renderToStaticMarkup} from 'react-dom/server';

const built = await build({
  stdin: {contents: "export {WorkspaceNavigation} from './src/components/WorkspaceNavigation'; export {TooltipProvider} from './src/components/ui/tooltip';", resolveDir: process.cwd(), loader: 'ts'}, bundle: true,
  platform: 'node', format: 'cjs', packages: 'external', jsx: 'automatic',
  loader: {'.css': 'empty'}, write: false,
});
const module = {exports: {}};
Function('require', 'module', 'exports', built.outputFiles[0].text)(createRequire(import.meta.url), module, module.exports);
const {WorkspaceNavigation, TooltipProvider} = module.exports;
const render = props => renderToStaticMarkup(createElement(TooltipProvider, null, createElement(WorkspaceNavigation, {branch: '', view: 'files', onView() {}, ...props})));

const plain = render({repo: false, withSessions: true});
assert(!plain.includes('data-workspace-view="history"'), 'A plain folder must not offer Git history');
assert(plain.includes('data-workspace-view="files"') && plain.includes('data-workspace-view="sessions"'), 'Plain folders retain Files and Sessions');
const repo = render({repo: true, withSessions: true});
assert(repo.includes('data-workspace-view="history"'), 'A Git workspace retains history');
assert.equal(render({repo: false}), '', 'The session workspace has no Git-only controls for a plain directory');
console.log('PASS real workspace navigation: plain folders and Git repositories');

// The complete App Lab must report the same repository boundary as the node.
const fixtureBuild=await build({entryPoints:['dev/lab/navigation.ts'],bundle:true,format:'esm',write:false});
const {navigationFixture}=await import('data:text/javascript;base64,'+Buffer.from(fixtureBuild.outputFiles[0].text).toString('base64'));
const lab=navigationFixture([{id:'local',name:'local',me:true},{id:'remote',name:'remote',me:false}],'http://fixture.invalid',false);
const run=(command,dir,machine='local')=>lab.request(command,{machine,dir});
for(const path of ['/preview/资料库','/preview/project-other']){
 assert.equal(run('checkout',path),false);
 assert.throws(()=>run('git_status',path),/No Git repository/);
 assert.throws(()=>run('git_history',path),/No Git repository/);
}
for(const path of ['/preview/project','/preview/project/src']){
 assert.equal(run('checkout',path),true);
 assert(run('git_history',path).commits.length);
}
assert.equal(run('checkout','/preview/project','remote'),false,'repository facts belong to the selected machine');
console.log('PASS App Lab repository facts: plain directory, path boundaries, subdirectory, machine identity');
