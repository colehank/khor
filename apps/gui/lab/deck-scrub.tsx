/** Real DeckBand; deterministic data, no node, native page or PTY. */
import {useEffect,useRef,useState} from 'react';
import {createRoot} from 'react-dom/client';
import {DeckBand} from '@/components/DeckBand';
import {type TabFace} from '@/browse/TabStrip';
import {TooltipProvider} from '@/components/ui/tooltip';
import {Button} from '@/components/ui/button';
import {GlassCircle} from '@/components/Glass';
import {IconBack,IconReload,IconKeyboard} from '@/components/icons';
import {TabsGlyph} from '@/components/SessionBar';
import {ContentOwnerIdentity} from '@/views/MobileSessionContent';
import {fileKind,pageKind} from '@/components/SessionFace';
import {checkExplore} from './deck-explore-check';
import {DeckTuningDialog,TuningFields} from './DeckTuning';
import {DEFAULT_TUNING,getTuning,setTuning,type DeckTuning} from './deck-tuning';
import type {Avatar} from '@/gen/bindings/Avatar';
import fixture from './tab-continuity-fixture.json';
import {speakLang} from '@/lang';
import '@/styles/tokens.css';import '@/styles/app.css';import '@/styles/session-content.css';
import './deck-scrub.css';
speakLang('zh');
const faces:TabFace[]=['khor-codex','ACL2027','KapelIntro','khor-cc','openneuro','ASR','研究笔记与跨机器实验结果整理：从数据处理到可视化分析的完整工作记录','turing · shell'].map((label,i)=>({id:String(i),label,path:'~/working/'+label,face:fixture.devices[i%fixture.devices.length].face as Avatar,session:{kind:'agent',category:i===0?'codex':'claude',word:'idle'}}));
const names=['khor','README.md','TerminalPane.tsx','终端配色文档','SHIP.md','package.json','本周跨机器实验结果、问题复盘与后续工作计划.md'];
const wait=(ms:number)=>new Promise(r=>setTimeout(r,ms));
function Lab(){
 const [active,setActive]=useState('0'),[nested,setNested]=useState(false),[dark,setDark]=useState(true),[order,setOrder]=useState(faces),[events,setEvents]=useState<string[]>([]),[report,setReport]=useState('');
 const [tuning,setTuningValue]=useState(getTuning),[tuningOpen,setTuningOpen]=useState(false);
 const checking=useRef(false);const [isChecking,setIsChecking]=useState(false);
 const tune=(patch:Partial<DeckTuning>)=>{if(!checking.current)setTuningValue(setTuning(patch));};
 const runCheck=async(check:()=>Promise<unknown>)=>{if(checking.current)return;checking.current=true;setIsChecking(true);const saved=getTuning();setTuning(DEFAULT_TUNING,false);try{await check();}finally{setTuning(saved,false);checking.current=false;setIsChecking(false);}};
 const openTuning=()=>{if(checking.current)return;staged.current?.();setTuningOpen(true);};
 const staged=useRef<(()=>void)|null>(null);
 useEffect(()=>()=>staged.current?.(),[]);
 const real=useRef({active,events});real.current={active,events};
 const items=nested?order.map((f,i)=>({...f,label:names[i%names.length],session:undefined,mark:i===3?pageKind():fileKind(names[i%names.length])})):order;
 const focus=(id:string)=>{setActive(id);setEvents(old=>[...old,id]);};
 const mode=(value:boolean)=>{staged.current?.();setNested(value);setActive('0');setEvents([]);};
 const responseCheck=async()=>{
  staged.current?.();setReport('运行中');
  const b=document.querySelector<HTMLElement>('[data-deck-band]')!;
  const original={set:b.setPointerCapture,has:b.hasPointerCapture,release:b.releasePointerCapture};let captured=false;
  b.setPointerCapture=()=>{captured=true;};b.hasPointerCapture=()=>captured;b.releasePointerCapture=()=>{captured=false;};
  const r=b.getBoundingClientRect(),x=r.left+r.width/2,y=r.top+r.height/2;
  const pointer=(type:string,dx=0)=>b.dispatchEvent(new PointerEvent(type,{bubbles:true,cancelable:true,pointerId:84,isPrimary:true,pointerType:'touch',button:0,clientX:x+dx,clientY:y}));
  const results:{label:string;pass:boolean;ms?:number}[]=[];
  const before=real.current.events.length;
  try{
   const began=performance.now();pointer('pointerdown');
   while(b.dataset.bandPhase!=='rail'&&performance.now()-began<1000)await wait(16);
   const entry=performance.now()-began;
   results.push({label:'长按唤出选择带',pass:b.dataset.bandPhase==='rail'&&entry<400,ms:Math.round(entry)});
   const shifted=-r.width*.68;
   pointer('pointermove',shifted);await wait(380);const parked=b.dataset.bandPreview;
   results.push({label:'滑动后先停靠候选',pass:b.dataset.bandDirection==='0'});
   await wait(1100);
   results.push({label:'完整停靠后留出阅读等待',pass:b.dataset.bandPreview===parked&&b.dataset.bandDirection==='0'});
   await wait(500);
   results.push({label:'偏移处静止后自动切换',pass:b.dataset.bandDirection==='1'&&b.dataset.bandPreview!==parked});
   const candidate=b.dataset.bandPreview;
   const previous=items[(items.findIndex(f=>f.id===candidate)+items.length-1)%items.length]?.id;
   pointer('pointermove',shifted+14);await wait(380);
   results.push({label:'仍在初始触点左侧，反向即可回上一项',pass:b.dataset.bandPreview===previous&&b.dataset.bandDirection==='0'});
   await wait(800);
   results.push({label:'反向后重新等待，不立刻连续翻选',pass:b.dataset.bandPreview===previous&&b.dataset.bandDirection==='0'});
   await wait(500);
   results.push({label:'再次静止后，自动沿最新滑动方向继续',pass:b.dataset.bandDirection==='-1'&&b.dataset.bandPreview!==previous});
   const slower=b.dataset.bandPreview;await wait(600);
   results.push({label:'反向后使用慢速阅读停顿，不继承原快速度',pass:b.dataset.bandPreview===slower});
   const center=b.dataset.bandPreview;pointer('pointermove',0);await wait(350);await wait(1500);
   results.push({label:'回到初始触点附近持续暂停，保留当前候选',pass:b.dataset.bandPreview===center&&b.dataset.bandDirection==='0'});
   results.push({label:'预选期间不打开中间内容',pass:real.current.events.length===before});
   pointer('pointercancel');await wait(30);
   results.push({label:'取消恢复原药丸',pass:b.dataset.bandPhase==='idle'&&real.current.events.length===before});
   setReport(JSON.stringify({pass:results.every(v=>v.pass),results},null,2));
  }catch(e){setReport(JSON.stringify({pass:false,error:String(e),results},null,2));}
  finally{pointer('pointercancel');b.setPointerCapture=original.set;b.hasPointerCapture=original.has;b.releasePointerCapture=original.release;}
 };
 const releaseCheck=async()=>{
  staged.current?.();setReport('运行中');
  const b=document.querySelector<HTMLElement>('[data-deck-band]')!;
  const original={set:b.setPointerCapture,has:b.hasPointerCapture,release:b.releasePointerCapture};let captured=false;
  b.setPointerCapture=()=>{captured=true;};b.hasPointerCapture=()=>captured;b.releasePointerCapture=()=>{captured=false;};
  const r=b.getBoundingClientRect(),x=r.left+r.width/2,y=r.top+r.height/2;
  const p=(type:string,dx=0)=>b.dispatchEvent(new PointerEvent(type,{bubbles:true,cancelable:true,pointerId:87,isPrimary:true,pointerType:'touch',button:0,clientX:x+dx,clientY:y}));
  const results:{label:string;pass:boolean}[]=[];
  try{
   const before=real.current.events.length;
   p('pointerdown');p('pointermove',-r.width*.45);await wait(100);
   const target=b.dataset.bandPreview;p('pointerup');await wait(16);
   const snapshots=[b.dataset.bandPreview];
   for(let i=0;i<12;i++){p('pointermove',r.width*.7);p('pointerup');await wait(16);snapshots.push(b.dataset.bandPreview);}
   results.push({label:'松手立即锁定候选，停靠期间不跳回',pass:snapshots.every(id=>id===target)});
   results.push({label:'迟到移动和重复松手不改变目标，只打开一次',pass:real.current.active===target&&real.current.events.length===before+1});
   setReport(JSON.stringify({pass:results.every(v=>v.pass),results},null,2));
  }finally{p('pointercancel');b.setPointerCapture=original.set;b.hasPointerCapture=original.has;b.releasePointerCapture=original.release;}
 };
 const boundaryCheck=async()=>{
  staged.current?.();setReport('运行中');
  const b=document.querySelector<HTMLElement>('[data-deck-band]')!;
  const original={set:b.setPointerCapture,has:b.hasPointerCapture,release:b.releasePointerCapture};let captured=false;
  b.setPointerCapture=()=>{captured=true;};b.hasPointerCapture=()=>captured;b.releasePointerCapture=()=>{captured=false;};
  const r=b.getBoundingClientRect(),x=r.left+r.width/2,y=r.top+r.height/2,shift=-r.width*.68;
  const p=(type:string,dx=0)=>b.dispatchEvent(new PointerEvent(type,{bubbles:true,cancelable:true,pointerId:89,isPrimary:true,pointerType:'touch',button:0,clientX:x+dx,clientY:y}));
  const results:{label:string;pass:boolean}[]=[],before=real.current.events.length;
  try{
   p('pointerdown');await wait(430);p('pointermove',shift);await wait(1640);const candidate=b.dataset.bandPreview;
   p('pointermove',shift+14);await wait(35);p('pointermove',shift);await wait(380);
   results.push({label:'纠正途中再次反向，跟随最新意图且不多跳一项',pass:b.dataset.bandPreview===candidate&&b.dataset.bandDirection==='0'});
   await wait(800);
   results.push({label:'再次纠正也重新留出完整阅读时间',pass:b.dataset.bandPreview===candidate&&b.dataset.bandDirection==='0'});
   await wait(500);let uninterrupted=b.dataset.bandDirection==='1';
   for(let i=0;i<20;i++){p('pointermove',shift+(i%2?3:-3));await wait(60);uninterrupted=uninterrupted&&b.dataset.bandDirection==='1';}
   results.push({label:'轻微抖动不反复打断自动',pass:uninterrupted});
   p('pointermove',8);await wait(350);const parked=b.dataset.bandPreview;await wait(1200);
   results.push({label:'起点附近保留容易触达的持续暂停区',pass:b.dataset.bandPreview===parked&&b.dataset.bandDirection==='0'});
   p('pointercancel');await wait(40);
   results.push({label:'全部操作仅预选，取消不打开中间内容',pass:real.current.events.length===before&&b.dataset.bandPhase==='idle'});
   setReport(JSON.stringify({pass:results.every(v=>v.pass),results},null,2));
  }finally{p('pointercancel');b.setPointerCapture=original.set;b.hasPointerCapture=original.has;b.releasePointerCapture=original.release;}
 };
 const geometryCheck=async()=>{
  staged.current?.();
  setReport('运行中');const results:{label:string;pass:boolean;detail?:unknown}[]=[];
  const b=document.querySelector<HTMLElement>('[data-deck-band]')!;
  const original={set:b.setPointerCapture,has:b.hasPointerCapture,release:b.releasePointerCapture};let captured=false;
  b.setPointerCapture=()=>{captured=true;};b.hasPointerCapture=()=>captured;b.releasePointerCapture=()=>{captured=false;};
  const r=b.getBoundingClientRect(),x=r.left+r.width/2,y=r.top+r.height/2;
  const p=(type:string,dx=0,dy=0)=>b.dispatchEvent(new PointerEvent(type,{bubbles:true,cancelable:true,pointerId:82,isPrimary:true,pointerType:'touch',button:0,clientX:x+dx,clientY:y+dy}));
  const check=(pass:boolean,label:string,detail?:unknown)=>results.push({pass,label,detail});
  const rest=()=>[...b.querySelectorAll<HTMLElement>('[data-band-seg]')].filter(el=>getComputedStyle(el).display!=='none'&&el.getBoundingClientRect().width>.5);
  try{
   check(rest().length===1,'静止只绘制当前标签',rest().map(el=>({role:el.dataset.bandSeg,width:el.getBoundingClientRect().width})));
   p('pointerdown');p('pointermove',r.width*.2);await wait(25);
   const prev=b.querySelector<HTMLElement>('[data-band-seg="prev"]')!,face=prev.querySelector('[data-deck-face],.content-owner-face')!;
   check(Math.abs(face.getBoundingClientRect().left-r.left)<1,'右滑新头像固定在左沿');
   const inner=prev.querySelector<HTMLElement>('[data-band-body]');const w1=inner?.getBoundingClientRect().width;
   p('pointermove',r.width*.3);await wait(25);
   check(!!inner&&Math.abs(inner.getBoundingClientRect().width-w1!)<1,'右滑只扩大裁切，不压缩内部排版');
   p('pointercancel');await wait(30);
   p('pointerdown');await wait(620);
   const selector=document.querySelector('[data-deck-selector]')?.getBoundingClientRect(),host=b.closest('[data-deck-explore-host]')!.getBoundingClientRect();
   check(!!selector&&Math.abs(selector.left-host.left)<1&&Math.abs(selector.right-host.right)<1&&Math.abs(selector.bottom-r.top)<1,'探索层覆盖内容并止于药丸上沿',selector&&{selector:[selector.left,selector.right,selector.bottom],host:[host.left,host.right],deckTop:r.top});
   p('pointermove',r.width*.3,-150);await wait(30);
   check(!!document.querySelector('[data-deck-selector]'),'手指移到药丸上方仍在选择');
   p('pointermove',r.width*.3,50);await wait(30);
   check(!!document.querySelector('[data-deck-selector]'),'手指移到药丸下方仍在选择');
   p('pointercancel');await wait(30);check(rest().length===1,'取消后相邻头像不露出');
   p('pointerdown');p('pointermove',-r.width*.68);await wait(700);
   p('pointermove',r.width*.68);await wait(35);
   const outgoing=b.querySelector('[data-band-seg="cur"]')?.textContent;
   p('pointerup');await wait(20);
   check(b.querySelector('[data-band-seg="cur"]')?.textContent===outgoing,'连续反向后松手不翻转动画方向');
   await wait(200);check(rest().length===1,'松手后只留最终标签');
  }finally{b.setPointerCapture=original.set;b.hasPointerCapture=original.has;b.releasePointerCapture=original.release;}
  setReport(JSON.stringify({pass:results.every(v=>v.pass),results},null,2));
 };
 const preview=async(rail:boolean)=>{
  staged.current?.();
  const b=document.querySelector<HTMLElement>('[data-deck-band]')!;
  const original={set:b.setPointerCapture,has:b.hasPointerCapture,release:b.releasePointerCapture};
  staged.current=()=>{b.dispatchEvent(new PointerEvent('pointercancel',{bubbles:true,pointerId:83,isPrimary:true}));b.setPointerCapture=original.set;b.hasPointerCapture=original.has;b.releasePointerCapture=original.release;staged.current=null;};
  // A staged synthetic contact for visual inspection, using production handlers.
  // The same capture stub as the assertions; ended by the explicit preview control.
  b.setPointerCapture=()=>{};b.hasPointerCapture=()=>true;b.releasePointerCapture=()=>{};
  const r=b.getBoundingClientRect(),x=r.left+r.width/2,y=r.top+r.height/2;
  b.dispatchEvent(new PointerEvent('pointercancel',{bubbles:true,pointerId:83,isPrimary:true}));
  b.dispatchEvent(new PointerEvent('pointerdown',{bubbles:true,pointerId:83,isPrimary:true,pointerType:'touch',button:0,clientX:x,clientY:y}));
  if(rail)await wait(620);
  b.dispatchEvent(new PointerEvent('pointermove',{bubbles:true,cancelable:true,pointerId:83,isPrimary:true,pointerType:'touch',clientX:x+r.width*.2,clientY:y-(rail?140:0)}));
 };
 const inspect=async()=>{
  staged.current?.();
  setReport('运行中');
  const results:string[]=[];
  const check=(pass:boolean,label:string)=>{if(!pass)throw Error(label);results.push(label);};
  // Only capture is stubbed for untrusted test events; handlers and lifecycle
  // listeners are the same as real input. Native mouse is checked separately.
  const b=document.querySelector<HTMLElement>('[data-deck-band]')!;
  const original={set:b.setPointerCapture,has:b.hasPointerCapture,release:b.releasePointerCapture};
  let captured=false;
  b.setPointerCapture=()=>{captured=true;};b.hasPointerCapture=()=>captured;b.releasePointerCapture=()=>{captured=false;};
  const rect=b.getBoundingClientRect(),x=rect.left+rect.width/2,y=rect.top+rect.height/2;
  const pointer=(type:string,dx=0,dy=0)=>b.dispatchEvent(new PointerEvent(type,{bubbles:true,cancelable:true,pointerId:81,isPrimary:true,pointerType:'touch',button:0,clientX:x+dx,clientY:y+dy}));
  try{
   const before=real.current.events.length,start=real.current.active;
   pointer('pointerdown');pointer('pointermove',-rect.width*.3);await wait(450);
   check(real.current.events.length===before,'按住预选不打开中间内容');
   await wait(450);pointer('pointermove',-rect.width*.68);await wait(700);
   check(real.current.events.length===before,'调速仍不打开中间内容');
   pointer('pointercancel');await wait(180);
   check(b.dataset.bandPhase==='idle'&&real.current.active===start,'取消恢复原标签');
   pointer('pointerdown');await wait(420);
   check(!!document.querySelector('[data-deck-selector]'),'长按展开上方选择带');
   pointer('pointermove',-rect.width*.3);await wait(1640);pointer('pointermove',-rect.width*.3+14);await wait(250);
   check(b.dataset.bandDirection==='0','同侧反向先停止自动并重新等待');
   const target=b.dataset.bandPreview;pointer('pointerup');await wait(300);
   check(real.current.events.length===before+(target===start?0:1)&&real.current.active===target,'松手只打开最终目标一次');
   pointer('pointerdown');await wait(420);pointer('pointermove',0,40);await wait(30);
   check(!!document.querySelector('[data-deck-selector]'),'长按下拉仍保持选择');pointer('pointercancel');await wait(180);
   const nextCount=real.current.events.length;
   pointer('pointerdown');pointer('pointermove',-rect.width*.6);window.dispatchEvent(new Event('blur'));pointer('pointerup');await wait(220);
   check(real.current.events.length===nextCount&&b.dataset.bandPhase==='idle','失焦后不会迟到切换');
   pointer('pointerdown');pointer('pointermove',0,30);pointer('pointerup');await wait(180);
   check(real.current.events.length===nextCount,'纵向手势不切换标签');
   pointer('pointerdown');pointer('pointermove',-rect.width*.6);setOrder(v=>[...v].reverse());await wait(50);
   pointer('pointerup');await wait(180);
   check(b.dataset.bandPhase==='idle'&&real.current.events.length===nextCount,'标签顺序变化撤销旧预选');
   setOrder(v=>[...v].reverse());await wait(30);
   const touch=(type:string)=>{
     const e=new Event(type,{bubbles:true});
     const t={identifier:19,target:b,clientX:x-rect.width*.6,clientY:y};
     Object.defineProperties(e,{touches:{value:type==='touchstart'?[t]:[]},changedTouches:{value:[t]}});window.dispatchEvent(e);
   };
   pointer('pointerdown');touch('touchstart');pointer('pointermove',-rect.width*.6);await wait(100);touch('touchend');pointer('pointerup');await wait(200);
   check(real.current.events.length===nextCount+1,'原生 touchend 补足结束且不重复提交');
   setReport(JSON.stringify({pass:true,results},null,2));
  }catch(e){setReport(JSON.stringify({pass:false,error:String(e),results},null,2));}
  finally{b.setPointerCapture=original.set;b.hasPointerCapture=original.has;b.releasePointerCapture=original.release;}
 };
 return <TooltipProvider><main className="deck-proof"><aside><h1>药丸停留自动 Lab</h1><p>横滑快速切换；长按进入浅 U 探索：滑动选择，停靠后等 {tuning.readMs} ms 自动翻选；反向回上一项并重新等待，回到起点暂停，松手确认。</p><TuningFields value={tuning} onChange={tune} disabled={isChecking}/><p className="text-caption text-muted-foreground">下次触碰生效 · 正式默认值 1200 / 200 ms</p><div className="deck-proof-actions"><Button disabled={isChecking} onClick={openTuning}>更多参数</Button><Button onClick={()=>mode(false)}>会话</Button><Button onClick={()=>mode(true)}>文件与网页</Button><Button onClick={()=>{document.documentElement.dataset.theme=dark?'light':'dark';setDark(!dark);}}>明暗</Button><Button onClick={()=>{setOrder(v=>v.length===1?faces:[faces[0]]);setActive('0');}}>单个／多个</Button><Button onClick={()=>void preview(false)}>右滑预览</Button><Button onClick={()=>void preview(true)}>长按移出预览</Button><Button onClick={()=>staged.current?.()}>结束预览</Button><Button onClick={()=>void runCheck(responseCheck)}>运行响应验收</Button><Button onClick={()=>void runCheck(releaseCheck)}>运行松手验收</Button><Button onClick={()=>void runCheck(boundaryCheck)}>运行边界验收</Button><Button onClick={()=>void runCheck(geometryCheck)}>运行几何验收</Button><Button onClick={()=>void runCheck(inspect)}>运行手势验收</Button><Button onClick={async()=>{staged.current?.();setActive('6');setReport('运行中');await wait(80);await runCheck(async()=>setReport(JSON.stringify(await checkExplore(),null,2)));}}>运行探索验收</Button></div><p>正在显示：<b data-proof-open>{items.find(f=>f.id===active)?.label}</b></p><p>实际切换：<b data-proof-count>{events.length}</b></p><p>验收按钮使用正式默认参数，结束后恢复你的设置；真实 iPhone 触控与触感另验</p><output data-deck-proof-report>{report}</output></aside>
 <section className="deck-proof-phone" data-deck-explore-host><div className="deck-proof-content"><Button disabled={isChecking} onClick={openTuning}>调节参数</Button><h2>{items.find(f=>f.id===active)?.label}</h2><p>› 预选期间保持当前内容</p><pre>{'terminal colours       passed\ncontent navigation     passed\n\n✓ ready'}</pre><input placeholder="编辑输入保留自己的手势"/></div>
 <footer data-session-deck className="mobile-content-deck"><GlassCircle aria-label="返回"><IconBack/></GlassCircle><div className="glass mobile-content-pill"><div className={nested?"mobile-file-identity":"contents"}><DeckBand faces={items} active={active} onFocus={focus} onTitle={()=>mode(!nested)} renderFace={nested?(f)=><ContentOwnerIdentity face={faces[0]} title={f.label}/>:undefined}/></div><IconReload/></div>{!nested&&<GlassCircle aria-label="键盘"><IconKeyboard/></GlassCircle>}<GlassCircle aria-label="标签"><TabsGlyph n={items.length}/></GlassCircle></footer></section><DeckTuningDialog value={tuning} onChange={tune} open={tuningOpen} onOpenChange={setTuningOpen}/></main></TooltipProvider>;
}
const root=createRoot(document.getElementById('root')!);root.render(<Lab/>);if(import.meta.hot)import.meta.hot.dispose(()=>root.unmount());
