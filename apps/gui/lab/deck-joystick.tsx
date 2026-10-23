/** Isolated frontend Lab: real SessionScreen + xterm + shared tab rows. */
import {useEffect,useLayoutEffect,useRef,useState} from 'react';
import {createRoot} from 'react-dom/client';
import {Terminal} from '@xterm/xterm';
import {FitAddon} from '@xterm/addon-fit';
import {SessionScreen,BarSeat,TabSheetRows} from '@/components/SessionBar';
import {type TabFace} from '@/browse/TabStrip';
import {GlassCircle,GlassItem} from '@/components/Glass';
import {IconClose,IconKeyboard,IconSearch,IconReload} from '@/components/icons';
import {Button} from '@/components/ui/button';
import {Input} from '@/components/ui/input';
import {TooltipProvider} from '@/components/ui/tooltip';
import {fileKind} from '@/components/SessionFace';
import {MobileContentSheet} from '@/views/MobileSessionContent';
import {speakLang} from '@/lang';
import type {Avatar} from '@/gen/bindings/Avatar';
import {JoystickContext} from './deck-joystick/Band';
import {DEFAULTS,type Tuning} from './deck-joystick/engine';
import fixture from './tab-continuity-fixture.json';
import '@/styles/tokens.css';
import '@/styles/app.css';
import '@/styles/session-content.css';
import '@xterm/xterm/css/xterm.css';
import './deck-joystick/style.css';

speakLang('zh');
const names=['khor','了解当前仓库','Claude Code','App 工作区改版调研','理解方法和公式逻辑','优化智能体课程第二讲大纲','vibing','订阅与额度同步','跨机器实验结果整理与后续工作计划','turing · shell','README.md','新会话'];
const seed:TabFace[]=names.map((label,i)=>({id:`session-${i}`,label,path:`~/working/${i<4?'khor':'research'}`,machine:i%3?'MacLab':'turing',face:fixture.devices[i%fixture.devices.length].face as Avatar,
  ...(i===10?{mark:fileKind('README.md')}:{session:{kind:i===9?'shell':'agent',category:i%2?'claude':'codex',word:['busy','idle','blocked','done'][i%4]}})}));
const tuningStore='khor.joystick-lab.v2';
const persisted=()=>{try{return {...DEFAULTS,...JSON.parse(localStorage.getItem(tuningStore)||'{}')};}catch{return DEFAULTS;}};

function TerminalSample({face,dark,refresh}:{face:TabFace;dark:boolean;refresh:number}){
  const seat=useRef<HTMLDivElement>(null),term=useRef<Terminal|null>(null),fit=useRef<FitAddon|null>(null);
  useEffect(()=>{
    const cs=getComputedStyle(seat.current!);
    const terminal=new Terminal({fontFamily:cs.getPropertyValue('--font-mono'),fontSize:14,lineHeight:1.35,cursorBlink:false,convertEol:true,theme:{background:cs.getPropertyValue('--card').trim(),foreground:cs.getPropertyValue('--foreground').trim()},scrollback:500});
    const addon=new FitAddon();terminal.loadAddon(addon);terminal.open(seat.current!);term.current=terminal;fit.current=addon;
    const observer=new ResizeObserver(()=>{if(seat.current?.clientWidth&&seat.current.clientHeight)addon.fit();});observer.observe(seat.current!);
    return()=>{observer.disconnect();terminal.dispose();term.current=null;fit.current=null;};
  },[]);
  useEffect(()=>{const terminal=term.current;if(!terminal)return;const cs=getComputedStyle(seat.current!);terminal.options.theme={background:cs.getPropertyValue('--card').trim(),foreground:cs.getPropertyValue('--foreground').trim()};terminal.refresh(0,terminal.rows-1);},[dark]);
  useEffect(()=>{
    const terminal=term.current;if(!terminal)return;terminal.reset();fit.current?.fit();
    terminal.writeln(`${face.machine||'MacLab'}  ~/working/khor`);
    terminal.writeln('');terminal.writeln(`❯ ${face.label}`);terminal.writeln('');
    terminal.writeln('  正在整理工作区的会话与内容。');terminal.writeln('  已连接，终端保持当前阅读位置。');terminal.writeln('');
    terminal.writeln('  changed   apps/gui/src/components');terminal.writeln('  changed   apps/gui/src/views');terminal.writeln('');
    terminal.writeln('  预选时，这里的内容保持不动。');terminal.writeln('  松手确认后，只打开最终会话。');terminal.writeln('');
    terminal.writeln('────────────────────────────────');terminal.writeln('');terminal.write('❯ ');
  },[face,refresh]);
  return <div className="joystick-terminal" ref={seat} data-open-session={face.id}/>;
}

