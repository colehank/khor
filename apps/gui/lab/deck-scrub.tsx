/** Real DeckBand; deterministic data, no node, native page or PTY. */
import {useRef,useState} from 'react';
import {createRoot} from 'react-dom/client';
import {DeckBand} from '@/components/DeckBand';
import {TabAvatar,type TabFace} from '@/browse/TabStrip';
import {TooltipProvider} from '@/components/ui/tooltip';
import {Button} from '@/components/ui/button';
import {GlassCircle} from '@/components/Glass';
import {IconBack,IconReload} from '@/components/icons';
import {TabsGlyph} from '@/components/SessionBar';
import type {Avatar} from '@/gen/bindings/Avatar';
import fixture from './tab-continuity-fixture.json';
import {speakLang} from '@/lang';
import '@/styles/tokens.css';import '@/styles/app.css';import '@/styles/session-content.css';
import './deck-scrub.css';
speakLang('zh');
const faces:TabFace[]=['khor-codex','ACL2027','KapelIntro','khor-cc','openneuro','ASR','hinton · shell','turing · shell'].map((label,i)=>({id:String(i),label,path:'~/working/'+label,face:fixture.devices[i%fixture.devices.length].face as Avatar,session:{kind:'agent',category:i===0?'codex':'claude',word:'idle'}}));
const names=['khor','README.md','TerminalPane.tsx','终端配色文档','SHIP.md','package.json'];
const wait=(ms:number)=>new Promise(r=>setTimeout(r,ms));
function Lab(){
 const [active,setActive]=useState('0'),[nested,setNested]=useState(false),[dark,setDark]=useState(true),[order,setOrder]=useState(faces),[events,setEvents]=useState<string[]>([]),[report,setReport]=useState('');
 const real=useRef({active,events});real.current={active,events};
 const items=nested?order.map((f,i)=>({...f,label:names[i%names.length]})):order;
 const focus=(id:string)=>{setActive(id);setEvents(old=>[...old,id]);};
 const mode=(value:boolean)=>{setNested(value);setActive('0');setEvents([]);};
 const inspect=async()=>{
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
   pointer('pointermove',rect.width*.3);await wait(700);pointer('pointermove',0);await wait(20);
   const paused=b.dataset.bandPreview;await wait(700);
   check(b.dataset.bandPreview===paused,'回中停止连续翻选');
   pointer('pointerup');await wait(220);
   check(real.current.events.length===before+1,'松手只打开最终目标一次');
   pointer('pointerdown');await wait(420);pointer('pointermove',0,40);await wait(30);
   check(!document.querySelector('[data-deck-selector]'),'长按下拉取消');pointer('pointerup');await wait(180);
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
 return <TooltipProvider><main className="deck-proof"><aside><h1>正式药丸手势</h1><p>轻推慢翻，推远加速，回中暂停；松手打开。原地长按展开选择带。</p><div className="deck-proof-actions"><Button onClick={()=>mode(false)}>会话</Button><Button onClick={()=>mode(true)}>文件与网页</Button><Button onClick={()=>{document.documentElement.dataset.theme=dark?'light':'dark';setDark(!dark);}}>明暗</Button><Button onClick={()=>{setOrder(v=>v.length===1?faces:[faces[0]]);setActive('0');}}>单个／多个</Button><Button onClick={()=>void inspect()}>运行手势验收</Button></div><p>正在显示：<b data-proof-open>{items.find(f=>f.id===active)?.label}</b></p><p>实际切换：<b data-proof-count>{events.length}</b></p><p>合成事件只替代指针捕获 API；真实 iPhone 触控、原生网页和系统触感另验</p><output data-deck-proof-report>{report}</output></aside>
 <section className="deck-proof-phone"><div className="deck-proof-content"><h2>{items.find(f=>f.id===active)?.label}</h2><p>› 预选期间保持当前内容</p><pre>{'terminal colours       passed\ncontent navigation     passed\n\n✓ ready'}</pre><input placeholder="编辑输入保留自己的手势"/></div>
 <footer className="mobile-content-deck"><GlassCircle aria-label="返回"><IconBack/></GlassCircle><div className="glass mobile-content-pill"><DeckBand faces={items} active={active} onFocus={focus} onTitle={()=>mode(!nested)} renderFace={nested?(f)=><span className="content-owner-identity"><span className="content-owner-face"><TabAvatar f={f} bare="size-tab-face" seat="tab"/></span><span className="content-owner-name">khor-codex</span><span className="content-owner-title">{f.label}</span></span>:undefined}/><IconReload/></div><GlassCircle aria-label="标签"><TabsGlyph n={items.length}/></GlassCircle></footer></section></main></TooltipProvider>;
}
const root=createRoot(document.getElementById('root')!);root.render(<Lab/>);if(import.meta.hot)import.meta.hot.dispose(()=>root.unmount());
