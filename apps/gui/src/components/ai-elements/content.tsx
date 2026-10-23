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
