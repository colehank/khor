import { useMemo, useState } from "react";
import { Message, MessageContent } from "@/components/ui/ai/message";
import { Markdown } from "@/components/Markdown";
import { projectAcp, type AcpItem } from "@/lib/acp-view";
import { AgentPlan, AgentReasoning, AgentTool } from "./agent";

/** Identical content components for ACP-hosted agents and khor's ACP receipts. */
export function AgentContent({ item, streaming, active = true, open, onOpenChange, onOpenFile }: {
  item: AcpItem; streaming: boolean; active?: boolean; open: boolean;
  onOpenChange: (open: boolean) => void; onOpenFile: (path: string) => void;
}) {
  if(item.who==='compaction')return <details data-chat-compaction data-compaction-id={item.id} data-compaction-status={item.status} className="rounded-md bg-fill px-row py-in text-callout"><summary>{item.status==='in_progress'?'正在压缩上下文':item.status==='completed'?'上下文已压缩':item.status==='cancelled'?'已取消压缩':item.status==='failed'?'上下文压缩失败':`上下文压缩 · ${item.status}`}</summary>{item.summary&&<Markdown text={item.summary}/>}{item.error&&<p role="alert" className="text-destructive">{item.error}</p>}</details>;
  if(item.who==='resource'){
    if(item.kind==='audio'&&item.data&&item.mime?.startsWith('audio/'))return <audio data-chat-audio controls aria-label={item.name} src={`data:${item.mime};base64,${item.data}`}/>;
    if(item.kind==='image'&&item.data&&item.mime?.startsWith('image/')&&item.mime!=='image/svg+xml')return <img data-chat-image className="max-w-full rounded-md" alt={item.name} src={`data:${item.mime};base64,${item.data}`}/>;
    return <div data-chat-resource className="rounded-md bg-fill px-row py-in text-callout"><span>{item.name}</span>{item.uri&&<code className="block break-all text-caption">{item.uri}</code>}</div>;
  }
  if (item.who === "thought") return <AgentReasoning text={item.text} isStreaming={streaming} />;
  if (item.who === "tool") return <AgentTool tool={item} active={active} open={open} onOpenChange={onOpenChange} onOpenFile={onOpenFile} />;
  return <Message data-chat-message className="text-read py-row"><MessageContent><Markdown text={item.text} className="text-read-ink" /></MessageContent></Message>;
}

export function AgentTranscript({ updates, busy, fallback, onOpenFile }: {
  updates: readonly string[]; busy: boolean; fallback: string; onOpenFile: (path: string) => void;
}) {
  const { items, plan } = useMemo(() => projectAcp(updates), [updates]);
  const [open, setOpen] = useState<ReadonlySet<string>>(new Set());
  return <>
    <AgentPlan entries={plan} />
    {items.map((item, i) => <AgentContent key={item.who === "tool" ? `tool:${item.id}` : `${item.who}:${i}`} item={item}
      streaming={busy && i === items.length - 1} active={busy}
      open={item.who === "tool" && open.has(item.id)} onOpenFile={onOpenFile}
      onOpenChange={(shown) => { if (item.who === "tool") setOpen((now) => { const next = new Set(now); if (shown) next.add(item.id); else next.delete(item.id); return next; }); }} />)}
    {!items.some((item) => item.who === "agent") && fallback && <Message data-chat-message className="text-read py-row"><MessageContent><Markdown text={fallback} className="text-read-ink" /></MessageContent></Message>}
  </>;
}
