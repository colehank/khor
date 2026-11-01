// Real shared headers and scroll-edge binding; no Khor node or native window.
import {useState} from 'react';
import {createRoot} from 'react-dom/client';
import {PaneBar} from '@/components/PaneBar';
import {TooltipProvider} from '@/components/ui/tooltip';
import {Button} from '@/components/ui/button';
import {band,followScrollEdge,settleBands} from '@/lib/scroll-edge';
import {speakLang} from '@/lang';
import '@/styles/tokens.css';
import '@/styles/app.css';
import './scroll-surfaces.css';
speakLang('zh');followScrollEdge();
const wait=()=>new Promise<void>(resolve=>requestAnimationFrame(()=>requestAnimationFrame(()=>resolve())));
function Lab(){
 const processes=new URLSearchParams(location.search).get('surface')==='processes';
 const [report,setReport]=useState('');
 const material=(transparent:boolean,opacity=45)=>{
   document.documentElement.toggleAttribute('data-window-transparent',transparent);
   document.documentElement.style.setProperty('--window-opacity',`${opacity}%`);
 };
 const inspect=async()=>{
   const results:unknown[]=[];
   for(const theme of ['dark','light'])for(const transparent of [false,true])for(const opacity of [0,45,100]){
     document.documentElement.dataset.theme=theme;material(transparent,opacity);settleBands();await wait();
     for(const pane of document.querySelectorAll<HTMLElement>('[data-scroll-proof]')){
       const head=pane.querySelector<HTMLElement>('[data-band]')!,scroll=pane.querySelector<HTMLElement>('[data-band-scroll]')!;
       scroll.scrollTop=0;await wait();
       const first=scroll.firstElementChild!.getBoundingClientRect().top,range=scroll.scrollHeight-scroll.clientHeight;
       scroll.scrollTop=180;await wait();
       const paint=getComputedStyle(head,'::before'),title=getComputedStyle(head),body=getComputedStyle(scroll);
       const sr=scroll.getBoundingClientRect(),hr=head.getBoundingClientRect();
       const clear=paint.backgroundColor==='rgba(0, 0, 0, 0)'&&title.backgroundColor==='rgba(0, 0, 0, 0)';
       const titleFull=title.maskImage==='none'&&title.opacity==='1';
       const noDivider=(parseFloat(title.borderBottomWidth)||0)===0&&title.boxShadow==='none';
       const fadeStart=sr.top+(parseFloat(scroll.style.getPropertyValue('--sticky-fade-start'))||0);
       const fadeEnd=fadeStart+(parseFloat(scroll.style.getPropertyValue('--sticky-fade-length'))||0);
       const bodyFades=scroll.hasAttribute('data-sticky-fading')&&body.maskImage!=='none'&&Math.abs(fadeStart-hr.top)<1&&fadeEnd>=hr.bottom;
       const pass=head.hasAttribute('data-scrolled')&&first>=hr.bottom-1&&clear&&titleFull&&noDivider&&bodyFades;
       results.push({pane:pane.dataset.scrollProof,theme,transparent,opacity,pass,clear,titleFull,noDivider,bodyFades,fadeStart,fadeEnd,headTop:hr.top,headBottom:hr.bottom,range,first,top:scroll.scrollTop});
     }
   }
   material(true);document.documentElement.dataset.theme='dark';await wait();
   setReport(JSON.stringify({pass:results.every(r=>(r as {pass:boolean}).pass),results},null,2));
 };
 return <TooltipProvider><main className="scroll-proof"><header><strong>共享列表标题 · CSS 合成验收</strong><Button onClick={()=>material(true)}>半透明</Button><Button onClick={()=>material(false)}>实色</Button><Button onClick={()=>{document.documentElement.dataset.theme=document.documentElement.dataset.theme==='dark'?'light':'dark'}}>明暗</Button><Button onClick={()=>void inspect()}>检查列表</Button></header><p>背景图案仅用于观察透明层；不模拟原生毛玻璃。</p><div className="scroll-proof-columns" style={processes?{gridTemplateColumns:'minmax(0,1fr)'}:undefined}>{processes?<section data-scroll-proof="processes">
 <div ref={band} data-band className="scroll-proof-head">Processes</div>
 <div className="scroll-proof-list"><div data-load-top data-sticky-group role="table" className="text-callout tabular-nums" style={{gridTemplateColumns:'max-content max-content minmax(0,1fr)'}}>
 <div data-top-heads data-sticky-head role="row" className="bg-card text-callout text-muted-foreground">{['PID','Status','Command'].map(label=><div key={label} role="columnheader"><button type="button" className="inline-flex min-h-touch min-w-touch w-full items-center gap-in"><span data-process-heading>{label}</span></button></div>)}</div>
 <div data-top-body data-sticky-body="flush" role="rowgroup">{Array.from({length:40},(_,n)=><div key={n} data-top-row={n} role="row">{[String(88000+n),'Running',`PROCESS_BODY_${n}_readable_command`].map((text,i)=><span key={i} role="cell" className="py-in">{text}</span>)}</div>)}</div>
 </div></div>
 </section>:['会话','文件','机器'].map((name,i)=><section key={name} data-scroll-proof={name}>
 {i===1?<PaneBar searchLabel="搜索目录文件" query="" onQuery={()=>{}} band/>:<div ref={band} data-band className="scroll-proof-head">{name}</div>}
 <div className="scroll-proof-list">{Array.from({length:40},(_,n)=><div key={n}>条目 {n+1} · {name}内容</div>)}</div>
 </section>)}</div><details open><summary>验收记录</summary><output data-scroll-proof-report>{report}</output></details></main></TooltipProvider>;
}
createRoot(document.getElementById('root')!).render(<Lab/>);
