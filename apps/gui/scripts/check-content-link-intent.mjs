// Production prose link callbacks, with transport and navigation observed only.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {execFileSync} from 'node:child_process';
import {transform,build} from 'esbuild';
const legacy=process.argv.includes('--legacy');
const read=path=>legacy?execFileSync('git',['show','e7c335c8:apps/gui/'+path],{encoding:'utf8'}):readFileSync(path,'utf8');
const pathSource=read('src/components/PathLinks.tsx').split('export function PathLink')[1];
const markdownSource=read('src/components/Markdown.tsx').match(/function Link\([\s\S]*?\n\}\n/)[0];
const pathCode=(await transform('function PathLink'+pathSource,{loader:'tsx'})).code;
const markdownCode=(await transform(markdownSource,{loader:'tsx'})).code;
const results=[];
async function test(name,run){try{await run();results.push({name,pass:true});console.log('PASS',name);}catch(e){results.push({name,pass:false,error:e.message});console.error('FAIL',name,e.message);}}
const intent=e=>e.metaKey||e.ctrlKey?'system':'internal';
function render(kind,options={}){
 let state;
 const calls=[];
 const home={machine:'remote-machine',root:'/project',known:()=>true,see(){},open:(path,line)=>calls.push(['internal-path',path,line]),openPage:url=>calls.push(['internal-url',url]),launch:path=>calls.push(['remote-launch',path])};
 const React={createElement:(type,props,...children)=>({type,props,children})};
 const env={React,useState:initial=>{if(state===undefined)state=initial;return [state,value=>state=value];},useEffect(){},useLinkHome:()=>home,underRoot:(_,written)=>({abs:written,line:null}),cn:(...v)=>v.join(' '),linkIntent:intent,assertPageUrl:url=>{if(!/^https?:\/\//i.test(url))throw Error('not a page');},openSystemPath:async path=>{calls.push(['system-path',path]);if(options.failures>0){options.failures--;throw Error('fixture refused');}},openSystemUrl:async url=>calls.push(['system-url',url]),openLink:async url=>calls.push(['system-url',url]),launchOn:async(machine,url)=>calls.push(['remote-url',machine,url]),jump:()=>calls.push(['anchor'])};
 const component=Function(...Object.keys(env),(kind==='path'?pathCode:markdownCode)+';return '+(kind==='path'?'PathLink':'Link')+';')(...Object.values(env));
 const props=kind==='path'?{written:'/project/file.txt'}:{href:'http://localhost:3000/path?query=raw',children:'website'};
 const redraw=(patch={})=>component({...props,...patch});
 return {calls,element:redraw(),redraw};
}
for(const kind of ['path','url'])for(const modifiers of [{},{metaKey:true},{ctrlKey:true}])await test(kind+' '+JSON.stringify(modifiers)+' chooses exactly one requested opener',async()=>{
 const {calls,element}=render(kind);await element.props.onClick({...modifiers,currentTarget:{}});
 const system=intent(modifiers)==='system';assert.deepEqual(calls,[[`${system?'system':'internal'}-${kind}`,kind==='path'?'/project/file.txt':'http://localhost:3000/path?query=raw',...(kind==='path'&&!system?[null]:[])]]);
});
await test('path Enter uses the same system intent',async()=>{const {calls,element}=render('path');element.props.onKeyDown({key:'Enter',metaKey:true,preventDefault(){}});await Promise.resolve();assert.deepEqual(calls,[['system-path','/project/file.txt']]);});
await test('a failed path handoff remains interactive and can be retried',async()=>{const f=render('path',{failures:1});await f.element.props.onClick({metaKey:true});const refused=f.redraw();assert.equal(typeof refused.props.onClick,'function');assert.match(refused.props.title,/fixture refused/);await refused.props.onClick({metaKey:true});assert.equal(f.redraw().props.title,'/project/file.txt');});
await test('a reused path component never applies another target refusal',async()=>{const f=render('path',{failures:1});await f.element.props.onClick({metaKey:true});assert.equal(f.redraw({written:'/project/other.txt'}).props.title,'/project/other.txt');});
const helper=(await build({stdin:{contents:"export {fileUri} from './src/lib/content-link-intent';export {assertPageUrl,openSystemPath,openSystemUrl,openSystemFileUri} from './src/lib/client-opener';",resolveDir:process.cwd(),loader:'ts'},bundle:true,write:false,format:'esm',plugins:[{name:'client-fixture',setup(b){b.onResolve({filter:/^@\/api$/},()=>({path:'api',namespace:'fixture'}));b.onLoad({filter:/.*/,namespace:'fixture'},()=>({contents:'export const inApp=false;export const windowsApp=false;export const launchOn=()=>{throw Error("server opener called")};export const openLink=()=>{throw Error("server opener called")};'}));}}]})).outputFiles[0].text;
const {fileUri,openSystemPath,openSystemUrl,openSystemFileUri}=await import('data:text/javascript;base64,'+Buffer.from(helper).toString('base64'));
await test('file URI preserves spaces, percent, fragment and query characters',()=>{const raw='/Users/示例/a b%20#?.txt';const uri=new URL(fileUri(raw));assert.equal(decodeURIComponent(uri.pathname),raw);assert.equal(uri.search,'');assert.equal(uri.hash,'');});
await test('a POSIX backslash is a filename character, not a separator',()=>{assert.equal(fileUri('/tmp/a\\b.txt'),'file:///tmp/a%5Cb.txt');});
await test('Windows drive and UNC handoffs preserve the file target',()=>{assert.equal(fileUri('C:\\Work\\a b.txt'),'file:///C:/Work/a%20b.txt');assert.equal(fileUri('\\\\server\\share\\a.txt'),'file://server/share/a.txt');});
await test('web system handoff never calls an opener on its server',async()=>{const calls=[];globalThis.window={open:(...args)=>{calls.push(args);return null;}};await openSystemPath('/Users/example/file.txt');await openSystemUrl('http://localhost:3000/raw');assert.deepEqual(calls,[['file:///Users/example/file.txt','_blank','noopener'],['http://localhost:3000/raw','_blank','noopener']]);});
await test('web file URI handoff preserves authority and drive without reinterpretation',async()=>{const calls=[];globalThis.window={open:(...args)=>calls.push(args)};for(const uri of ['file://server/share/a.txt','file:///C:/Work/a%20b.txt'])await openSystemFileUri(uri);assert.deepEqual(calls.map(c=>c[0]),['file://server/share/a.txt','file:///C:/Work/a%20b.txt']);});
await test('non-page protocols retain the existing URL constraint',async()=>{const calls=[];globalThis.window={open:(...args)=>calls.push(args)};await assert.rejects(openSystemUrl('mailto:someone@example.com'));assert.deepEqual(calls,[]);const f=render('url');await f.redraw({href:'mailto:someone@example.com'}).props.onClick({});assert.deepEqual(f.calls,[]);});
await test('native OSC8 file handoff passes the original URI to the client opener',async()=>{const nativeCode=(await build({entryPoints:['src/lib/client-opener.ts'],bundle:true,write:false,format:'esm',plugins:[{name:'native-client-fixture',setup(b){b.onResolve({filter:/^@\/api$/},()=>({path:'api',namespace:'native'}));b.onLoad({filter:/.*/,namespace:'native'},()=>({contents:'export const inApp=true;export const launchOn=()=>{throw Error("path rewrite used")};export const openLink=uri=>globalThis.nativeFileCalls.push(uri);'}));}}]})).outputFiles[0].text;const native=await import('data:text/javascript;base64,'+Buffer.from(nativeCode).toString('base64'));globalThis.nativeFileCalls=[];for(const uri of ['file://server/share/a.txt','file:///C:/Work/a%20b.txt'])await native.openSystemFileUri(uri);assert.deepEqual(globalThis.nativeFileCalls,['file://server/share/a.txt','file:///C:/Work/a%20b.txt']);delete globalThis.nativeFileCalls;});
console.log(JSON.stringify({legacy,passed:results.filter(t=>t.pass).length,total:results.length,results},null,2));assert(results.every(t=>t.pass),'content link intent regressions');
