import assert from 'node:assert/strict';
import {build} from 'esbuild';
import {execFileSync} from 'node:child_process';

const baseline=process.argv.includes('--baseline');
const plugins=baseline?[{name:'published-baseline',setup(build){
  build.onLoad({filter:/content-link-intent\.ts$/},()=>({contents:execFileSync('git',['show','27a9ce5c:apps/gui/src/lib/content-link-intent.ts'],{encoding:'utf8'}),loader:'ts'}));
  build.onLoad({filter:/term-link-gesture\.ts$/},()=>({contents:'export const claimTerminalLinkClicks=()=>()=>{};',loader:'ts'}));
}}]:[];
const out=await build({stdin:{contents:"export * from './src/lib/term-link-gesture'; export * from './src/lib/content-link-intent';",resolveDir:process.cwd(),loader:'ts'},bundle:true,format:'esm',write:false,plugins});
const {claimTerminalLinkClicks,canActivateTerminalLink,linkIntent}=await import('data:text/javascript;base64,'+Buffer.from(out.outputFiles[0].text).toString('base64'));

class Mouse extends Event {
  constructor(type,options={}) {
    const {bubbles=false,cancelable=true,...fields}=options;
    super(type,{bubbles,cancelable});
    Object.assign(this,{button:0,buttons:type==='mouseup'?0:1,clientX:2,clientY:2,metaKey:false,ctrlKey:false,shiftKey:false,altKey:false,...fields});
  }
  get target(){return this.origin??super.target;}
}

function fixture({tracking='vt200',selected=false,hovered=true}={}) {
  const doc=new EventTarget(),view=new EventTarget(),box=new EventTarget(),calls=[],program=[];
  doc.defaultView=view;view.MouseEvent=Mouse;box.ownerDocument=doc;box.contains=target=>target===box;
  const term={modes:{mouseTrackingMode:tracking},hasSelection:()=>selected,focus(){},_core:{linkifier:{currentLink:undefined}}};
  const link={text:'target',range:{start:{x:1,y:1},end:{x:20,y:1}},activate:event=>{if(canActivateTerminalLink(event,term))calls.push(linkIntent(event));}};
  if(hovered)term._core.linkifier.currentLink={link};
  const stop=claimTerminalLinkClicks(term,box,(x,y)=>({col:Math.floor(x),row:Math.floor(y/10)}),10);
  // Model the target's downstream xterm listeners, after outer capture.
  box.addEventListener('mousedown',()=>program.push('down'));
  doc.addEventListener('mousemove',()=>program.push('move'));
  doc.addEventListener('mouseup',event=>{program.push('up');link.activate(event);});
  const send=(type,options={})=>{
    const event=new Mouse(type,{origin:box,...options});
    (type==='mousedown'?box:doc).dispatchEvent(event);
    return event;
  };
  return {doc,view,box,term,link,calls,program,send,stop};
}

let passed=0;
function test(name,run){run();passed++;console.log('PASS',name);}

for(const tracking of ['vt200','drag','any'])for(const modifiers of [{},{metaKey:true},{ctrlKey:true}])test(`${tracking} resolved link uses ${JSON.stringify(modifiers)} once without program mouse input`,()=>{
  const f=fixture({tracking});f.send('mousedown',modifiers);f.send('mouseup',modifiers);
  assert.deepEqual(f.calls,[linkIntent(modifiers)]);assert.deepEqual(f.program,[]);f.stop();
});
test('ordinary text still belongs to the program',()=>{const f=fixture({hovered:false});f.send('mousedown');f.send('mouseup');assert.deepEqual(f.program,['down','up']);assert.deepEqual(f.calls,[]);f.stop();});
for(const modifiers of [{button:2},{shiftKey:true},{altKey:true}])test('right click and forced selection are not claimed '+JSON.stringify(modifiers),()=>{
  const f=fixture();f.send('mousedown',modifiers);f.send('mouseup',modifiers);assert.deepEqual(f.program,['down','up']);assert.deepEqual(f.calls,[]);f.stop();
});
test('existing selection is retained instead of opening',()=>{const f=fixture({selected:true});f.send('mousedown');f.send('mouseup');assert.deepEqual(f.program,['down','up']);assert.deepEqual(f.calls,[]);f.stop();});
test('a small move remains a click with one destination',()=>{const f=fixture();f.send('mousedown');f.send('mousemove',{clientX:5});f.send('mouseup',{clientX:5});assert.deepEqual(f.calls,['internal']);assert.deepEqual(f.program,[]);f.stop();});
test('dragging hands the original press back and never opens',()=>{const f=fixture();f.send('mousedown');f.send('mousemove',{clientX:16});f.send('mouseup',{clientX:16});assert.deepEqual(f.program,['down','move','up']);assert.deepEqual(f.calls,[]);f.stop();});
test('a drag cannot open if tracking ends before release',()=>{const f=fixture();f.send('mousedown');f.send('mousemove',{clientX:16});f.term.modes.mouseTrackingMode='none';f.send('mouseup',{clientX:16});assert.deepEqual(f.calls,[]);f.stop();});
test('release on another cell does not open the old link',()=>{const f=fixture();f.send('mousedown');f.send('mouseup',{clientX:25});assert.deepEqual(f.calls,[]);assert.deepEqual(f.program,[]);f.stop();});
test('changed provider result cannot open the old target',()=>{const f=fixture();f.send('mousedown');f.term._core.linkifier.currentLink=undefined;f.send('mouseup');assert.deepEqual(f.calls,[]);f.stop();});
test('release outside the terminal does not open',()=>{const f=fixture();f.send('mousedown');f.send('mouseup',{origin:f.doc});assert.deepEqual(f.calls,[]);f.stop();});
test('blur cancels a pending claimed click',()=>{const f=fixture();f.send('mousedown');f.view.dispatchEvent(new Event('blur'));f.send('mouseup');assert.deepEqual(f.calls,[]);assert.deepEqual(f.program,[]);f.stop();});
test('dispose removes the ownership listeners',()=>{const f=fixture();f.send('mousedown');f.stop();f.send('mousedown');f.send('mouseup');assert.deepEqual(f.program,['down','up']);assert.deepEqual(f.calls,[]);});
test('a plain shell keeps its normal downstream click handling',()=>{const f=fixture({tracking:'none'});f.send('mousedown');f.send('mouseup');assert.deepEqual(f.program,['down','up']);assert.deepEqual(f.calls,['internal']);f.stop();});
console.log(`terminal link gesture: ${passed} event harness checks passed`);
