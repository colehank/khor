import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {transform} from 'esbuild';
const source=readFileSync('src/views/CommitPage.tsx','utf8');
const jump=source.match(/  const jump = \(path: string\) => \{[\s\S]*?\n  \};/)?.[0];assert(jump);
const code=(await transform(jump,{loader:'ts'})).code;
const results=[];
function test(name,run){try{run();results.push({name,pass:true});console.log('PASS',name);}catch(error){results.push({name,pass:false,error:error.message});console.error('FAIL',name,error.message);}}
function fixture(padding){
 const box={scrollTop:100,getBoundingClientRect:()=>({top:72}),querySelectorAll:()=>sections};
 const sections=[512,1800].map((position,i)=>({dataset:{commitFile:'file-'+i},getBoundingClientRect:()=>({top:position-box.scrollTop+72}),focus(options){assert.equal(options.preventScroll,true);}}));
 const run=Function('scroller','getComputedStyle',code+';return jump;')({current:box},()=>({scrollPaddingTop:padding}));
 return {box,sections,run};
}
test('a primary band leaves the selected file body below its visible title',()=>{const f=fixture('52px');f.run('file-0');const sectionTop=f.sections[0].getBoundingClientRect().top;assert.equal(sectionTop,124);assert(sectionTop+32>=124+32,'first body line must not land under the held file title');});
test('a local port keeps zero-inset navigation without inherited primary padding',()=>{const f=fixture('auto');f.run('file-0');assert.equal(f.sections[0].getBoundingClientRect().top,72);});
test('repeated file jumps preserve the same visible destination after scrolling',()=>{const f=fixture('52px');f.run('file-0');const first=f.box.scrollTop;f.run('file-1');assert.equal(f.sections[1].getBoundingClientRect().top,124);f.run('file-0');assert.equal(f.box.scrollTop,first);});
test('an absent file target leaves the reading position unchanged',()=>{const f=fixture('52px');f.run('missing');assert.equal(f.box.scrollTop,100);});
console.log(JSON.stringify({passed:results.filter(v=>v.pass).length,total:results.length,results},null,2));assert(results.every(v=>v.pass),'commit sticky jump regression');
