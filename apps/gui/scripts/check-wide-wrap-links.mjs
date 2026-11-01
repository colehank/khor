// Real xterm produces the wide-character wrap padding; only directory IO is fixed.
import assert from 'node:assert/strict';
import {build} from 'esbuild';
import {fileURLToPath} from 'node:url';
import {execFileSync} from 'node:child_process';
const legacy=process.argv.includes('--legacy');
const gui=fileURLToPath(new URL('..',import.meta.url));
const bundle=await build({stdin:{contents:"export {Terminal} from '@xterm/xterm';export {pathsOn,covers,urlAt} from '@/lib/term-links';",resolveDir:gui,loader:'ts'},bundle:true,format:'esm',platform:'node',write:false,plugins:[{name:'fixed-directory',setup(b){if(legacy)b.onLoad({filter:/src\/lib\/term-links\.ts$/},()=>({contents:execFileSync('git',['show','e58f21a1:apps/gui/src/lib/term-links.ts'],{encoding:'utf8'}),loader:'ts'}));b.onResolve({filter:/^@\/api$/},()=>({path:'api',namespace:'fixture'}));b.onLoad({filter:/.*/,namespace:'fixture'},()=>({contents:'export const fetchLs=(machine,path)=>globalThis.fixtureLs(path);'}));b.onResolve({filter:/^@\//},a=>b.resolve('./src/'+a.path.slice(2),{resolveDir:gui,kind:a.kind}));}}]});
const {Terminal,pathsOn,covers,urlAt}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
const parent='/fixture/projects/khor/source/very/long/component';
const name='测试'.repeat(12)+'.tsx',target=parent+'/'+name;
globalThis.window={setTimeout:()=>0};
globalThis.fixtureLs=async path=>({path,entries:path===parent?[{name,dir:false},{name:'my file 测试.tsx',dir:false}]:[]});
const results=[];
async function test(name,run){const t=new Terminal({cols:40,rows:20,allowProposedApi:true});try{await run(t);results.push({name,pass:true});console.log('PASS',name);}catch(error){results.push({name,pass:false,error:error.message});console.error('FAIL',name,error.message);}finally{t.dispose();}}
const write=(t,text)=>new Promise(resolve=>t.write(text+'\r\n',resolve));
const rows=t=>Array.from({length:t.buffer.active.length},(_,y)=>({y,line:t.buffer.active.getLine(y)})).filter(({line})=>line.translateToString(true).trim());
await test('wide filename moved off the last column remains one path with exact cell ranges',async t=>{
 const prefix='FILE '+' '.repeat((t.cols-1-((5+parent.length+1)%t.cols)+t.cols)%t.cols);
 await write(t,prefix+target+':12:3');
 const printed=rows(t),padding=printed.find(({y,line})=>line.getCell(t.cols-1).getCode()===0&&t.buffer.active.getLine(y+1)?.isWrapped&&t.buffer.active.getLine(y+1).getCell(0).getWidth()===2);
 assert(padding,'real xterm must produce a code0 last-column spacer before a wrapped wide character');
 for(const {y} of printed){const found=(await pathsOn(t,'wide-fixture',()=>'/fixture/projects/khor',y)).find(link=>link.target.path===target);assert(found,'complete path must be found from each wrapped row '+y);assert.deepEqual(found.from,{x:prefix.length+1,y:0},'link must start at the absolute path, not re-resolve a detached filename');assert.equal(found.target.line,12);assert.equal(found.target.col,3);if(y===padding.y+1)assert(covers(found,y,0),'wide first cell belongs to the link');}
});
await test('an explicit space before a wrapped wide character stays part of a quoted filename',async t=>{
 const spaced=parent+'/my file 测试.tsx',firstWide=spaced.indexOf('测');
 const prefix='FILE '+' '.repeat((t.cols-((6+firstWide)%t.cols))%t.cols);
 await write(t,prefix+"'"+spaced+"'");
 const blank=rows(t).find(({y,line})=>line.getCell(t.cols-1).getCode()===32&&t.buffer.active.getLine(y+1)?.isWrapped&&t.buffer.active.getLine(y+1).getCell(0).getWidth()===2);
 assert(blank,'fixture must put a real U+0020 space in the last column');
 const found=await pathsOn(t,'wide-fixture',()=>'/fixture/projects/khor',blank.y+1);assert(found.some(link=>link.target.path===spaced),'explicit filename space must not be trimmed');
});
await test('independent hard lines never become one complete filename',async t=>{
 await write(t,parent+'/\r\n'+name);
 for(const {y} of rows(t)){const found=await pathsOn(t,'wide-fixture',()=>'/elsewhere',y);assert(!found.some(link=>link.target.path===target));}
});
await test('a wide URL keeps its full target when tapped after the wrap',async t=>{
 const url='https://example.invalid/'+name;
 const prefix='URL '+' '.repeat((t.cols-1-((4+url.indexOf('测'))%t.cols)+t.cols)%t.cols);
 await write(t,prefix+url);
 const last=rows(t).at(-1);assert.equal(urlAt(t,last.y,2),url);
});
console.log(JSON.stringify({legacy,passed:results.filter(t=>t.pass).length,total:results.length,results},null,2));assert(results.every(t=>t.pass),'wide-wrap link regressions');
