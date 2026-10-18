/** Real product views with a disposable transport. All UI changes go through
 * the same settings and subscriptions used by the app. */
import {useLayoutEffect,useRef,useState} from 'react';
import {Terminal} from '@xterm/xterm';
import {FitAddon} from '@xterm/addon-fit';
import {createRoot} from 'react-dom/client';
import {TerminalPane,EndedPane} from '@/views/TerminalPane';
import {TerminalThemeSection} from '@/views/more/TerminalThemeSection';
import {TooltipProvider} from '@/components/ui/tooltip';
import {Button} from '@/components/ui/button';
import {wearTermPalette,terminalAppearance,onTermTheme} from '@/lib/term-theme';
import {holdsTmuxHistory} from '@/lib/term-tmux-history';
import {speakLang} from '@/lang';
import '@/styles/tokens.css';
import '@/styles/app.css';
import './terminal-colors-production.css';
speakLang('zh');wearTermPalette();
function HistoryProof(){
 const scroller=useRef<HTMLDivElement>(null),room=useRef<HTMLDivElement>(null),mount=useRef<HTMLDivElement>(null),seat=useRef<HTMLDivElement>(null);
 useLayoutEffect(()=>{
   const box=mount.current!,css=getComputedStyle(box);
   const t=new Terminal({cols:80,rows:24,...terminalAppearance(box),fontFamily:css.fontFamily,fontSize:parseFloat(css.fontSize),allowProposedApi:true,disableStdin:true});
   const fit=new FitAddon();t.loadAddon(fit);t.open(box);fit.fit();
   const repaint=()=>{t.options=terminalAppearance(box)};const stop=onTermTheme(repaint);
   const history=holdsTmuxHistory({known:true,live:t,seat:seat.current!,scroller:scroller.current!,room:room.current!,read:async()=>({pane:'%fixture',alternate:false,mouse:false,refill:true,lead:'',lines:Array.from({length:140},(_,i)=>`tmux history ${i}`),screen:['\x1b[48;2;31;31;31m > Ask Codex to do anything \x1b[0m'],anchor:{pane:'%fixture',size:140,tail:[]},top:0,left:0,window:`${t.cols}x${t.rows}`}),changed(){},outOfReach(){},moved(){},gained(){}});
   history.joined();history.shown(true);
   const resize=new ResizeObserver(()=>{if(box.clientWidth)fit.fit()});resize.observe(box);
   return()=>{resize.disconnect();history.stop();stop();t.dispose()};
 },[]);
 return <div className="production-history-wrap"><div ref={scroller} className="production-history-scroller"><div ref={room} className="production-history-room"/></div><div ref={mount} className="production-history-live font-term text-term text-(--term-fg)"/><div ref={seat} data-terminal className="production-history-seat font-term text-term text-(--term-fg)"/></div>;
}
function Proof(){
 const settingsOnly=new URLSearchParams(location.search).has('settings');
 const [dark,setDark]=useState(false),[front,setFront]=useState('live');
 const [report,setReport]=useState('');
 const original=useRef(new Map<string,Terminal>());
 const terminal=(surface:string)=>document.querySelector<HTMLElement>(`[data-proof-surface="${surface}"] [data-terminal], [data-proof-surface="${surface}"] [data-term-ended-screen]`) as (HTMLElement&{__xterm?:Terminal})|null;
 const inspect=async()=>{
   const counts=await fetch('/colour-proof-counts').then(r=>r.json());
   const views=['live','retained','ended','history'].map(id=>{
     const t=terminal(id)?.__xterm;if(!t)return {id,ready:false};
     if(!original.current.has(id))original.current.set(id,t);
     const buffer=t.buffer.active,lines=Array.from({length:buffer.length},(_,i)=>buffer.getLine(i)!);
     const painted=lines.find(l=>l.translateToString().includes('Ask Codex'));
     return {id,same:original.current.get(id)===t,length:buffer.length,y:buffer.viewportY,cols:t.cols,rows:t.rows,ratio:t.options.minimumContrastRatio,bg:t.options.theme?.background,red:t.options.theme?.red,rawBg:painted?.getCell(0)?.getBgColor(),rawRgb:!!painted?.getCell(0)?.isBgRGB()};
   });
   setReport(JSON.stringify({counts,views}));
 };
 useLayoutEffect(()=>{document.documentElement.dataset.theme=dark?'dark':'light';},[dark]);
 return <TooltipProvider><main className="production-colour-proof">
   <header><strong>正式终端配色 · 隔离数据</strong><Button onClick={()=>setDark(v=>!v)}>{dark?'切换浅色':'切换深色'}</Button>{[['live','当前终端'],['retained','保留终端'],['ended','结束快照'],['history','tmux 历史']].map(([id,label])=><Button key={id} aria-pressed={front===id} onClick={()=>setFront(id)}>{label}</Button>)}<Button onClick={()=>terminal(front)?.__xterm?.scrollLines(-30)}>向上浏览历史</Button><Button onClick={()=>void inspect()}>检查连接</Button></header>
   <div className="production-colour-columns" data-settings-only={settingsOnly}>{!settingsOnly&&<section aria-label="终端呈现" className="production-colour-terminal">
     <div data-proof-surface="live" data-proof-visible={front==='live'} aria-hidden={front!=='live'} inert={front!=='live'}><TerminalPane id="shell/colour-live" active={front==='live'}/></div>
     <div data-proof-surface="retained" data-proof-visible={front==='retained'} aria-hidden={front!=='retained'} inert={front!=='retained'}><TerminalPane id="shell/colour-retained" active={front==='retained'}/></div>
     <div data-proof-surface="ended" data-proof-visible={front==='ended'} aria-hidden={front!=='ended'} inert={front!=='ended'}><EndedPane id="shell/colour-ended" at={1}/></div>
     <div data-proof-surface="history" data-proof-visible={front==='history'} aria-hidden={front!=='history'} inert={front!=='history'}><HistoryProof/></div>
   </section>}<aside aria-label="终端配色设置"><TerminalThemeSection/><details className="production-colour-diagnostics"><summary>验收数据</summary><output data-proof-report className="production-colour-report">{report}</output></details></aside></div>
 </main></TooltipProvider>;
}
const root=createRoot(document.getElementById('root')!);root.render(<Proof/>);
if(import.meta.hot)import.meta.hot.dispose(()=>root.unmount());
