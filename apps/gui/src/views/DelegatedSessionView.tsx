import { useEffect, useLayoutEffect, useMemo, useRef, useState } from "react";
import { khorAgent, type SessionRow } from "@/api";
import { delegatedSessionRef, rememberDelegatedSession, type DelegatedChild } from "@/lib/delegated-sessions";
import { useHidden } from "@/hooks/use-hidden";
import { useChatScroll } from "@/hooks/use-chat-scroll";
import { offerTabAct } from "@/lib/tab-acts";
import { askOpenPath } from "@/lib/composer-handoff";
import { AgentContent, AgentTranscript } from "@/components/ai-elements/content";
import { AgentResource } from "@/components/ai-elements/resource";
import { Bubble, BubbleContent } from "@/components/ui/ai/bubble";
import { MessageActions } from "@/components/MessageActions";
import { Markdown } from "@/components/Markdown";
import { ScrollToBottom } from "@/components/ScrollToBottom";
import { StatusWord } from "@/components/StatusWord";
import { Button } from "@/components/ui/button";
import { BarSeat } from "@/components/SessionBar";
import { gui } from "@/gen/catalog";
import type { AcpResource } from "@/lib/acp-view";

type Part = { text?: string; raw?: string; url?: string; data?: unknown; filename?: string; mediaType?: string };
type TranscriptMessage = { id: string; role: "user" | "agent"; parts: Part[]; updates?: string[] | null; trace_partial?: boolean };
type Readout = DelegatedChild & { messages: TranscriptMessage[]; artifacts?: {artifactId?: string; name?: string; parts: Part[]}[] | null; terminal: boolean };
const positions = new Map<string, {top:number; following:boolean}>();

function ResourcePart({ part, onOpenFile }: { part: Part; onOpenFile: (path: string) => void }) {
  const item = useMemo<AcpResource>(() => {
    let data = part.raw, mime = part.mediaType, name = part.filename ?? "";
    if (part.data !== undefined) {
      const bytes = new TextEncoder().encode(JSON.stringify(part.data, null, 2));
      let binary = "";
      for (let from = 0; from < bytes.length; from += 16384) binary += String.fromCharCode(...bytes.subarray(from, from + 16384));
      data = btoa(binary); mime = "application/json"; name ||= "result.json";
    }
    return {who:"resource",kind:mime?.startsWith("image/")?"image":mime?.startsWith("audio/")?"audio":"resource",name,uri:part.url,data,mime};
  }, [part]);
  return <AgentResource item={item} onOpenFile={onOpenFile}/>;
}

