// Real production geometry functions; no browser, native views or mock API.
import assert from 'node:assert/strict';
import {execFileSync} from 'node:child_process';
import fs from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';
const source=fs.readFileSync(new URL('../src/views/BrowserPane.tsx',import.meta.url),'utf8');
const compile=source=>ts.transpileModule(source,{compilerOptions:{target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.None}}).outputText;
if(process.argv.includes('--legacy')){
 const old=execFileSync('git',['show','2ba5e983:apps/gui/src/views/BrowserPane.tsx'],{encoding:'utf8'});
 const file=ts.createSourceFile('old.tsx',old,ts.ScriptTarget.Latest,true,ts.ScriptKind.TSX);let initializer;
 function visit(node){if(ts.isVariableDeclaration(node)&&node.name.getText(file)==='at'&&node.initializer?.getText(file).includes('native.getBoundingClientRect().bottom')){assert.equal(initializer,undefined);initializer=node.initializer.getText(file);}ts.forEachChild(node,visit);}visit(file);assert(initializer);
 const context=vm.createContext({r:{left:0,top:80,width:390,bottom:844},native:{getBoundingClientRect:()=>({top:660,bottom:844})},window:{innerHeight:844}});
 const actual=vm.runInContext(compile(`(${initializer})`),context);
 assert.equal(actual.h,580,'the existing native viewport must end at deck.top, not extend to deck.bottom');
 process.exit(0);
}
const file=ts.createSourceFile('source.tsx',source,ts.ScriptTarget.Latest,true,ts.ScriptKind.TSX);
const functions=[];for(const node of file.statements)if(ts.isFunctionDeclaration(node)&&['pageBox','hostBox'].includes(node.name?.text))functions.push(node.getText(file).replace(/^export /,''));assert.equal(functions.length,2);
const context=vm.createContext({window:{innerHeight:844}});vm.runInContext(compile(functions.join('\n')+'\nglobalThis.geometry={pageBox,hostBox};'),context);
const {pageBox,hostBox}=context.geometry;let passed=0;function check(name,run){run();console.log('PASS',name);passed++;}
const rect=(top,bottom,width=390,left=0)=>({left,top,bottom,width,height:bottom-top});
const result=(...args)=>({...pageBox(...args)});
check('expanded deck is outside the actual native page viewport',()=>assert.deepEqual(result(rect(80,844),660,844),{x:0,y:80,w:390,h:580,view:844}));
check('folded deck releases only its real layout height',()=>assert.equal(pageBox(rect(80,844),780,844).h,700));
check('desktop without a deck preserves its original host rectangle',()=>assert.deepEqual(result(rect(60,760,900,240),null,800),{x:240,y:60,w:900,h:700,view:800}));
check('resized keyboard viewport and safe-area origin retain correct coordinates',()=>assert.deepEqual(result(rect(44,420,358,16),320,420),{x:16,y:44,w:358,h:276,view:420}));
check('overlapping/smaller hosts never invent height or extend below themselves',()=>{assert.equal(pageBox(rect(400,500),300,600).h,0);assert.equal(pageBox(rect(40,240),500,600).h,200);});
check('the same production host measurement uses whichever visible deck is higher',()=>{
 const el={getBoundingClientRect:()=>rect(80,844),closest:selector=>({querySelector:()=>({getBoundingClientRect:()=>selector==='[data-session-screen]'?rect(700,844):rect(660,760)})})};
 assert.equal(hostBox(el).h,580);
 el.closest=selector=>({querySelector:()=>({getBoundingClientRect:()=>selector==='[data-session-screen]'?rect(700,844):rect(0,0,0)})});
 assert.equal(hostBox(el).h,620,'a hidden retained local deck has no geometry');
});
// Compose the actual JS deck-bounds producer with the native pure CGRect
// expression. CGRect constructors are data-only fixtures, not UIKit paint.
const drawingSource=fs.readFileSync(new URL('../src/lib/native-deck.ts',import.meta.url),'utf8');
const drawingAst=ts.createSourceFile('deck.ts',drawingSource,ts.ScriptTarget.Latest,true,ts.ScriptKind.TS);
let bounds;
function visitDrawing(node){
 if(ts.isReturnStatement(node)&&node.expression&&ts.isObjectLiteralExpression(node.expression)){
  const names=new Map(node.expression.properties.filter(ts.isPropertyAssignment).map(p=>[p.name.getText(drawingAst),p.initializer.getText(drawingAst)]));
  if(['x','y','w','h','inset'].every(key=>names.has(key))){assert.equal(bounds,undefined);bounds=Object.fromEntries(['x','y','w','h','inset'].map(key=>[key,names.get(key)]));}
 }
 ts.forEachChild(node,visitDrawing);
}visitDrawing(drawingAst);assert(bounds,'real readDeck serializes the actual deck bounds');
const nativeSource=fs.readFileSync(new URL('../src-tauri/src/phone_page.rs',import.meta.url),'utf8');
const expression=/fn deck_frame\(origin: NSPoint, geometry: NSRect\) -> NSRect \{([\s\S]*?)\n\}/.exec(nativeSource)?.[1].trim();assert(expression);
const create=/initWithFrame: (deck_frame\(origin, geometry\))/.exec(nativeSource)?.[1];
const move=/setFrame: (deck_frame\(origin, k\.geometry\))/.exec(nativeSource)?.[1];assert(create);assert(move,'create and move use the same DOM-frame mapping');
assert(nativeSource.includes('page.viewport_origin = parent_frame.origin;'),'placement updates the coordinate origin rather than deriving deck top from clipped WK height');
const native=vm.createContext({NSPoint:{new:(x,y)=>({x,y})},NSRect:{new:(origin,size)=>({origin,size})}});
vm.runInContext(`function deck_frame(origin,geometry){return ${expression.replaceAll('NSRect::new','NSRect.new').replaceAll('NSPoint::new','NSPoint.new')};}globalThis.frame=deck_frame;`,native);
const produce=o=>{const value=vm.runInNewContext(`({${Object.entries(bounds).map(([k,v])=>`${k}:${v}`).join(',')}})`,{o});return{...value};};
if(process.argv.includes('--legacy-deck')){
 const old=execFileSync('git',['show','4eb36d47:apps/gui/src-tauri/src/phone_page.rs'],{encoding:'utf8'});
 const oldArgs=/initWithFrame: rect\((pf\.origin\.x, pf\.origin\.y \+ pf\.size\.height - dr\.h, pf\.size\.width, dr\.h)\)/.exec(old)?.[1];assert(oldArgs);
 const dr=produce(rect(660,844)),body=pageBox(rect(80,844),660,844),pf={origin:{x:0,y:body.y},size:{width:body.w,height:body.h}};
 const frame=vm.runInNewContext(`rect(${oldArgs})`,{pf,dr,rect:(x,y,w,h)=>({x,y,w,h})});
 assert.equal(frame.y,660,'the clipped native page bottom is deck.top; the old native formula must not lift the deck by its height');
 process.exit(0);
}
for(const [name,bodyRect,deckRect,surface] of [
 ['portrait/safe-area',rect(80,844,358,16),rect(660,844),{x:3,y:22}],
 ['folded',rect(80,844),rect(780,844),{x:0,y:0}],
 ['keyboard',rect(44,420),rect(320,420),{x:0,y:-12}],
])check(`DOM viewport and native deck create/move compose without overlap: ${name}`,()=>{
 const body=pageBox(bodyRect,deckRect.top,844),drawing=produce(deckRect);assert.equal(drawing.inset,0);
 const geometry={origin:{x:drawing.x,y:drawing.y},size:{width:drawing.w,height:drawing.h}};
 const created=vm.runInNewContext(create,{deck_frame:native.frame,origin:surface,geometry});
 assert.equal(created.origin.y,surface.y+body.y+body.h);assert.equal(created.origin.y+created.size.height,surface.y+deckRect.bottom);assert.equal(created.size.width,deckRect.width);
 const movedOrigin={x:surface.x+7,y:surface.y+9};
 const moved=vm.runInNewContext(move,{deck_frame:native.frame,origin:movedOrigin,k:{geometry}});
 assert.equal(moved.origin.y,movedOrigin.y+body.y+body.h);assert.equal(moved.origin.x,movedOrigin.x+deckRect.left);assert.equal(moved.size.width,drawing.w);
});
console.log(`${passed} native page viewport/deck composition checks passed; controlled CGRect arithmetic, no WK`);
