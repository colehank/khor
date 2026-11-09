import {useEffect,useLayoutEffect,useRef} from "react";
import {Terminal,type IMarker,type IDecoration} from "@xterm/xterm";
import {FitAddon} from "@xterm/addon-fit";
import "@xterm/xterm/css/xterm.css";
import {bellHaptic,inApp} from "@/api";
import {gui} from "@/gen/catalog";
import {protectTerminal} from "@/lib/term-legibility";
import {drawBlocks} from "@/lib/term-block-decoration";
import {trackMarks} from "@/lib/term-marks";
import {useTermBlockLines} from "@/lib/term-block-lines";
import {termBell,useTermBell,TERM_BELL_MS} from "@/lib/term-bell";
import {stackOf,useTermFont} from "@/lib/term-font";
import {useTypeSize} from "@/lib/type-size";
import {useTermCursor} from "@/lib/term-cursor";
import {onTermTheme,termTheme,terminalAppearance,type Palette} from "@/lib/term-theme";

const mark=(name:string)=>`\x1b]133;${name}\x07`;
const ANSI=Array.from({length:16},(_,i)=>`\x1b[${i<8?30+i:90+i-8}m${i}\x1b[0m`).join(" ");
const SAMPLE=mark("A")+"\x1b[34m~/khor\x1b[0m \ue0a0 main $ "+mark("B")+"printf 'preview'"+mark("C")+
  "\r\n中文 · English · Il1 O0 · \x1b[31merror\x1b[0m · \x1b[33mwarning\x1b[0m"+
  "\r\n"+ANSI+"\r\n\x1b[38;2;128;160;200mRGB\x1b[0m · \x1b[2mDIM\x1b[0m · \x1b[48;2;31;31;31mbackground\x1b[0m"+
  mark("D;0")+"\r\n"+mark("A")+"$ "+mark("B");

/** A host-free real renderer: shared colours, marks and block decorations.
 * Its demonstration cursor paints without taking focus from the settings. */
