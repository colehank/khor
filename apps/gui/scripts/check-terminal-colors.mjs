// Resolve the real viewer configuration in an isolated environment. Browser
// rendering is verified separately with the actual terminal components.
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import vm from 'node:vm';
import ts from 'typescript';
const source=readFileSync(new URL('../src/lib/term-theme.ts',import.meta.url),'utf8');
const presets=JSON.parse(readFileSync(new URL('../src/lib/term-palettes.json',import.meta.url),'utf8'));
const js=ts.transpileModule(source,{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText;
function load(storage=new Map()){
  const root={dataset:{theme:'light'},style:{values:new Map(),setProperty(k,v){this.values.set(k,v)},removeProperty(k){this.values.delete(k)}}};
  const events=new Map(),heard=new Map(),observers=[];
  const window={localStorage:{getItem:k=>storage.get(k)||null,setItem:(k,v)=>storage.set(k,v)},matchMedia:()=>({matches:false,addEventListener(){},removeEventListener(){}}),addEventListener(k,fn){const list=events.get(k)||new Set();list.add(fn);events.set(k,list)},removeEventListener(k,fn){events.get(k)?.delete(fn)},dispatchEvent(e){for(const fn of events.get(e.type)||[])fn(e)}};
  class MutationObserver{constructor(fn){this.fn=fn}observe(){observers.push(this)}disconnect(){}}
  const context={exports:{},window,document:{documentElement:root},MutationObserver,Event:class{constructor(type){this.type=type}},structuredClone,crypto:{randomUUID:()=>String(storage.size)},getComputedStyle:()=>({color:root.dataset.theme==='dark'?'rgb(238, 238, 231)':'rgb(26, 26, 20)',getPropertyValue:k=>k==='--canvas-solid'?(root.dataset.theme==='dark'?'#000000':'#f4f4ef'):''}),require:name=>{
    if(name==='react')return {useEffect(){},useState(){}};
    if(name.includes('other-windows'))return {heardFrom:(k,fn)=>heard.set(k,fn)};
    if(name.endsWith('term-palettes.json'))return {default:presets};
    throw Error(name);
  }};
  vm.runInNewContext(js,context);
  const api=context.exports;
  return {api,root,storage,mode(value){root.dataset.theme=value;for(const o of [...observers])o.fn([])},other(value){storage.set(api.TERM_THEME_KEY,JSON.stringify(value));heard.get(api.TERM_THEME_KEY)()}};
}
const env=load(),p=env.api,el={};
assert.equal(typeof p.terminalAppearance,'function','one resolver must supply theme and contrast to every terminal');
const fresh=p.terminalAppearance(el);
assert.equal(fresh.minimumContrastRatio,4.5);
assert.equal(fresh.theme.background,'#f4f4ef');
env.mode('dark');
const dark=p.terminalAppearance(el);
assert.equal(dark.theme.background,'#000000');
assert.notEqual(fresh.theme.red,dark.theme.red,'Follow needs separate light/dark ANSI colours');
assert.notEqual(fresh.theme.yellow,dark.theme.yellow);
assert.equal(fresh.theme.red,presets.latte.theme.red);
assert.equal(dark.theme.red,presets.mocha.theme.red);
assert.equal(fresh.theme.extendedAnsi,undefined,'the standard 16–255 cube stays standard');

// An existing saved choice stays fixed and acquires the safe default.
p.setTermTheme({selected:'nord',customs:[]});
env.mode('light');assert.equal(p.terminalAppearance(el).theme.background,presets.nord.theme.background);
assert.equal(p.terminalAppearance(el).minimumContrastRatio,4.5);
p.setTermTheme({...p.termTheme(),contrast:1});
assert.equal(load(env.storage).api.terminalAppearance(el).minimumContrastRatio,1,'explicit off survives a restart');
for(const value of [NaN,Infinity,-1,0,2,21,'7',null]){
  p.setTermTheme({...p.termTheme(),contrast:value});
  assert.equal(p.terminalAppearance(el).minimumContrastRatio,4.5,'invalid contrast: '+value);
}

for(const [id,light,dark] of [['auto-catppuccin','latte','mocha'],['auto-tokyo','tokyo-day','tokyo']]){
  p.setTermTheme({selected:id,customs:[],contrast:7});
  env.mode('light');assert.equal(p.terminalAppearance(el).theme.background,presets[light].theme.background);
  assert.equal(env.root.style.values.get('--term-palette-bg'),presets[light].theme.background);
  env.mode('dark');assert.equal(p.terminalAppearance(el).theme.background,presets[dark].theme.background);
  assert.equal(env.root.style.values.get('--term-palette-bg'),presets[dark].theme.background,'CSS surface changes with family');
  assert.equal(p.terminalAppearance(el).minimumContrastRatio,7);
  assert.equal(p.clonePalette(el).foreground,presets[dark].theme.foreground);
}

// Export/clone remains pure colour data. Preview overrides keep the viewer's
// protection without changing the selected theme, stream or upstream data.
const before=JSON.stringify(presets),selected=p.termTheme().selected;
const copy=p.clonePalette(el);copy.extendedAnsi=['#123456'];
const preview=p.terminalAppearance(el,p.termTheme(),copy);
preview.theme.extendedAnsi[0]='#abcdef';
assert.equal(copy.extendedAnsi[0],'#123456');
assert.equal(p.termTheme().selected,selected);
assert.equal(JSON.stringify(presets),before);
assert(!('minimumContrastRatio' in p.clonePalette(el)));
const custom=p.savePalette('Mine',copy);
assert.equal(p.termTheme().contrast,7,'saving a palette preserves independent protection');
assert.equal(p.terminalAppearance(el).theme.extendedAnsi[0],'#123456');
env.mode('light');assert.equal(p.termTheme().selected,custom,'DIY is not auto-converted');

let notices=0;const stop=p.onTermTheme(()=>notices++);
env.other({selected:'auto-catppuccin',customs:[],contrast:3});
assert.equal(p.terminalAppearance(el).minimumContrastRatio,3);
assert.equal(p.terminalAppearance(el).theme.background,presets.latte.theme.background);
env.mode('dark');assert.equal(p.terminalAppearance(el).theme.background,presets.mocha.theme.background);
assert(notices>=2,'other-window and theme changes notify retained viewers');stop();
console.log('terminal colours: paired/fixed/DIY, default/off/invalid contrast, surface updates, cross-window/reload, isolated preview and standard colour cube passed');