/** This reader has no control attachment, prompt draft, or reply transport. */
export function DelegatedSessionView({ row, active, onParent }: { row: SessionRow; active: boolean; onParent?: () => void }) {
  const reference = delegatedSessionRef(row.id)!;
  const root = useRef<HTMLElement>(null);
  const hidden = useHidden();
  const [readout, setReadout] = useState<Readout | null>(null);
  const [failure, setFailure] = useState<string | null>(null);
  const [attempt, setAttempt] = useState(0);
  const {scroller,column,onScroll,captureDisclosure,toBottom,away,fresh,following,readingLatest,capturePositionTicket,acceptRestoredPosition} = useChatScroll(readout);
  const restore = useRef(positions.get(row.id));
  useLayoutEffect(()=>{
    const saved=restore.current;
    if(!readout||!saved)return;
    restore.current=undefined;
    following.current=saved.following;readingLatest.current=saved.following;
    if(!saved.following)acceptRestoredPosition(capturePositionTicket(),()=>{if(scroller.current)scroller.current.scrollTop=saved.top;});
  },[readout,following,readingLatest,scroller,capturePositionTicket,acceptRestoredPosition]);
  useLayoutEffect(()=>{
    const element=root.current,scroll=scroller.current;
    return()=>{
      if(scroll&&!restore.current){positions.set(row.id,{top:scroll.scrollTop,following:following.current});if(positions.size>256)positions.delete(positions.keys().next().value!);}
      element?.querySelectorAll<HTMLMediaElement>('audio,video').forEach(media=>{media.pause();media.removeAttribute('src');media.load();});
    };
  },[row.id,scroller,following]);
  useEffect(() => offerTabAct(row.id, {kind:"reload",run:()=>setAttempt(value=>value+1)}), [row.id]);
  useEffect(() => {
    if (!active || hidden) return;
    let stopped = false;
    let timer: ReturnType<typeof setTimeout> | undefined;
    const read = async () => {
      let terminal = false;
      try {
        const {data} = await khorAgent("manage", reference.owner, {operation:"children",session:reference.parent,detail:{run:reference.run,key:reference.key}});
        const value = data as Readout | null;
        if (!Array.isArray(value?.messages) || value.run !== reference.run || value.key !== reference.key) throw new Error(gui.child_task_unavailable);
        terminal = value.terminal;
        if (!stopped) { setReadout(value); setFailure(null); rememberDelegatedSession(row.id,value); }
      } catch (error) { if (!stopped) setFailure(String(error)); }
      if (!stopped && !terminal) timer = setTimeout(read, 5000);
    };
    void read();
    return () => { stopped = true; clearTimeout(timer); };
  }, [row.id, active, hidden, attempt]);
  const openFile = (path: string) => askOpenPath(row.home,path,false,null,row.id);
  return <section ref={root} data-child-session data-child-session-readonly data-child-session-ref={row.id} className="relative flex min-h-0 flex-1 flex-col">
    <div ref={scroller} data-chat-scroll onScroll={onScroll} onClickCapture={captureDisclosure} className="min-h-0 flex-1 overflow-y-auto px-4 py-3 [overflow-anchor:none]">
      <div ref={column} data-chat-column data-selectable className="mx-auto w-full max-w-measure space-y-1 break-words text-read">
        {failure && <div role="status" className="flex items-center gap-in py-in text-callout text-muted-foreground"><span>{failure}</span><Button size="sm" onClick={()=>setAttempt(value=>value+1)}>{gui.region_retry}</Button></div>}
        {!readout && !failure && <p role="status" className="py-row text-callout text-muted-foreground">{gui.content_loading}</p>}
        {readout?.messages.map(message => {
          const text = message.parts.filter(part=>typeof part.text==="string").map(part=>part.text!).join("\n");
          return <div key={message.id} data-child-session-message={message.id} data-child-session-role={message.role}>
            {message.role === "agent" && message.updates?.length ? <AgentTranscript updates={message.updates} busy={false} fallback="" onOpenFile={openFile}/> : null}
            {text && (message.role === "user" ? <div role="article" aria-label="你" className="flex flex-col items-end gap-in py-row"><MessageActions text={text} align="end"><div className="flex justify-end"><Bubble align="end" variant="muted"><BubbleContent data-said className="text-read"><Markdown text={text}/></BubbleContent></Bubble></div></MessageActions></div> :
              <AgentContent item={{who:"agent",text}} streaming={false} active={false} open={false} onOpenChange={()=>{}} onOpenFile={openFile}/>)}
            {message.parts.filter(part=>typeof part.text!=="string").map((part,index)=><ResourcePart key={index} part={part} onOpenFile={openFile}/>)}
            {message.trace_partial && <p className="py-in text-caption text-muted-foreground">{gui.child_session_trace_partial}</p>}
          </div>;
        })}
        {readout?.artifacts?.map((artifact,index)=><section key={artifact.artifactId??index} data-child-session-artifact className="py-row">
          {artifact.name&&<h3 className="text-body font-title">{artifact.name}</h3>}
          {artifact.parts.map((part,index)=>typeof part.text==="string"?<Markdown key={index} text={part.text}/>:<ResourcePart key={index} part={part} onOpenFile={openFile}/>)}
        </section>)}
        {readout && readout.messages.length===0 && !readout.artifacts?.length && <p className="py-row text-callout text-muted-foreground">{gui.child_session_empty}</p>}
      </div>
    </div>
    {away&&<ScrollToBottom fresh={fresh} onClick={toBottom}/>}
    <BarSeat at="above"><div className="flex min-h-touch flex-none items-center gap-in px-row py-in text-caption text-muted-foreground">
      <span data-chat-readonly className="min-w-0 flex-1">{gui.chat_readonly}</span><StatusWord word={row.word}/>
      {onParent&&<Button size="sm" variant="ghost" data-child-session-parent onClick={onParent}>{gui.child_session_parent}</Button>}
    </div></BarSeat>
  </section>;
}