export function TerminalThemePreview({theme}: {theme?:Palette}) {
  const box=useRef<HTMLDivElement>(null);
  const font=useTermFont(),size=useTypeSize(),cursor=useTermCursor();
  const lines=useTermBlockLines(),bell=useTermBell();
  const current=useRef({theme,font:font.choice,size:size.term,cursor,lines});
  current.current={theme,font:font.choice,size:size.term,cursor,lines};
  const repaint=useRef<(()=>void)|null>(null),activate=useRef<(()=>void)|null>(null);
  const previousBell=useRef(bell);
  useLayoutEffect(()=>{
    const el=box.current;if(!el)return;
    const options=()=>({
      ...terminalAppearance(el,termTheme(),current.current.theme),
      fontFamily:stackOf(current.current.font),fontSize:current.current.size,
      cursorStyle:current.current.cursor.shape,cursorInactiveStyle:"none" as const,
      cursorBlink:current.current.cursor.blink,
    });
    const term=new Terminal({cols:58,rows:64,allowProposedApi:true,disableStdin:true,scrollback:0,...options()});
    const fit=new FitAddon();term.loadAddon(fit);term.open(el);
    // Pointer settings and native colour pickers must retain their own focus.
    if(term.textarea){term.textarea.tabIndex=-1;term.textarea.disabled=true;}
    const marks=trackMarks(term),stopProtection=protectTerminal(term);
    let alive=true,generation=0,layoutKey="",frame=0,finishFrame=0;
    let blocks:ReturnType<typeof drawBlocks>|undefined;
    let cursorDecoration:IDecoration|undefined,cursorMarker:IMarker|undefined;
    let cursorElement:HTMLElement|undefined,blink:Animation|undefined,blinkKey="";
    let bellTimer:ReturnType<typeof setTimeout>|undefined;
    const motion=window.matchMedia("(prefers-reduced-motion: reduce)");
    const marked=el as HTMLDivElement&{__xterm?:Terminal};marked.__xterm=term;
    const paintCursor=(node:HTMLElement)=>{
      node.classList.add("term-preview-cursor");node.dataset.shape=current.current.cursor.shape;
      node.setAttribute("aria-hidden","true");
      const key=String(current.current.cursor.blink&&!motion.matches);
      if(node===cursorElement&&key===blinkKey)return;
      blink?.cancel();cursorElement=node;blinkKey=key;node.dataset.blink=key;
      // xterm's normal cadence: 600 ms visible, then 600 ms hidden.
      blink=key==="true"?node.animate([{opacity:1,offset:0},{opacity:1,offset:0.5},{opacity:0,offset:0.5},{opacity:0,offset:1}],{duration:1200,iterations:Infinity}):undefined;
    };
    const showCursor=()=>{
      blink?.cancel();cursorDecoration?.dispose();cursorMarker?.dispose();cursorElement=undefined;
      cursorMarker=term.registerMarker(0);
      if(cursorMarker){
        cursorDecoration=term.registerDecoration({marker:cursorMarker,x:Math.min(term.buffer.active.cursorX,term.cols-1),width:1,height:1,layer:"top"});
        cursorDecoration?.onRender(paintCursor);
      }
    };
    const decorate=()=>{
      if(!current.current.lines){blocks?.dispose();blocks=undefined;}
      else blocks??=drawBlocks(term,marks);
      if(cursorElement)paintCursor(cursorElement);
    };
    const finish=()=>{
      // Read the renderer's actual cell height, including the chosen font.
      const screen=el.querySelector<HTMLElement>(".xterm-screen");
      if(!screen)return;
      const cell=screen.getBoundingClientRect().height/term.rows;
      if(!Number.isFinite(cell)||cell<=0)return;
      const occupied=term.buffer.active.cursorY+1;
      const css=getComputedStyle(el);
      const height=Math.ceil(occupied*cell+parseFloat(css.paddingTop)+parseFloat(css.paddingBottom));
      el.style.height=`${height}px`;
      term.resize(term.cols,occupied);
      el.dataset.previewRows=String(occupied);
      showCursor();decorate();term.refresh(0,term.rows-1);
    };
    const paint=()=>{
      if(!alive)return;
      term.options=options();
      const palette=term.options.theme;
      if(palette?.cursor)el.style.setProperty("--term-preview-cursor-color",palette.cursor);
      if(palette?.yellow)el.style.setProperty("--term-preview-bell-color",palette.yellow);
      decorate();
      const dimensions=fit.proposeDimensions();
      if(!dimensions||!el.clientWidth)return;
      const cols=Math.max(2,dimensions.cols),key=`${cols}/${current.current.font}/${current.current.size}`;
      if(key===layoutKey){term.refresh(0,term.rows-1);return;}
      layoutKey=key;const ticket=++generation;
      // Only this isolated sample is regenerated on font/width changes.
      // Leave enough rows while parsing so wrapping cannot discard its start.
      blocks?.dispose();blocks=undefined;cursorDecoration?.dispose();cursorMarker?.dispose();blink?.cancel();cursorElement=undefined;
      term.resize(cols,64);term.reset();
      term.write(SAMPLE,()=>{
        if(!alive||ticket!==generation)return;
        cancelAnimationFrame(finishFrame);
        finishFrame=requestAnimationFrame(()=>{if(alive&&ticket===generation)finish();});
      });
    };
    const rung=term.onBell(()=>{
      if(!termBell())return;
      el.dataset.previewBell="true";
      if(inApp&&window.matchMedia("(pointer: coarse)").matches)void bellHaptic().catch(()=>{});
      clearTimeout(bellTimer);bellTimer=setTimeout(()=>{delete el.dataset.previewBell;},TERM_BELL_MS);
    });
    activate.current=()=>{if(alive&&termBell())term.write("\x07");};
    repaint.current=paint;
    const stopTheme=onTermTheme(paint);
    const resized=new ResizeObserver(()=>{cancelAnimationFrame(frame);frame=requestAnimationFrame(paint);});resized.observe(el);
    motion.addEventListener("change",paint);paint();
    return()=>{
      alive=false;generation++;cancelAnimationFrame(frame);cancelAnimationFrame(finishFrame);clearTimeout(bellTimer);
      repaint.current=null;activate.current=null;resized.disconnect();motion.removeEventListener("change",paint);
      stopTheme();rung.dispose();blink?.cancel();cursorDecoration?.dispose();cursorMarker?.dispose();blocks?.dispose();marks.dispose();stopProtection();
      delete marked.__xterm;term.dispose();
    };
  },[]);
  useEffect(()=>{repaint.current?.();},[font.choice,size.term,cursor,theme,lines]);
  useEffect(()=>{if(bell&&!previousBell.current)activate.current?.();previousBell.current=bell;},[bell]);
  return <div ref={box} data-term-theme-preview role="button" tabIndex={0} aria-label={gui.term_preview_activate}
    onMouseDown={event=>event.preventDefault()} onClick={()=>activate.current?.()}
    onKeyDown={event=>{if(event.key==="Enter"||event.key===" "){event.preventDefault();event.stopPropagation();activate.current?.();}}}
    className="min-w-0 flex-none overflow-hidden rounded-md p-in font-term text-term text-foreground"
    style={{backgroundColor:theme?.background??"var(--term-bg)",color:theme?.foreground??"var(--term-fg)",height:"calc(5 * var(--text-term--line-height) + 2 * var(--gap-in))",fontFamily:"var(--font-term)",fontSize:"var(--text-term)"}}/>;
}
