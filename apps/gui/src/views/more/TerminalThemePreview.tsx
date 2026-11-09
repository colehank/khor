import {useEffect,useLayoutEffect,useRef} from "react";
import {Terminal} from "@xterm/xterm";
import {FitAddon} from "@xterm/addon-fit";
import {protectTerminal} from "@/lib/term-legibility";
import {stackOf,useTermFont} from "@/lib/term-font";
import {useTypeSize} from "@/lib/type-size";
import {useTermCursor} from "@/lib/term-cursor";
import {onTermTheme,termTheme,terminalAppearance,type Palette} from "@/lib/term-theme";

const SAMPLE = "\x1b[32mzgh@MacLab\x1b[0m \x1b[34m~/khor\x1b[0m\r\n中文 · English · \x1b[31merror\x1b[0m · \x1b[33mwarning\x1b[0m\r\n\x1b[30m0 \x1b[31m1 \x1b[32m2 \x1b[33m3 \x1b[34m4 \x1b[35m5 \x1b[36m6 \x1b[37m7\x1b[0m    \x1b[90m8 \x1b[91m9 \x1b[92m10 \x1b[93m11 \x1b[94m12 \x1b[95m13 \x1b[96m14 \x1b[97m15\x1b[0m\r\n\x1b[38;5;16m16\x1b[0m · \x1b[38;2;128;160;200mRGB\x1b[0m \x1b[48;2;31;31;31m RGB background \x1b[0m > ";

/** A read-only real renderer with the same live appearance as session terminals. */
export function TerminalThemePreview({theme}: {theme?:Palette}) {
  const box=useRef<HTMLDivElement>(null);
  const font=useTermFont(),size=useTypeSize(),cursor=useTermCursor();
  const current=useRef({theme,font:font.choice,size:size.term,cursor});
  current.current={theme,font:font.choice,size:size.term,cursor};
  const repaint=useRef<(()=>void)|null>(null);
  useLayoutEffect(()=>{
    const el=box.current;if(!el)return;
    const options=()=>({
      ...terminalAppearance(el,termTheme(),current.current.theme),
      fontFamily:stackOf(current.current.font),fontSize:current.current.size,
      cursorStyle:current.current.cursor.shape,cursorInactiveStyle:current.current.cursor.shape,
      cursorBlink:current.current.cursor.blink,
    });
    const term=new Terminal({cols:58,rows:8,disableStdin:true,scrollback:0,...options()});
    let alive=true;
    const fit=new FitAddon();term.loadAddon(fit);term.open(el);
    const stopProtection=protectTerminal(term);
    const marked=el as HTMLDivElement&{__xterm?:Terminal};marked.__xterm=term;
    const paint=()=>{if(!alive)return;term.options=options();if(el.clientWidth&&el.clientHeight)fit.fit();term.refresh(0,term.rows-1);};
    repaint.current=paint;
    const stopTheme=onTermTheme(paint);
    const resized=new ResizeObserver(paint);resized.observe(el);
    term.write(SAMPLE,paint);
    return()=>{alive=false;repaint.current=null;resized.disconnect();stopTheme();stopProtection();delete marked.__xterm;term.dispose();};
  },[]);
  useEffect(()=>{repaint.current?.();},[font.choice,size.term,cursor,theme]);
  return <div ref={box} data-term-theme-preview className="min-w-0 flex-none overflow-hidden rounded-md p-in font-term text-term text-foreground"
    style={{backgroundColor:theme?.background??"var(--term-bg)",color:theme?.foreground??"var(--term-fg)",height:"calc(8 * var(--text-term--line-height) + 2 * var(--gap-in))",fontFamily:"var(--font-term)",fontSize:"var(--text-term)"}}/>;
}
