/** A colour experiment with the installed xterm renderers and production
 * palettes. No bridge, PTY, agent process or persisted setting is touched. */
import {useEffect,useLayoutEffect,useRef,useState} from 'react';
import {createRoot} from 'react-dom/client';
import {Terminal,type ITheme} from '@xterm/xterm';
import {FitAddon} from '@xterm/addon-fit';
import {WebglAddon} from '@xterm/addon-webgl';
import {PRESETS,terminalTheme} from '@/lib/term-theme';
import '@xterm/xterm/css/xterm.css';
import '@/styles/tokens.css';
import './terminal-colors.css';

type Mode='light'|'dark';
type Scene='cached'|'fresh'|'samples';
type Engine='dom'|'webgl';
type Pair='catppuccin'|'tokyo';
type Policy='follow'|'pair'|'protected';
const ESC='\x1b',RESET=ESC+'[0m';
const presets={catppuccin:{light:'latte',dark:'mocha'},tokyo:{light:'tokyo-day',dark:'tokyo'}};
// Read the checked-in upstream IDs rather than introducing a new colour table.
function paired(pair:Pair,mode:Mode):ITheme{
  const ids=presets[pair];
  const id=ids[mode];
  const palette=PRESETS.find(p=>p.id===id);
  if(!palette)throw Error('Missing upstream preset '+id);
  return palette.theme;
}
function parseRgb(reply:string):number[]|null{
  const m=reply.match(/rgb:([\da-f]+)\/([\da-f]+)\/([\da-f]+)/i);
  return m?m.slice(1).map(s=>Math.round(parseInt(s,16)*255/(16**s.length-1))):null;
}
const hex=(rgb:number[])=>'#'+rgb.map(c=>c.toString(16).padStart(2,'0')).join('');
const sgr=(colour:number[],background=false)=>`${ESC}[${background?48:38};2;${colour.join(';')}m`;
const line=(text:string)=>text+'\r\n';
// Mirrors Codex style.rs user_message_bg_rgb: black over light at .04,
// white over dark at .12. This is controlled ANSI data, not captured chat.
function promptBg(bg:number[]){
  const light=bg[0]*.299+bg[1]*.587+bg[2]*.114>128;
  const alpha=light?.04:.12,top=light?0:255;
  return bg.map(c=>Math.round(c*(1-alpha)+top*alpha));
}
function fixture(cols:number,scene:Scene,bg:number[]):string{
  const width=Math.max(1,cols-2),fill=promptBg(bg);
  const bar=(s:string)=>sgr(fill,true)+s.padEnd(width,' ')+RESET;
  const ansi=(n:number,s:string)=>`${ESC}[${n}m${s}${RESET}`;
  let text=line(ansi(2,'OpenAI Codex  /  controlled terminal fixture'))+'\r\n';
  text+=line(bar(' > Update terminal colours'))+'\r\n';
  text+=line('Theme belongs to the viewer.')+line('The stream may keep an older RGB fill.')+'\r\n';
  text+=line(ansi(32,'• Ran ')+ansi(34,'git status --short'));
  text+=line(ansi(2,'  main · working tree clean'))+'\r\n';
  text+=line(ansi(31,'• Hook failed'))+line(ansi(2,'  hook exited with code 1'))+'\r\n';
  text+=line(ansi(36,'Read ')+ansi(94,'src/terminal-colours.ts'));
  text+=line(ansi(2,'  Secondary text keeps its DIM attribute.'))+'\r\n';
  text+=line(bar(' > Ask Codex to do anything'));
  text+=line(ansi(2,'Enter to send · Ctrl+C to interrupt'))+'\r\n';
  text+=line(ansi(93,'Warning')+'  '+ansi(92,'Success')+'  '+ansi(91,'Error')+'  '+ansi(94,'Link'));
  if(scene==='samples'){
    text+='\r\n'+line('ANSI 16 / unchanged indices');
    for(let row=0;row<2;row++)text+=line(Array.from({length:8},(_,i)=>`${ESC}[${(row?90:30)+i}mAa ${RESET}`).join(''));
    text+='\r\n'+line('ANSI 256 / fixed cube');
    text+=line([196,208,226,46,51,21,201].map(n=>`${ESC}[38;5;${n}mAa ${RESET}`).join(''));
    text+='\r\n'+line('RGB / paired foreground & background');
    text+=line(sgr([200,220,240])+sgr([25,40,60],true)+' TRUECOLOR '+RESET+'  '+sgr([30,60,90])+sgr([220,235,245],true)+' TRUECOLOR '+RESET);
    text+=line(`${ESC}[7m Reverse ${RESET}  ${ESC}[4mUnderline${RESET}  ${ESC}[2mDIM${RESET}`);
  }
  return text;
}
function contrast(a:number[],b:number[]){
  const lum=(cs:number[])=>cs.map(c=>c/255).map(c=>c<=.04045?c/12.92:((c+.055)/1.055)**2.4).reduce((s,v,i)=>s+v*[.2126,.7152,.0722][i],0);
  const x=lum(a),y=lum(b);return (Math.max(x,y)+.05)/(Math.min(x,y)+.05);
}
type Probe={fg:string;bg:string;fill:string;raw:number;drawn?:number;renderer:string};
function TerminalSample({policy,pair,mode,scene,ratio,engine,epoch}:{policy:Policy;pair:Pair;mode:Mode;scene:Scene;ratio:number;engine:Engine;epoch:number}){
  const mount=useRef<HTMLDivElement>(null);
  const [probe,setProbe]=useState<Probe|null>(null);
  const latest=useRef({pair,mode,scene,ratio,epoch});latest.current={pair,mode,scene,ratio,epoch};
  const paint=useRef<()=>void>(()=>{});
  useLayoutEffect(()=>{
    const host=mount.current!;
    const font=getComputedStyle(host);
    const term=new Terminal({allowProposedApi:true,cols:48,rows:29,scrollback:200,cursorBlink:false,cursorInactiveStyle:'none',fontFamily:font.fontFamily,fontSize:parseFloat(font.fontSize),lineHeight:1.12,screenReaderMode:true});
    const fit=new FitAddon();term.loadAddon(fit);term.open(host);fit.fit();
    if(term.textarea){term.textarea.readOnly=true;term.textarea.tabIndex=-1;}
    let renderer='DOM',disposed=false,serial=0;
    let gl:WebglAddon|undefined;
    if(engine==='webgl')try{gl=new WebglAddon();term.loadAddon(gl);renderer='WebGL';}catch{renderer='DOM · WebGL unavailable';}
    const draw=async()=>{
      const run=++serial,options=latest.current;
      const theme=policy==='follow'?terminalTheme(host,{selected:'follow',customs:[]}):paired(options.pair,options.mode);
      term.options.minimumContrastRatio=policy==='protected'?options.ratio:1;
      term.options.theme=theme;
      host.style.backgroundColor=theme.background||'';
      const answers=new Map<number,number[]>();
      // These are real xterm OSC answers, retained entirely within the lab.
      const heard=term.onData(data=>{
        const match=data.match(/\](10|11);/),value=parseRgb(data);
        if(match&&value)answers.set(Number(match[1]),value);
      });
      await new Promise<void>(resolve=>term.write(`${ESC}]10;?${ESC}\\${ESC}]11;?${ESC}\\`,resolve));heard.dispose();
      if(disposed||run!==serial)return;
      const foreground=answers.get(10)!,background=answers.get(11)!;
      if(!foreground||!background)return;
      // Cached case is one identical RGB stream on all viewers. Fresh
      // case intentionally reprobes and re-renders for each viewer.
      const outputBg=options.scene==='cached'?[0,0,0]:background;
      term.reset();term.write(fixture(term.cols,options.scene,outputBg),()=>{
        if(disposed||run!==serial)return;
        const fill=promptBg(outputBg);
        setProbe({fg:hex(foreground),bg:hex(background),fill:hex(fill),raw:contrast(foreground,fill),renderer});
        requestAnimationFrame(()=>{
          if(disposed||run!==serial||renderer!=='DOM')return;
          // Browser DOM styles reflect xterm's real foreground correction.
          const node=[...host.querySelectorAll<HTMLElement>('.xterm-rows span')].find(e=>e.textContent?.includes('Update terminal colours'));
          if(!node)return;
          const c=getComputedStyle(node).color.match(/[\d.]+/g)?.slice(0,3).map(Number);
          if(c)setProbe(p=>p?{...p,drawn:contrast(c,fill)}:p);
        });
      });
    };
    paint.current=()=>void draw();
    paint.current();
    let width=host.clientWidth;
    const resize=new ResizeObserver(()=>{if(host.clientWidth!==width){width=host.clientWidth;fit.fit();paint.current();}});resize.observe(host);
    return()=>{disposed=true;serial++;resize.disconnect();term.dispose();};
  },[engine,policy]);
  useEffect(()=>paint.current(),[pair,mode,scene,ratio,epoch]);
  const names={follow:'现有跟随',pair:'成对配色',protected:'配色 + 可读性保护'};
  const hints={follow:'同一套 ANSI 色表',pair:pair==='catppuccin'?'Catppuccin Latte / Mocha':'Tokyo Day / Night',protected:`xterm 最低对比度 ${ratio}:1`};
  return <article data-policy={policy}>
    <header><h2>{names[policy]}</h2><p>{hints[policy]}</p></header>
    <div className="colour-terminal" ref={mount}/>
    <footer>
      <span>{probe?.renderer||'准备终端'}</span>
      <span title="默认文字与程序显式背景之间的原始对比度">原始 {probe?.raw.toFixed(2)||'—'}:1{probe?.drawn!==undefined&&<> → 实绘 <b data-contrast={probe.drawn}>{probe.drawn.toFixed(2)}:1</b></>}</span>
    </footer>
    <details><summary>颜色探测</summary><dl><dt>终端文字 · OSC 10</dt><dd data-probe-fg>{probe?.fg}</dd><dt>终端背景 · OSC 11</dt><dd data-probe-bg>{probe?.bg}</dd><dt>程序显式背景</dt><dd data-probe-fill>{probe?.fill}</dd></dl></details>
  </article>;
}
function Lab(){
  const [mode,setMode]=useState<Mode>('light'),[pair,setPair]=useState<Pair>('catppuccin'),[scene,setScene]=useState<Scene>('cached'),[ratio,setRatio]=useState(4.5),[engine,setEngine]=useState<Engine>('dom'),[epoch,setEpoch]=useState(0);
  const [mobile,setMobile]=useState(false),[active,setActive]=useState<Policy>('protected');
  useLayoutEffect(()=>{document.documentElement.setAttribute('data-theme',mode);},[mode]);
  return <main className="colour-lab" data-mobile={mobile} data-active={active}>
    <div className="colour-intro"><div><p className="colour-eyebrow">KHOR / TERMINAL LAB</p><h1>颜色跟着你走</h1><p>同一份输出，在浅深主题里都能读清</p></div><div className="colour-mode" role="group" aria-label="观看主题"><button aria-pressed={mode==='light'} onClick={()=>setMode('light')}>浅色</button><button aria-pressed={mode==='dark'} onClick={()=>setMode('dark')}>深色</button></div></div>
    <section className="colour-controls" aria-label="配色实验">
      <label>输出场景<select value={scene} onChange={e=>setScene(e.target.value as Scene)}><option value="cached">沿用深色端的输出</option><option value="fresh">按当前背景重新绘制</option><option value="samples">通用颜色样本</option></select></label>
      <label>成对配色<select value={pair} onChange={e=>setPair(e.target.value as Pair)}><option value="catppuccin">Catppuccin</option><option value="tokyo">Tokyo Night / Day</option></select></label>
      <label>保护强度<select value={ratio} onChange={e=>setRatio(Number(e.target.value))}><option value="3">3:1 · 轻度</option><option value="4.5">4.5:1 · 标准</option><option value="7">7:1 · 加强</option><option value="1">关闭</option></select></label>
      <label>渲染器<select value={engine} onChange={e=>setEngine(e.target.value as Engine)}><option value="dom">DOM · 可测量</option><option value="webgl">WebGL · 桌面常用</option></select></label>
      <button aria-pressed={mobile} onClick={()=>setMobile(v=>!v)}>窄屏预览</button>
      <button onClick={()=>setEpoch(v=>v+1)}>重新探测</button>
    </section>
    <p className="colour-context" role="status">{scene==='cached'?'程序沿用深色背景生成的 RGB 色块。只换色表，无法改变这些色块。':'程序按当前终端背景生成色块。用来区分配色问题与过时输出。'}</p>
    <nav className="colour-mobile-tabs" aria-label="对比方案">{(['follow','pair','protected'] as Policy[]).map((id,i)=><button key={id} aria-pressed={id===active} onClick={()=>setActive(id)}>{['现有','成对配色','加保护'][i]}</button>)}</nav>
    <div className="colour-grid">{(['follow','pair','protected'] as Policy[]).map(policy=><TerminalSample key={policy} {...{policy,mode,pair,scene,ratio,engine,epoch}}/>)}</div>
    <aside className="colour-note"><strong>可读性保护能救文字，不能让旧背景重新适配。</strong><p>切换「按当前背景重新绘制」查看理想的背景层次。DIM 文字按 xterm 原生规则采用一半目标对比度；并非整屏所有字符都保证 4.5:1。</p><p>真实 xterm · 仓库内的上游配色 · 受控 ANSI 夹具 · 不连接现有会话</p></aside>
  </main>;
}
const root=createRoot(document.getElementById('root')!);
root.render(<Lab/>);
if(import.meta.hot)import.meta.hot.dispose(()=>root.unmount());