function Lab(){
  const [faces,setFaces]=useState(seed),[active,setActive]=useState(seed[3].id),[scene,setScene]=useState<'terminal'|'overview'|'content'>('terminal');
  const [dark,setDark]=useState(true),[query,setQuery]=useState(''),[tuning,setTuning]=useState<Tuning>(persisted),[controls,setControls]=useState(false),[keyboard,setKeyboard]=useState(false),[count,setCount]=useState(0);
  const [refresh,setRefresh]=useState(0);
  const keyboardInput=useRef<HTMLInputElement>(null),pageRef=useRef<HTMLElement>(null);
  const face=faces.find(f=>f.id===active)||faces[0];
  useLayoutEffect(()=>{document.documentElement.dataset.theme=dark?'dark':'light';},[dark]);
  useEffect(()=>{
    if(scene==='terminal')return;
    const before=document.activeElement as HTMLElement|null;
    pageRef.current?.querySelector<HTMLButtonElement>('button')?.focus({preventScroll:true});
    return()=>{if(before?.isConnected)before.focus({preventScroll:true});};
  },[scene]);
  useEffect(()=>{localStorage.setItem(tuningStore,JSON.stringify(tuning));},[tuning]);
  const focus=(id:string)=>{if(id!==active)setCount(n=>n+1);setActive(id);setScene('terminal');};
  const all=()=>{setQuery('');setScene('overview');};
  const close=(id:string)=>{const at=faces.findIndex(f=>f.id===id),rest=faces.filter(f=>f.id!==id);setFaces(rest);if(active===id)setActive(rest[Math.min(at,rest.length-1)]?.id||'');};
  const add=()=>{const f={...seed[0],id:`new-${Date.now()}`,label:'新会话'};setFaces(v=>[...v,f]);setActive(f.id);setScene('terminal');};
  const showKeyboard=()=>{setKeyboard(v=>!v);requestAnimationFrame(()=>keyboardInput.current?.focus());};
  const fields:{key:keyof Tuning;label:string;min:number;max:number;step:number;unit:string}[]=[
    {key:'holdMs',label:'长按展开',min:180,max:450,step:10,unit:'ms'},
    {key:'neutral',label:'中心停区半径',min:6,max:20,step:1,unit:'px'},
    {key:'reach',label:'推到最快的距离',min:32,max:88,step:2,unit:'px'},
    {key:'slowMs',label:'慢档间隔',min:1200,max:2000,step:50,unit:'ms'},
    {key:'mediumMs',label:'中档间隔',min:850,max:1150,step:50,unit:'ms'},
    {key:'fastMs',label:'快档间隔',min:550,max:800,step:50,unit:'ms'},
  ];
  const shown=faces.filter(f=>(f.label+' '+f.machine).toLowerCase().includes(query.toLowerCase()));
  return <TooltipProvider><main className="joystick-lab" data-joystick-lab>
    <div className="joystick-mobile-lab-bar"><span>khor · 拨杆 Lab</span><Button size="sm" onClick={()=>setControls(v=>!v)}>{controls?'返回试用':'参数'}</Button></div>
    <aside className="joystick-controls" data-visible={controls||undefined}>
      <p className="joystick-eyebrow">khor · 交互 Lab</p><h1>轻拨，切换</h1>
      <p className="joystick-intro">轻点查看内容，上滑打开全部标签。长按后轻拨切换，推远加速，回中暂停。</p>
      <details className="joystick-tuning"><summary>调节手感</summary><div className="joystick-settings">
        {fields.map(f=><label key={f.key}><span>{f.label}<output>{tuning[f.key]} {f.unit}</output></span><input aria-label={f.label} type="range" min={f.min} max={f.max} step={f.step} value={tuning[f.key]} onChange={e=>setTuning(v=>({...v,[f.key]:Number(e.target.value)}))}/></label>)}
      </div>
      <div className="joystick-lab-actions"><Button onClick={()=>setDark(v=>!v)}>{dark?'浅色':'深色'}</Button><Button onClick={()=>setTuning(DEFAULTS)}>恢复参数</Button><Button onClick={()=>{setFaces(seed);setActive(seed[3].id);setCount(0);setScene('terminal');}}>重置会话</Button></div>
      <p className="joystick-lab-facts">当前：<b data-lab-active>{face?.label||'没有打开的会话'}</b><br/>实际切换：<b data-lab-commits>{count}</b> 次</p>
      <p className="joystick-lab-note">参数在下次触碰时生效。键盘：左右切换，上方向键进入总览，Esc 取消预选。示例数据，复用当前前端组件。</p>
      </details>
    </aside>
    <section className="joystick-phone" data-deck-explore-host>
      <div className="joystick-screen-seat"><JoystickContext.Provider value={{onOverview:all,tuning}}>
        <SessionScreen faces={faces} active={active} onFocus={focus} onClose={close} onNew={add} onBack={all} onTabs={all} onTitle={()=>setScene('content')} folded={false} away={scene!=='terminal'} contentMode={scene!=='terminal'}>
          <div className="joystick-terminal-seat" inert={scene!=='terminal'}>{face?<TerminalSample face={face} dark={dark} refresh={refresh}/>:<div className="joystick-empty">没有打开的会话<br/><Button onClick={add}>新建会话</Button></div>}</div>
          <BarSeat at="lead"><GlassItem aria-label="刷新" onClick={()=>setRefresh(n=>n+1)}><IconReload/></GlassItem></BarSeat>
          <BarSeat at="trail"><GlassCircle aria-label="键盘" onClick={showKeyboard}><IconKeyboard/></GlassCircle></BarSeat>
          <BarSeat at="under"><div className="joystick-key-row">{['esc','ctrl','alt','tab','↑','↓'].map(key=><button key={key} type="button" onClick={()=>keyboardInput.current?.focus()}>{key}</button>)}</div>{keyboard&&<Input ref={keyboardInput} className="joystick-shell-input" placeholder="终端输入" aria-label="终端输入"/>}</BarSeat>
      {scene!=='terminal'&&<MobileContentSheet onClose={()=>setScene('terminal')}><section ref={pageRef} className="joystick-page" role="dialog" aria-modal="true" aria-label={scene==='overview'?'全部标签':'会话内容'} onKeyDown={e=>{
        if(e.key==='Escape'){e.preventDefault();e.stopPropagation();setScene('terminal');}
        if(e.key==='Tab'){
          const stops=Array.from(e.currentTarget.querySelectorAll<HTMLElement>('button,input')).filter(el=>!el.hasAttribute('disabled'));
          const index=stops.indexOf(document.activeElement as HTMLElement);
          if(index<0||e.shiftKey&&index===0||!e.shiftKey&&index===stops.length-1){e.preventDefault();stops[e.shiftKey?stops.length-1:0]?.focus();}
        }
      }}>
        <header><h2>{scene==='overview'?'全部标签':face?.label}</h2><GlassCircle aria-label="返回会话" onClick={()=>setScene('terminal')}><IconClose/></GlassCircle></header>
        {scene==='overview'?<><div className="joystick-search"><IconSearch/><Input type="search" aria-label="查找标签" placeholder="查找标签" value={query} onChange={e=>setQuery(e.target.value)}/></div><div className="joystick-tab-list"><TabSheetRows faces={shown} active={active} onFocus={focus} onClose={close} onNew={add} newLabel="新建会话"/>{!shown.length&&<p className="joystick-empty">{faces.length?'没有匹配的标签':'没有打开的会话'}</p>}</div></>:<div className="joystick-content-demo"><p>当前会话的工作区与最近内容</p>{['README.md','apps/gui/src/components','docs/design-system','最近访问的网页'].map(label=><button key={label} onClick={()=>setScene('terminal')}>{label}</button>)}</div>}
      </section></MobileContentSheet>}
        </SessionScreen>
      </JoystickContext.Provider></div>
    </section>
  </main></TooltipProvider>;
}

const root=createRoot(document.getElementById('root')!);root.render(<Lab/>);
if(import.meta.hot)import.meta.hot.dispose(()=>root.unmount());
