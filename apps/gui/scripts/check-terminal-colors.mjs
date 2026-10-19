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

{
// Paint remains separate from PTY data. Exercise the pinned renderer adapter
// as well as the contrast maths; actual xterm is checked in the paired Lab.
function moduleAt(name,dependencies={}){
  const code=ts.transpileModule(readFileSync(new URL(`../src/lib/${name}.ts`,import.meta.url),'utf8'),{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText;
  const exports={};vm.runInNewContext(code,{exports,Float32Array,require:n=>{assert(n in dependencies,n);return dependencies[n]}});return exports;
}
const paint=moduleAt('term-contrast'),{protectTerminal}=moduleAt('term-legibility',{'./term-contrast':paint});
const {contrastRatio,readableRgb,cursorPair}=paint;
for(const bg of [0,0x1e1e1e,0x282828,0xf4f4ef,0xffffff,0x007acc,0x777777])for(const fg of [0,0x1a1a14,0xeeeee7,0xffffff,0xcdd6f4])for(const ratio of [3,4.5,7]){
  const actual=readableRgb(fg,bg,ratio),attainable=Math.max(contrastRatio(0,bg),contrastRatio(0xffffff,bg));
  assert(contrastRatio(actual,bg)+.000001>=Math.min(ratio,attainable),`text ${fg.toString(16)}/${bg.toString(16)} @ ${ratio}`);
  const pair=cursorPair(bg,fg,fg,ratio);
  assert(contrastRatio(pair.cursor,bg)>=Math.min(ratio,attainable));
  assert(contrastRatio(pair.accent,pair.cursor)>=Math.min(Math.max(4.5,ratio),Math.max(contrastRatio(0,pair.cursor),contrastRatio(0xffffff,pair.cursor))));
  assert.equal(readableRgb(fg,bg,1),fg);
}
const rgb=(n,css='#'+n.toString(16).padStart(6,'0'))=>({rgba:((n<<8)|255)>>>0,css});
const DIM=0x8000000,RGB=0x3000000,INVERSE=0x4000000;
const colors={foreground:rgb(0x1a1a14),background:rgb(0xf4f4ef),cursor:rgb(0x1a1a14,'rgba(26,26,20,.94)'),cursorAccent:rgb(0xf4f4ef),ansi:Array.from({length:256},(_,n)=>rgb(n<8?0x282828:0xf4f4ef))};
const originalPalette=JSON.stringify(colors),properties=new Map();
const makeCell=(fg=RGB|0x1a1a14,bg=RGB|0x1e1e1e|DIM,code=65)=>Object.freeze({fg,bg,isDim:()=>bg&DIM,getCode:()=>code});
let cell=makeCell(),drawn;
const publicCell={getFgColorMode:()=>cell.fg&RGB,getBgColorMode:()=>cell.bg&RGB,getFgColor:()=>cell.fg&0xffffff,getBgColor:()=>cell.bg&0xffffff,isInverse:()=>cell.fg&INVERSE};
const service={_renderer:{},setRenderer(r){this._renderer.value=r}};
const term={_core:{_renderService:service},options:{minimumContrastRatio:4.5,drawBoldTextInBrightColors:true},cols:80,rows:24,buffer:{active:{baseY:0,cursorX:0,cursorY:0,getLine:()=>({getCell:()=>publicCell})}},element:{style:{setProperty:(k,v)=>properties.set(k,v),removeProperty:k=>properties.delete(k)}},refresh(){}};
function dom(){return {_themeService:{colors},_injectCss(c){this.css=c},_rowFactory:{_applyMinimumContrast(el,bg,fg,c){el.style.color='native';return false}},renderRows(){drawn={cursor:colors.cursor.css,accent:colors.cursorAccent.css}}};}
let renderer=dom();service.setRenderer(renderer);
const originalSet=service.setRenderer,originalApply=renderer._rowFactory._applyMinimumContrast,originalRender=renderer.renderRows;
const stopPaint=protectTerminal(term),el={style:{}};
renderer._rowFactory._applyMinimumContrast(el,rgb(0x1e1e1e),rgb(0x1a1a14),cell,rgb(0x1e1e1e));
assert(contrastRatio(parseInt(el.style.color.slice(1),16),0x1e1e1e)>=4.5,'DIM uses full target');
const translucentBg={rgba:((0x1a1a14<<8)|240)>>>0,css:'rgba(26,26,20,.94)'};
renderer._rowFactory._applyMinimumContrast(el,translucentBg,colors.background,cell);
assert(contrastRatio(parseInt(el.style.color.slice(1),16),paint.mixRgb(0x1a1a14,0xf4f4ef,240/255))>=4.5,'DOM inverse ground includes palette alpha');
renderer.renderRows(0,23);
assert.equal(properties.get('--khor-term-cursor'),'#ffffff','dark input gets visible cursor');
assert.equal(JSON.stringify(colors),originalPalette,'colour query values stay original');
term.options.minimumContrastRatio=1;renderer.renderRows(0,23);
assert.equal(properties.get('--khor-term-cursor'),colors.cursor.css,'off retains cursor alpha');
renderer._rowFactory._applyMinimumContrast(el,colors.background,colors.foreground,cell);
assert.equal(el.style.color,'native','off delegates DIM to xterm');
term.options.minimumContrastRatio=4.5;
renderer._rowFactory._applyMinimumContrast(el,colors.background,colors.foreground,makeCell(0,DIM,0x2500));
assert.equal(el.style.color,'native','decorative glyphs retain xterm exclusions');
const old=renderer,resolver={result:{fg:0,bg:0,ext:0},resolve(c){this.result.fg=c.fg;this.result.bg=c.bg}};
const originalResolve=resolver.resolve,originalFloat=new Float32Array([.1,.1,.1,1]);
renderer={_themeService:{colors},_cellColorResolver:resolver,_rectangleRenderer:{value:{_cursorFloat:originalFloat}},renderRows(){resolver.resolve(cell,0,0,1);drawn={cursor:colors.cursor,accent:colors.cursorAccent,float:[...this._rectangleRenderer.value._cursorFloat],fg:resolver.result.fg,bg:resolver.result.bg}}};
service.setRenderer(renderer);
assert.equal(old._rowFactory._applyMinimumContrast,originalApply,'retired renderer released');assert.equal(old.renderRows,originalRender);
for(const ratio of [3,4.5,7]){
  term.options.minimumContrastRatio=ratio;
  for(const [fg,bg] of [[RGB|0x1a1a14,RGB|0x1e1e1e|DIM],[RGB|0xeeeee7,RGB|0xf4f4ef|DIM],[INVERSE,DIM],[0x1000001,0x1000007|DIM]]){
    cell=makeCell(fg,bg);const before=JSON.stringify(cell);renderer.renderRows(0,23);
    assert(!(drawn.bg&DIM),'render scratch DIM cleared');assert(!(drawn.fg&INVERSE),'scratch inverse resolved');
    assert(contrastRatio(drawn.fg&0xffffff,drawn.bg&0xffffff)+.000001>=ratio);
    assert.equal(JSON.stringify(cell),before,'buffer unchanged');assert.equal(JSON.stringify(colors),originalPalette,'palette restored');assert.equal(renderer._rectangleRenderer.value._cursorFloat,originalFloat);
  }
}
term.options.minimumContrastRatio=1;renderer.renderRows(0,23);
assert.equal(drawn.fg,cell.fg);assert.equal(drawn.bg,cell.bg);assert.equal(drawn.cursor,colors.cursor);
const savedCursor=colors.cursor;
const broken={_themeService:{colors},_cellColorResolver:{result:{fg:0,bg:0,ext:0},resolve(){}},renderRows(){assert.notEqual(colors.cursor,savedCursor);throw Error('draw failure')}};term.options.minimumContrastRatio=4.5;service.setRenderer(broken);
assert.equal(resolver.resolve,originalResolve);assert.throws(()=>broken.renderRows(0,23),/draw failure/);assert.equal(JSON.stringify(colors),originalPalette);
stopPaint();stopPaint();assert.equal(service.setRenderer,originalSet);assert.equal(properties.size,0);
console.log('terminal paint: full DIM floor, adaptive cursor, RGB/ANSI/inverse, off, palette/buffer isolation, renderer fallback and teardown passed');

}
