// Semantic presentation and physical limits, independent of CSS/DOM routing.
import assert from 'node:assert/strict';
import {build} from 'esbuild';
const bundle=await build({entryPoints:['src/lib/overlay-policy.ts'],bundle:true,write:false,platform:'node',format:'esm'});
const {overlayPolicy,overlayHeights}=await import(`data:text/javascript;base64,${Buffer.from(bundle.outputFiles[0].text).toString('base64')}`);
const select=overlayPolicy({purpose:'selection'},true);
assert.equal(select.presentation,'sheet');assert.equal(select.initial,'medium');assert.equal(select.maximum,'large');assert.equal(select.focus,false);
assert.equal(overlayPolicy({purpose:'selection'},false).presentation,'dialog');
const edit=overlayPolicy({purpose:'edit'},true);
assert.equal(edit.initial,'fit');assert.equal(edit.maximum,'fit');assert.equal(edit.resizable,false);assert.equal(edit.outside,true);
assert.equal(overlayPolicy({purpose:'edit'},false).outside,false);
assert.equal(overlayPolicy({purpose:'edit',dismiss:'outside'},false).outside,true);
for(const narrow of [false,true]){
 const confirmation=overlayPolicy({purpose:'confirmation'},narrow);
 assert.equal(confirmation.presentation,'dialog');assert.equal(confirmation.width,'compact');
 const protectedEdit=overlayPolicy({purpose:'edit',dismiss:'explicit'},narrow);
 assert.equal(protectedEdit.outside,false);assert.equal(protectedEdit.dismissible,false);
}
assert.equal(overlayPolicy({purpose:'actions'},true).presentation,'actions');
const preview=overlayPolicy({purpose:'preview'},true);assert.equal(preview.initial,'large');assert.equal(preview.maximum,'large');
const short=overlayHeights({height:760,top:24},{initial:'fit',maximum:'fit',content:180});
assert.deepEqual(short,{half:180,full:180});
const large=overlayHeights({height:760,top:24},{initial:'medium',maximum:'large',content:2000});
assert.deepEqual(large,{half:380,full:736});
for(const initial of ['fit','medium','large'])for(const maximum of ['fit','medium','large'])for(const top of [0,24,88])for(const foot of [0,34,336]){
 let previous={half:0,full:0};
 for(let height=0;height<=1000;height+=5){
  const result=overlayHeights({height,top,foot},{initial,maximum,content:480});
  const room=Math.max(0,height-top-foot);
  assert(Number.isFinite(result.half)&&Number.isFinite(result.full));
  assert(result.half>=0&&result.half<=result.full&&result.full<=room);
  assert(result.half>=previous.half&&result.full>=previous.full,'more room lost a detent');
  previous=result;
 }
}
assert.deepEqual(overlayHeights({height:NaN},{initial:'medium',maximum:'large',content:NaN}),{half:0,full:0});
console.log('PASS: overlay purpose, responsive defaults, dismissal, independent initial/maximum stops, safe-area and keyboard bounds');
