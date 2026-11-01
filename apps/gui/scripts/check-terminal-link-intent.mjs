import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {execFileSync} from 'node:child_process';
import {transform,build} from 'esbuild';
const legacy=process.argv.includes('--legacy');
const read=path=>legacy?execFileSync('git',['show','e7c335c8:apps/gui/'+path],{encoding:'utf8'}):readFileSync(path,'utf8');
const pane=read('src/views/TerminalPane.tsx'),links=read('src/lib/term-links.ts');
const compile=async source=>(await transform(source,{loader:'tsx'})).code;
const followSource=pane.match(/(?:async )?function follow\([\s\S]*?\n\}/)?.[0];
const providerSource=links.match(/export function pathLinks\([\s\S]*?\n\}/)?.[0];
const urlSource=pane.match(/new WebLinksAddon\((\(ev, uri\) => \{[\s\S]*?\n      \})\)/)?.[1];
const oscSource=pane.match(/      linkHandler: \{[\s\S]*?        activate: (\(ev, uri\) => \{[\s\S]*?\n        \}),/)?.[1];
assert(followSource&&providerSource&&urlSource&&oscSource,'production terminal link callbacks exist');
const followCode=await compile(followSource),providerCode=await compile(providerSource.replace('export ',''));
const urlCode=await compile('const callback='+urlSource),oscCode=await compile('const callback='+oscSource);
const {linkIntent:intent,canActivateTerminalLink}=await import('data:text/javascript;base64,'+Buffer.from((await build({entryPoints:['src/lib/content-link-intent.ts'],bundle:true,format:'esm',write:false})).outputFiles[0].text).toString('base64'));
const results=[];
async function test(name,run){try{await run();results.push({name,pass:true});console.log('PASS',name);}catch(e){results.push({name,pass:false,error:e.message});console.error('FAIL',name,e.message);}}
function fixture(inApp,options={}){
 const term={hasSelection:()=>!!options.selected,modes:{mouseTrackingMode:options.tracking??'none'}};
 const calls=[];
 const follow=Function('inApp','askOpenUrl','openSystemUrl','assertPageUrl','window',followCode+';return follow;')(inApp,(machine,url)=>calls.push(['internal-url',machine,url]),async url=>calls.push(['system-url',url]),url=>{if(!/^https?:\/\//i.test(url))throw Error('not a page');},{open:url=>calls.push(['system-url',url])});
 const openUrl=(uri,mode='internal')=>follow(uri,'remote-machine',mode);
 const fileLink=(uri,mode='internal')=>calls.push([mode+'-file',uri]);
 const env={openUrl,follow:(uri,machine)=>follow(uri,machine),machine:'remote-machine',fileLink,linkIntent:intent,canActivateTerminalLink,term};
 const callback=code=>Function(...Object.keys(env),code+';return callback;')(...Object.values(env));
 return {calls,url:callback(urlCode),osc:callback(oscCode),follow};
}
for(const native of [true,false])for(const mods of [{},{metaKey:true}])await test(`${native?'native':'web'} detected URL ${JSON.stringify(mods)} uses requested destination`,async()=>{const f=fixture(native);await f.url(mods,'http://localhost:4321/raw');await Promise.resolve();assert.deepEqual(f.calls,[intent(mods)==='internal'?['internal-url','remote-machine','http://localhost:4321/raw']:['system-url','http://localhost:4321/raw']]);});
for(const scheme of ['http://localhost:4321/osc','file:///project/notes.txt'])for(const mods of [{},{metaKey:true}])await test('OSC8 '+scheme+' '+JSON.stringify(mods),async()=>{const f=fixture(true);await f.osc(mods,scheme);await Promise.resolve();const mode=intent(mods);assert.deepEqual(f.calls,[scheme.startsWith('file:')?[mode+'-file',scheme]:mode==='system'?['system-url',scheme]:['internal-url','remote-machine',scheme]]);});
const provider=Function('pathsOn','linkIntent','canActivateTerminalLink',providerCode+';return pathLinks;')(async()=>[{from:{x:0,y:0},to:{x:8,y:0},text:'/project/file.txt',target:{path:'/project/file.txt',dir:false,line:12,col:null}}],intent,canActivateTerminalLink);
for(const mods of [{},{metaKey:true},{ctrlKey:true}])await test('detected path '+JSON.stringify(mods)+' forwards one intent with unchanged target',async()=>{const calls=[];const found=await new Promise(resolve=>provider({hasSelection:()=>false,modes:{mouseTrackingMode:'none'}},'remote-machine',()=>'/project',(target,mode)=>calls.push([target,mode])).provideLinks(1,resolve));found[0].activate(mods);assert.deepEqual(calls,[[{path:'/project/file.txt',dir:false,line:12,col:null},intent(mods)]]);});
for(const source of ['url','osc'])await test(source+' selection drag never opens the selected link',async()=>{const f=fixture(true,{selected:true});await f[source]({button:0},'http://localhost:4321/raw');assert.deepEqual(f.calls,[]);});
await test('unclaimed tracked mouse events cannot also activate a link',async()=>{const f=fixture(true,{tracking:'vt200'});await f.url({button:0},'http://localhost:4321/raw');await f.url({button:0,metaKey:true},'http://localhost:4321/raw');assert.deepEqual(f.calls,[]);});
await test('right click keeps its context-menu behavior without opening',async()=>{const f=fixture(true);await f.url({button:2},'http://localhost:4321/raw');assert.deepEqual(f.calls,[]);});
await test('path provider preserves selection and tracked application clicks',async()=>{for(const term of [{hasSelection:()=>true,modes:{mouseTrackingMode:'none'}},{hasSelection:()=>false,modes:{mouseTrackingMode:'vt200'}}]){const calls=[];const found=await new Promise(resolve=>provider(term,'remote-machine',()=>'/project',(...args)=>calls.push(args)).provideLinks(1,resolve));found[0].activate({button:0});assert.deepEqual(calls,[]);}});
if(!legacy){
 const activateSource=pane.match(/  const activateLink=async\([\s\S]*?\n  \};/)?.[0];assert(activateSource,'local terminal link action coordinator exists');
 const activateCode=await compile(activateSource);
 function actions(){let failure=null;const linkAction={current:0};return {linkAction,get failure(){return failure;},activate:Function('linkAction','setLinkFailure','said',activateCode+';return activateLink;')(linkAction,value=>failure=value,error=>error.message)};}
 await test('opener failure is local and a succeeding action clears it',async()=>{const f=actions();await f.activate(()=>Promise.reject(new Error('cannot open target')));assert.equal(f.failure,'cannot open target');await f.activate(()=>{});assert.equal(f.failure,null);});
 await test('late failure cannot replace the result of a newer action',async()=>{const f=actions();let reject;const first=f.activate(()=>new Promise((_,no)=>reject=no));await f.activate(()=>{});reject(new Error('stale'));await first;assert.equal(f.failure,null);});
 await test('session or close invalidation prevents late failure state',async()=>{const f=actions();let reject;const first=f.activate(()=>new Promise((_,no)=>reject=no));f.linkAction.current++;reject(new Error('stale'));await first;assert.equal(f.failure,null);});
}

if(!legacy){
 const fileSource=pane.match(/    fileLink = \(uri,intent='internal'\) => \{[\s\S]*?\n    \};/)?.[0];assert(fileSource,'production file URI callback exists');
 const fileCode=await compile(fileSource);
 await test('OSC8 system file callback preserves raw URI before parsing',async()=>{const calls=[];const callback=Function('activateLink','openSystemFileUri','openPath',"let fileLink;"+fileCode+';return fileLink;')(open=>open(),uri=>calls.push(uri),()=>assert.fail('system URI parsed as internal path'));for(const uri of ['file://server/share/a.txt','file:///C:/Work/a%20b.txt'])callback(uri,'system');assert.deepEqual(calls,['file://server/share/a.txt','file:///C:/Work/a%20b.txt']);});
 const tappedSource=pane.match(/    const tapped = \(row: number, col: number\) => \{[\s\S]*?\n    \};/)?.[0];assert(tappedSource,'production coarse-tap callback exists');
 const tappedCode=await compile(tappedSource);
 await test('coarse URL/path taps preserve an active selection',async()=>{for(const state of [{selected:true}]){const term={hasSelection:()=>!!state.selected,modes:{mouseTrackingMode:state.tracking??'none'}};const callback=Function('picture','canActivateTerminalLink','term','urlAt',tappedCode+';return tapped;')(()=>false,canActivateTerminalLink,term,()=>assert.fail('tracked/selected tap tried link lookup'));assert.equal(callback(0,0),false);}});
 await test('a completed touch tap opens a URL inside a mouse-tracking TUI',async()=>{
   const calls=[];const term={hasSelection:()=>false,modes:{mouseTrackingMode:'vt200'}};
   const callback=Function('picture','canActivateTerminalLink','term','urlAt','openUrl','fileLink','pathsOn',tappedCode+';return tapped;')(()=>false,canActivateTerminalLink,term,()=> 'https://example.com/tui',uri=>calls.push(uri),()=>assert.fail('not a file'),()=>assert.fail('URL must not query paths'));
   assert.equal(callback(0,0),true);assert.deepEqual(calls,['https://example.com/tui']);
 });

}
console.log(JSON.stringify({legacy,passed:results.filter(t=>t.pass).length,total:results.length,results},null,2));assert(results.every(t=>t.pass),'terminal link intent regressions');
