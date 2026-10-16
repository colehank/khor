// The keyboard changes usable height; the platform's top inset remains.
// Exercise the production geometry independently from DOM event routing.
import assert from 'node:assert/strict';
import {build} from 'esbuild';
const bundle=await build({entryPoints:['src/lib/content-sheet-bounds.ts'],bundle:true,write:false,format:'esm',platform:'node'});
const {contentSheetHeights}=await import(`data:text/javascript;base64,${Buffer.from(bundle.outputFiles[0].text).toString('base64')}`);
const shares={half:.5,full:.92};
const read=(height,top=67,foot=0)=>contentSheetHeights({height,top,foot},shares);
// A tall portrait screen keeps its preferred detents; the same phone
// above its keyboard must leave 59 platform points plus 8 of breathing room.
assert.deepEqual(read(844),{half:422,full:776.48});
assert.deepEqual(read(500),{half:250,full:433});
assert.deepEqual(read(500,67,30),{half:220,full:403});
assert.deepEqual(read(40),{half:0,full:0});
assert.deepEqual(read(100),{half:33,full:33});
assert.deepEqual(read(0),{half:0,full:0});
assert.deepEqual(read(500,0),{half:250,full:460});
// Invariants across keyboard motion, narrow landscape and insets. These
// assert available space, not a duplicated preferred-detent formula.
for(const top of [0,20,59,67,88,140])for(const foot of [0,34,100]){
  let previous={half:0,full:0};
  for(let height=0;height<=1000;height++){
    const current=read(height,top,foot),available=Math.max(0,height-top-foot);
    for(const h of Object.values(current)){
      assert(Number.isFinite(h)&&h>=0&&h<=available,'detent invaded platform space');
    }
    assert(current.half<=current.full,'half exceeded full');
    assert(current.half>=previous.half&&current.full>=previous.full,'more room made a smaller detent');
    previous=current;
  }
}
assert.deepEqual(read(-1),{half:0,full:0});
assert.deepEqual(read(Number.NaN),{half:0,full:0});
console.log('PASS: content sheet keeps platform top and bottom clear across keyboard heights, with ordered nonnegative detents');
