import * as engine from './renderer.ts';
import type { Control, ControlGroup, ControlKey, IconParameters, IconPreset } from './renderer.ts';

type ElementConstructor<T extends Element> = { new(): T };
function requiredElement<T extends Element>(node: Element | null, type: ElementConstructor<T>, description: string): T {
  if (!(node instanceof type)) throw new Error(`缺少调校控件：${description}`);
  return node;
}
const $ = (id: string) => requiredElement(document.getElementById(id), HTMLElement, id);
const button = (id: string) => requiredElement(document.getElementById(id), HTMLButtonElement, id);
const input = (id: string) => requiredElement(document.getElementById(id), HTMLInputElement, id);
const select = (id: string) => requiredElement(document.getElementById(id), HTMLSelectElement, id);
const query = (selector: string) => requiredElement(document.querySelector(selector), HTMLElement, selector);
function all<T extends Element>(selector: string, type: ElementConstructor<T>): T[] {
  return Array.from(document.querySelectorAll(selector), node => requiredElement(node, type, selector));
}
function isOneOf<T extends string>(value: unknown, choices: readonly T[]): value is T {
  return typeof value === 'string' && choices.some(choice => choice === value);
}
function errorMessage(error: unknown): string {
  if (error instanceof Error) return error.message;
  return String(error);
}
window.addEventListener('unhandledrejection', event => {
  const reason: unknown = event.reason;
  $('notice').textContent = `调校台无法完成操作：${errorMessage(reason)}`;
  $('notice').hidden = false;
});
const VIEWS = ['single', 'compare', 'snapshot'] as const;
type View = typeof VIEWS[number];
type MobileGroup = ControlGroup | 'tools';
type Shape = 'rounded' | 'square';
const GROUNDS = ['paper', 'dark', 'gray'] as const;
type Ground = typeof GROUNDS[number];
interface StudioState {
  parameters: IconParameters;
  snapshot: IconParameters | null;
  view: View;
  mobileView: View;
  mobileGroup: MobileGroup;
  shape: Shape;
  ground: Ground;
}
interface SavedState {
  preset: IconPreset;
  snapshot: IconPreset | null;
  view: View;
  mobileView: View;
  mobileGroup: MobileGroup;
  shape: Shape;
  ground: Ground;
}
const STORAGE = 'khor.icon-lab.v1';
const mobileMode = window.matchMedia('(max-width:760px), (hover:none) and (pointer:coarse) and (max-width:1100px)');
const response = await fetch('./current-preset.json', { cache: 'no-store' });
if (!response.ok) throw new Error('无法读取当前图标参数');
const currentPayload: unknown = await response.json();
const current = engine.parsePreset(currentPayload);
const PRESETS = {
  current,
  aligned07: { ...engine.aligned07 },
  calibrated: { ...engine.defaults },
  soft: { ...engine.aligned07, external: .80, softness: .96, bloom: .72, ambient: .70 },
  inner: { ...engine.aligned07, external: .65, emission: 1.12, bloom: .70, reflection: .28, spill: .38 },
  clear: { ...engine.aligned07, glass: .30, outerGlass: .50, transparency: .88, frost: .03, edge: 1.12, transmission: .45 }
};
const GROUPS: readonly (readonly [ControlGroup, string, string, boolean])[] = [
  ['external', '外部柔光', '白光穿过坛城，另一侧透出绿色；透出方向自动跟随外光。', true],
  ['internal', '内部光源', '总发光同时作用于五颗圆珠和内环。', true],
  ['glass', '玻璃', '菱形与外环为玻璃；映光以柔化色团晕开。', false],
  ['background', '背景', '本组调节内光与背景；外光穿透后的绿色在“外光”调节。', false],
  ['geometry', '形体与表面', '保留 07 的构图，只微调厚度与圆珠表面。', false]
];
const presetKeys = Object.keys(PRESETS) as (keyof typeof PRESETS)[];
const state: StudioState = { parameters: { ...current }, snapshot: null, view: 'compare', mobileView: 'single', mobileGroup: 'external', shape: 'rounded', ground: 'paper' };
let storageAvailable = true, restored = false;
try {
  const saved = localStorage.getItem(STORAGE);
  if (saved) {
    const data: unknown = JSON.parse(saved);
    if (!engine.isRecord(data)) throw new Error('无法读取已保存的调校状态');
    state.parameters = engine.parsePreset(data.preset);
    if (data.snapshot) state.snapshot = engine.parsePreset(data.snapshot);
    state.view = isOneOf(data.view, VIEWS) ? data.view : 'compare';
    if (state.view === 'snapshot' && !state.snapshot) state.view = 'compare';
    if (isOneOf(data.mobileView, VIEWS)) state.mobileView=data.mobileView;
    if (state.mobileView==='snapshot' && !state.snapshot) state.mobileView='single';
    if (isOneOf(data.mobileGroup, [...GROUPS.map(g=>g[0]), 'tools'] as const)) state.mobileGroup=data.mobileGroup;
    state.shape = data.shape === 'square' ? 'square' : 'rounded';
    state.ground = isOneOf(data.ground, GROUNDS) ? data.ground : 'paper';
    restored = true;
  }
} catch (_) { storageAvailable = false; }
const history: IconParameters[] = [{ ...state.parameters }];
function historyEntry(index: number): IconParameters {
  const entry = history[index];
  if (!entry) throw new Error('调校历史位置无效');
  return entry;
}
let desktopView=state.view;
let historyIndex = 0, pendingFrame = 0, saveTimer = 0, noticeTimer = 0, miniTimer = 0;
const ranges = new Map<ControlKey, HTMLInputElement>(), values = new Map<ControlKey, HTMLInputElement>();
function controlInput(map: ReadonlyMap<ControlKey, HTMLInputElement>, key: ControlKey): HTMLInputElement {
  const element = map.get(key);
  if (!element) throw new Error(`缺少参数控件：${key}`);
  return element;
}
const same = (a: Readonly<IconParameters>,b: Readonly<IconParameters>) => engine.controls.every(c => a[c.id] === b[c.id]) && a.directionalSpill===b.directionalSpill;
const fmt = (c: Control, v: number) => c.step >= 1 ? String(Math.round(v)) : Number(v).toFixed(c.step < .01 ? 3 : 2);
function savedState(): SavedState {
  return { preset: engine.packagePreset(state.parameters), snapshot: state.snapshot && engine.packagePreset(state.snapshot), view: mobileMode.matches?desktopView:state.view, mobileView: state.mobileView, mobileGroup: state.mobileGroup, shape: state.shape, ground: state.ground };
}
function notice(message: string) {
  window.clearTimeout(noticeTimer);
  $('notice').textContent = message;
  $('notice').hidden = false;
  noticeTimer = window.setTimeout(() => { $('notice').hidden = true; }, 4500);
}
function persist() {
  window.clearTimeout(saveTimer);
  saveTimer = window.setTimeout(() => {
    try {
      localStorage.setItem(STORAGE, JSON.stringify(savedState()));
      storageAvailable = true;
      $('save-state').textContent = '已自动保存';
    } catch (_) {
      storageAvailable = false;
      $('save-state').textContent = '本地保存不可用 · 可导出参数';
    }
  }, 200);
}
function updateHistoryButtons() {
  button('undo').disabled = historyIndex === 0;
  button('redo').disabled = historyIndex === history.length - 1;
}
function commit() {
  if (!same(state.parameters, historyEntry(historyIndex))) {
    history.splice(historyIndex + 1);
    history.push({ ...state.parameters });
    if (history.length > 100) history.shift();
    historyIndex = history.length - 1;
  }
  updateHistoryButtons();
  persist();
}
function undo(direction: -1 | 1) {
  commit();
  const next = historyIndex + direction;
  if (next < 0 || next >= history.length) return;
  historyIndex = next;
  state.parameters = { ...historyEntry(historyIndex) };
  syncControls(); schedule(); updateHistoryButtons(); persist();
}
function detectPreset() {
  select('preset').value = presetKeys.find(key => same(PRESETS[key], state.parameters)) || 'custom';
}
function syncControls() {
  for (const c of engine.controls) {
    if(c.legacy) continue;
    controlInput(ranges, c.id).value = String(state.parameters[c.id]);
    controlInput(ranges, c.id).setAttribute('aria-valuenow',String(state.parameters[c.id]));
    controlInput(values, c.id).value = fmt(c, state.parameters[c.id]);
  }
  detectPreset(); updateLightPad();
}
function setParameter(c: Control, raw: string | number) {
  if (raw === '' || !Number.isFinite(Number(raw))) return false;
  const v = Math.max(c.min, Math.min(c.max, Number(raw)));
  state.parameters[c.id] = Number((Math.round(v / c.step) * c.step).toFixed(4));
  controlInput(ranges, c.id).value = String(state.parameters[c.id]);
  controlInput(ranges, c.id).setAttribute('aria-valuenow',String(state.parameters[c.id]));
  if (document.activeElement !== controlInput(values, c.id)) controlInput(values, c.id).value = fmt(c, state.parameters[c.id]);
  detectPreset(); updateLightPad(); schedule(); persist();
  return true;
}
function buildControls() {
  for (const [group, title, description, open] of GROUPS) {
    const details = document.createElement('details');
    details.className = 'control-group'; details.open = open; details.dataset.group=group;
    const summary = document.createElement('summary');
    summary.innerHTML = `${title}<svg class="disclosure" viewBox="0 0 12 12" aria-hidden="true"><path d="m3 4 3 3 3-3" fill="none" stroke="currentColor" stroke-width="1.3"/></svg>`;
    details.append(summary);
    const intro = document.createElement('p'); intro.className = 'group-description'; intro.textContent = description; details.append(intro);
    let directionPad: HTMLDivElement | null = null;
    if (group === 'external') {
      const pad = document.createElement('div'); pad.className = 'light-direction';
      pad.innerHTML = '<div id="light-pad" class="light-pad" role="slider" tabindex="0" aria-label="拖动外部光源方向" aria-valuemin="0" aria-valuemax="359"><span id="light-knob" class="light-knob"></span></div><p>拖动或用方向键调整<br><span id="transmission-direction"></span></p>';
      directionPad=pad;
    }
    for (const c of engine.controls.filter(c => c.group === group && !c.legacy)) {
      const row = document.createElement('div'); row.className = 'control-row';
      const label = document.createElement('label'); label.htmlFor = `range-${c.id}`; label.textContent = c.name;
      const valWrap = document.createElement('div'); valWrap.className = 'control-value';
      const val = document.createElement('input'); val.id = `value-${c.id}`; val.type = 'number'; val.min = String(c.min); val.max = String(c.max); val.step = String(c.step); val.value = fmt(c,state.parameters[c.id]); val.inputMode=c.step>=1?'numeric':'decimal'; val.enterKeyHint='done'; val.setAttribute('aria-label', `${c.name}数值`);
      const unit = document.createElement('span'); unit.textContent = c.unit || '';
      valWrap.append(val,unit);
      const range = document.createElement('input'); range.type = 'range'; range.id = `range-${c.id}`; range.min = String(c.min); range.max = String(c.max); range.step = String(c.step); range.value = String(state.parameters[c.id]);
      range.addEventListener('input', () => setParameter(c,range.value));
      range.addEventListener('change', commit);
      val.addEventListener('input', () => setParameter(c,val.value));
      val.addEventListener('change', () => { if (val.value !== '') setParameter(c,val.value); val.value = fmt(c,state.parameters[c.id]); commit(); });
      val.addEventListener('blur', () => { val.value = fmt(c,state.parameters[c.id]); });
      val.addEventListener('keydown',e=>{ if(e.key==='Enter') { e.preventDefault(); val.blur(); commit(); } });
      row.append(label,valWrap,range); details.append(row);
      ranges.set(c.id,range); values.set(c.id,val);
      if(c.id==='angle' && directionPad) details.append(directionPad);
    }
    $('controls').append(details);
  }
}
function updateLightPad() {
  const pad=$('light-pad'); if (!pad) return;
  const a = state.parameters.angle * Math.PI / 180;
  $('light-knob').style.transform = `translate(${Math.cos(a)*30}px,${Math.sin(a)*30}px)`;
  pad.setAttribute('aria-valuenow', String(state.parameters.angle));
  pad.setAttribute('aria-valuetext', `${state.parameters.angle} 度`);
  const exit=engine.lightingState(state.parameters).transmitted.angle;
  const name=['右','右下','下','左下','左','左上','上','右上'][Math.round(exit/45)%8];
  $('transmission-direction').textContent=`绿光透出：${name} ${exit}°`;
}
function schedule() {
  if (pendingFrame) return;
  pendingFrame = requestAnimationFrame(() => { pendingFrame = 0; render(); });
}
function svgData(svg: string) { return 'data:image/svg+xml;charset=utf-8,' + encodeURIComponent(svg); }
function render() {
  const svg = engine.render(state.parameters,'current');
  $('current-render').innerHTML = svg;
  window.clearTimeout(miniTimer);
  if(mobileMode.matches && state.mobileGroup!=='tools') return;
  miniTimer = window.setTimeout(() => {
    const src=svgData(engine.render(state.parameters,'mini'));
    for (const size of [96,64,32]) {
      let img=$(`size-${size}`).querySelector('img');
      if (!img) { img=new Image(); img.alt=`当前图稿 ${size} 像素预览`; $(`size-${size}`).append(img); }
      img.src=src;
    }
  }, 100);
}
function setView(view: View) {
  if (view === 'snapshot' && !state.snapshot) return;
  state.view=view;
  if(mobileMode.matches) state.mobileView=view;
  else desktopView=view;
  query('.preview-pair').dataset.view=view;
  all('[data-view]', HTMLElement).forEach(button => { if (button.tagName === 'BUTTON') button.setAttribute('aria-pressed', String(button.dataset.view === view)); });
  const snapshot=view==='snapshot';
  $('reference-image').hidden=snapshot;
  $('snapshot-render').hidden=!snapshot;
  $('reference-title').textContent=snapshot?'暂存 · 调整前':'07 · 视觉参照';
  $('reference-caption').textContent=snapshot?'保持暂存时的参数':'原始静态图稿';
  if (snapshot) $('snapshot-render').innerHTML=engine.render(state.snapshot,'snapshot');
  persist();
}
function setShape(shape: Shape) {
  state.shape=shape;
  query('.preview-pair').dataset.shape=shape;
  all('.mini-icon', HTMLElement).forEach(el => { el.style.borderRadius=shape==='square'?'0':'22%'; });
  persist();
}
function setGround(ground: Ground) { state.ground=ground; query('.preview-ground').dataset.ground=ground; persist(); }
function replaceParameters(parameters: unknown,message?: string) {
  state.parameters=engine.normalize(parameters); syncControls(); schedule(); commit(); if (message) notice(message);
}
function saveBlob(blob: Blob, filename: string) {
  const url=URL.createObjectURL(blob), link=document.createElement('a');
  link.href=url; link.download=filename; document.body.append(link); link.click(); link.remove();
  window.setTimeout(()=>URL.revokeObjectURL(url), 10000);
}
const filename=(ext: string) => `khor-icon-${new Date().toISOString().replace(/[:.]/g,'-')}.${ext}`;
async function exportPNG() {
  const exportButton=button('export-png'), size=Number(select('export-size').value);
  const captured={...state.parameters};
  exportButton.disabled=true; exportButton.textContent='正在导出';
  const url=URL.createObjectURL(new Blob([engine.render(captured,'export')],{type:'image/svg+xml;charset=utf-8'}));
  try {
    const img=new Image();
    await new Promise<void>((resolve,reject)=>{ img.onload=()=>resolve(); img.onerror=()=>reject(new Error('SVG 图稿未能加载')); img.src=url; });
    const canvas=document.createElement('canvas'); canvas.width=size; canvas.height=size;
    const ctx=canvas.getContext('2d'); if (!ctx) throw new Error('浏览器未提供绘图画布');
    ctx.drawImage(img,0,0,size,size);
    const blob=await new Promise<Blob | null>(resolve=>canvas.toBlob(resolve,'image/png'));
    if (!blob) throw new Error('PNG 编码失败');
    saveBlob(blob,filename('png')); notice(`已导出 ${size} × ${size} PNG`);
  } catch (error) { notice(`导出失败：${errorMessage(error)}。可先导出 SVG。`); }
  finally { URL.revokeObjectURL(url); exportButton.disabled=false; exportButton.textContent='导出 PNG'; }
}
buildControls();
// Keep one set of live controls/actions; move the existing tools into the
// phone inspector rather than duplicating event handlers or export state.
const extraTools=$('extra-tools'), snapshotButton=$('snapshot'), viewOptions=query('.view-options'), exportArea=query('.export-area');
const toolsHome=document.createComment('desktop tools'), snapshotHome=document.createComment('desktop snapshot'), optionsHome=document.createComment('desktop view options'), exportHome=document.createComment('desktop export');
extraTools.before(toolsHome); snapshotButton.before(snapshotHome); viewOptions.before(optionsHome); exportArea.before(exportHome);
const desktopOpen=new Map([...all('.control-group', HTMLDetailsElement)].map(el=>[el.dataset.group,el.open]));
const groupNames: readonly (readonly [MobileGroup, string])[]=[['external','外光'],['internal','内光'],['glass','玻璃'],['background','背景'],['geometry','形体'],['tools','保存']];
for(const [key,label] of groupNames) {
  const button=document.createElement('button'); button.textContent=label; button.type='button'; button.dataset.group=key;
  button.addEventListener('click',()=>{ state.mobileGroup=key; applyMobileGroup(); $('inspector-body').scrollTop=0; persist(); if(key==='tools') schedule(); });
  $('group-tabs').append(button);
}
function applyMobileGroup() {
  all('#group-tabs button', HTMLButtonElement).forEach(button=>button.setAttribute('aria-pressed',String(button.dataset.group===state.mobileGroup)));
  all('.control-group', HTMLDetailsElement).forEach(details=>{
    details.hidden=mobileMode.matches && details.dataset.group!==state.mobileGroup;
    if(mobileMode.matches) details.open=true;
  });
  extraTools.hidden=mobileMode.matches && state.mobileGroup!=='tools';
  query('.inspector-foot').hidden=mobileMode.matches;
}
function applyLayout() {
  if(mobileMode.matches) {
    $('inspector-body').append(extraTools);
    $('mobile-snapshot-slot').append(snapshotButton);
    extraTools.prepend(viewOptions);
    extraTools.prepend(exportArea);
    setView(state.mobileView);
  } else {
    toolsHome.after(extraTools); snapshotHome.after(snapshotButton); optionsHome.after(viewOptions);
    exportHome.after(exportArea);
    all('.control-group', HTMLDetailsElement).forEach(details=>{ details.open=desktopOpen.get(details.dataset.group) ?? false; });
    setView(desktopView);
  }
  applyMobileGroup(); schedule();
}
mobileMode.addEventListener('change',applyLayout);
function updateViewport() {
  const viewport=window.visualViewport;
  if(viewport && viewport.scale!==1) return;
  const height=viewport?viewport.height:window.innerHeight;
  document.documentElement.style.setProperty('--viewport-height',`${height}px`);
  document.body.classList.toggle('keyboard-open',mobileMode.matches && window.innerHeight-height>130);
}
window.visualViewport?.addEventListener('resize',updateViewport);
window.addEventListener('resize',updateViewport);
$('undo').addEventListener('click',()=>undo(-1));
$('redo').addEventListener('click',()=>undo(1));
$('reset').addEventListener('click',()=>replaceParameters(current,'已恢复当前图标参数，仍可撤销'));
$('mobile-reset').addEventListener('click',()=>replaceParameters(current,'已恢复当前图标参数，仍可撤销'));
const presetSelect=select('preset');
presetSelect.addEventListener('change',()=>{ if (isOneOf(presetSelect.value, presetKeys)) replaceParameters(PRESETS[presetSelect.value]); });
all('button[data-view]', HTMLButtonElement).forEach(button=>button.addEventListener('click',()=>{ if (isOneOf(button.dataset.view, VIEWS)) setView(button.dataset.view); }));
const shapeSelect=select('shape'); shapeSelect.value=state.shape;
shapeSelect.addEventListener('change',()=>{ if (isOneOf(shapeSelect.value,['square','rounded'])) setShape(shapeSelect.value); });
const groundSelect=select('ground'); groundSelect.value=state.ground;
groundSelect.addEventListener('change',()=>{ if (isOneOf(groundSelect.value,GROUNDS)) setGround(groundSelect.value); });
$('snapshot').addEventListener('click',()=>{
  state.snapshot={...state.parameters}; button('snapshot-view').disabled=false; $('snapshot-note').textContent='已暂存，可继续调节'; setView('snapshot'); notice('已暂存此刻的参数');
});
button('snapshot-view').disabled=!state.snapshot;
function requiredControl(key: ControlKey): Control {
  const control=engine.controls.find(c=>c.id===key);
  if (!control) throw new Error(`缺少参数定义：${key}`);
  return control;
}
const angleControl=requiredControl('angle'), pad=$('light-pad');
function moveLight(e: PointerEvent) { const rect=pad.getBoundingClientRect(), angle=(Math.atan2(e.clientY-rect.top-rect.height/2,e.clientX-rect.left-rect.width/2)*180/Math.PI+360)%360; setParameter(angleControl,Math.round(angle)%360); }
pad.addEventListener('pointerdown',e=>{ if (e.button!==0) return; pad.setPointerCapture(e.pointerId); pad.focus(); moveLight(e); });
pad.addEventListener('pointermove',e=>{ if (pad.hasPointerCapture(e.pointerId)) moveLight(e); });
pad.addEventListener('pointerup',e=>{ if (pad.hasPointerCapture(e.pointerId)) { pad.releasePointerCapture(e.pointerId); commit(); } });
pad.addEventListener('pointercancel',commit);
const arrowSteps: Readonly<Record<string, number | undefined>>={ArrowLeft:-1,ArrowDown:1,ArrowRight:1,ArrowUp:-1};
pad.addEventListener('keydown',e=>{ const delta=arrowSteps[e.key]; if (delta) { e.preventDefault(); setParameter(angleControl,(state.parameters.angle+delta*(e.shiftKey?10:1)+360)%360); commit(); } });
document.addEventListener('keydown',e=>{
  if (!(e.metaKey||e.ctrlKey)||e.key.toLowerCase()!=='z'||(e.target instanceof HTMLElement && /INPUT|SELECT|TEXTAREA/.test(e.target.tagName))) return;
  e.preventDefault(); undo(e.shiftKey?1:-1);
});
$('export-json').addEventListener('click',()=>{ saveBlob(new Blob([JSON.stringify(engine.packagePreset(state.parameters),null,2)+'\n'],{type:'application/json'}),filename('json')); notice('已导出参数文件'); });
$('export-svg').addEventListener('click',()=>{ saveBlob(new Blob([engine.render(state.parameters,'export')],{type:'image/svg+xml;charset=utf-8'}),filename('svg')); notice('已导出完整方形 SVG'); });
$('export-png').addEventListener('click',exportPNG);
const importFile=input('import-file');
$('import-json').addEventListener('click',()=>importFile.click());
importFile.addEventListener('change',async ()=>{
  const file=importFile.files?.[0]; if (!file) return;
  try {
    if (file.size>100000) throw new Error('参数文件过大，请选择此调校台导出的 JSON');
    const parameters=engine.parsePreset(JSON.parse(await file.text()));
    replaceParameters(parameters,'已导入参数，原来的调整可通过撤销恢复');
  } catch(error) { notice(`无法导入：${errorMessage(error)}`); }
  finally { importFile.value=''; }
});
window.addEventListener('pagehide',()=>{
  try { localStorage.setItem(STORAGE,JSON.stringify(savedState())); } catch(_) { /* Export remains available. */ }
});
syncControls(); setShape(state.shape); setGround(state.ground); updateViewport(); applyLayout(); render();
$('save-state').textContent=restored?'已恢复上次调整':storageAvailable?'自动保存在此浏览器':'本地保存不可用 · 可导出参数';
