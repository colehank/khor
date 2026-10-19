// Production capture listeners: candidates own the whole pan, not only bubbling.
import assert from 'node:assert/strict';
import {build} from 'esbuild';
const saved=Object.fromEntries(['document','Element','HTMLElement','HTMLInputElement','HTMLTextAreaElement','performance'].map(k=>[k,globalThis[k]]));
const doc=new EventTarget();
class ElementFixture extends EventTarget {
 constructor(parent=null,matches=[]){super();this.parent=parent;this.selectors=matches;}
 closest(selector){return selector.split(',').some(s=>this.selectors.includes(s.trim()))?this:this.parent?.closest(selector)||null;}
}
class InputFixture extends ElementFixture {type='text';isContentEditable=false;blur(){doc.activeElement=null;}}
try{
 Object.assign(globalThis,{document:doc,Element:ElementFixture,HTMLElement:ElementFixture,HTMLInputElement:InputFixture,HTMLTextAreaElement:class extends ElementFixture{},performance:{now:()=>0}});
 const bundle=await build({stdin:{contents:"export {followFieldSwipe} from './src/lib/field-swipe';",resolveDir:process.cwd(),loader:'ts'},bundle:true,format:'esm',write:false,define:{'import.meta.env':'{}'}});
 const {followFieldSwipe}=await import('data:text/javascript;base64,'+Buffer.from(bundle.outputFiles[0].text).toString('base64'));
 const stop=followFieldSwipe(),field=new InputFixture(),body=new ElementFixture();
 const touch=(type,target,x,y,count=1)=>{const e=new Event(type,{cancelable:true});Object.defineProperties(e,{target:{value:target},touches:{value:Array.from({length:count},()=>({clientX:x,clientY:y}))}});doc.dispatchEvent(e);assert(!e.defaultPrevented,'passive policy never prevents scrolling');};
 for(const selector of ['[data-sheet-hold]','[data-menu-frame]','[data-menu-viewport]','[role="listbox"]','[role="menu"]','input[type="range"]']){
  const row=new ElementFixture(new ElementFixture(null,[selector]));
  doc.activeElement=field;touch('touchstart',row,100,100);touch('touchmove',row,100,180);assert.equal(doc.activeElement,field,selector+' lost keyboard');
  touch('touchmove',body,100,250);assert.equal(doc.activeElement,field,'leaving menu must not transfer gesture');touch('touchend',body,100,250,0);
 }
 doc.activeElement=field;touch('touchstart',body,100,100);touch('touchmove',body,100,180);assert.equal(doc.activeElement,null,'main content must still dismiss');
 doc.activeElement=field;touch('touchstart',body,100,100);touch('touchmove',body,180,110);assert.equal(doc.activeElement,field,'horizontal gestures keep input');touch('touchcancel',body,180,110,0);
 doc.activeElement=field;touch('touchstart',body,100,100,2);touch('touchmove',body,100,200,2);assert.equal(doc.activeElement,field,'multi-touch keeps input');
 const terminal=new InputFixture(new ElementFixture(null,['[data-terminal]']));doc.activeElement=terminal;touch('touchstart',body,100,100);touch('touchmove',body,100,200);assert.equal(doc.activeElement,terminal,'terminal keeps its own keyboard rules');
 stop();doc.activeElement=field;touch('touchstart',body,100,100);touch('touchmove',body,100,180);assert.equal(doc.activeElement,field,'cleanup removes listeners');
 console.log('field interaction: menu pan ownership, boundary exit, body dismissal, horizontal/multiple touches, terminal isolation and cleanup passed');
}finally{Object.assign(globalThis,saved);}
