// Exercise production listeners, including native event/dragend ordering.
// No user session or real terminal input: this gate records the drop payload.
import assert from 'node:assert/strict';
import { build } from 'esbuild';
const saved=Object.fromEntries(['window','navigator','devicePixelRatio'].map(k=>[k,globalThis[k]]));
const native=[];let listening=0,unlistened=0;
try {
 const win=new EventTarget();win.matchMedia=()=>({matches:false});
 Object.assign(globalThis,{window:win,devicePixelRatio:2});Object.defineProperty(globalThis,'navigator',{configurable:true,value:{platform:'MacIntel'}});
 const bundle=await build({stdin:{contents:"export * from './src/lib/path-drag'; export * from './src/lib/window-drop';",resolveDir:process.cwd(),loader:'ts'},bundle:true,format:'esm',write:false,define:{'import.meta.env.DEV':'true'},plugins:[{name:'native-fixture',setup(b){b.onResolve({filter:/^@\/api$/},()=>({path:'api',namespace:'fixture'}));b.onResolve({filter:/^@tauri-apps\/api\/webview$/},()=>({path:'tauri',namespace:'fixture'}));b.onLoad({filter:/.*/,namespace:'fixture'},a=>({contents:a.path==='api'?'export const inApp=true':'export const getCurrentWebview=()=>({onDragDropEvent:globalThis.__listenDrop})',loader:'js'}));}}]});
 globalThis.__listenDrop=async cb=>{listening++;native.push(cb);return()=>{unlistened++;native.splice(native.indexOf(cb),1)}};
 const mod=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
 const emit=payload=>{for(const cb of [...native])cb({payload})};
 const seen=[],hidden=[];const offHidden=mod.onWindowDrop(d=>{if(false)hidden.push(d)}),off=mod.onWindowDrop(d=>seen.push(d));
 await new Promise(r=>setTimeout(r,0));
 const point={x:700,y:300};emit({type:'drop',paths:['/tmp/a b.txt'],position:point});assert.deepEqual(seen.pop(),{paths:['/tmp/a b.txt'],from:null,...point});assert.equal(listening,1,'kept panes share one native resolver');
 assert.deepEqual(mod.dropPoint({x:1400,y:600},'Win32',2),point);
 for(const platform of ['MacIntel','Linux x86_64'])for(const scale of [1,2])assert.deepEqual(mod.dropPoint(point,platform,scale),point);
 assert.deepEqual(mod.dropPoint(point,'MacIntel',2),point);assert.deepEqual(mod.dropPoint(point,'MacIntel',1),point);
 const drag=(machine='far',path='/srv/x y.txt')=>{const store=new Map(),source={isConnected:true};const dataTransfer={dropEffect:'none',setData:(k,v)=>store.set(k,v),getData:k=>store.get(k)||'',types:[mod.PATH_DRAG]};const props=mod.pathDrag(machine,path);props.onDragStart({currentTarget:source,dataTransfer});return {end:()=>props.onDragEnd({currentTarget:source,dataTransfer}),dataTransfer};};
 const enter=()=>emit({type:'enter',paths:[],position:point}),drop=()=>emit({type:'drop',paths:[],position:point});
 let d=drag();enter();drop();d.end();assert.equal(seen.length,1);assert.equal(seen.pop().from,'far');assert.equal(mod.droppedPath(d),null,'DOM fallback must not paste a consumed native drop twice');drop();assert.equal(seen.length,0);
 d=drag(null,'/tmp/local');enter();d.end();drop();assert.deepEqual(seen.pop(),{paths:['/tmp/local'],from:null,...point});
 d=drag();enter();emit({type:'leave'});d.end();drop();assert.equal(seen.length,0,'cancelled/left drag must not paste');
 d=drag();enter();win.dispatchEvent(new Event('blur'));drop();assert.equal(seen.length,0,'window blur cancels');
 d=drag();enter();const escape=new Event('keydown');Object.defineProperty(escape,'key',{value:'Escape'});win.dispatchEvent(escape);drop();assert.equal(seen.length,0,'Escape cancels');
 d=drag();enter();d.end();enter();drop();assert.equal(seen.length,0,'new native drag cannot reuse ended source');
 d=drag();enter();d.end();await new Promise(r=>setTimeout(r,550));drop();assert.equal(seen.length,0,'expired source cannot paste');
 d=drag();enter();emit({type:'drop',paths:['/tmp/Finder'],position:point});assert.deepEqual(seen.pop().paths,['/tmp/Finder']);drop();assert.equal(seen.length,0,'OS file paths never inherit a remote source');
 d=drag();enter();assert.deepEqual(mod.droppedPath(d),{machine:'far',path:'/srv/x y.txt'});drop();assert.equal(seen.length,0,'DOM then native is once');
 off();offHidden();assert.equal(unlistened,1);assert.deepEqual(hidden,[]);
 console.log('terminal drop: Mac/GTK logical coordinates, Windows scale, native event/dragend ordering, source machines, cancellation, fresh drag, expiry, dedup and retained-pane fanout passed');
} finally {delete globalThis.__listenDrop;for(const [k,v] of Object.entries(saved))Object.defineProperty(globalThis,k,{configurable:true,writable:true,value:v});}
