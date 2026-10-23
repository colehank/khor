// ACP adapters for the official shadcn/ui AI Elements patterns.
// Presentation primitives are vendored in ui/ai. Provenance: third-party/shadcn-ui.
import { lazy, Suspense, useRef, useState } from "react";
import { Markdown } from "@/components/Markdown";
import { CodeBlock, CodeBlockHeader, CodeBlockTitle, CodeBlockActions, CodeBlockCopyButton, CodeBlockContent } from "@/components/ui/ai/code-block";
import { Button } from "@/components/ui/ai/button";
import { Item, ItemMedia, ItemContent, ItemTitle, ItemDescription, ItemActions } from "@/components/ui/ai/item";
import { Trace, TraceCollapsible, TraceCollapsibleContent, TraceCollapsibleTrigger, TraceDescription, TraceIcon, TraceLine, TraceTitle } from "@/components/ui/ai/trace";
import { CircleCheckIcon, CircleDotIcon, CircleIcon, FilePenLineIcon, FileTextIcon, LoaderCircleIcon, RouteIcon, SearchIcon, TerminalIcon, WrenchIcon } from "./icons";
import { gui } from "@/gen/catalog";
import { word } from "@/words";
import type { AcpTool } from "@/lib/acp-view";

const AgentDiff = lazy(() => import("./diff").then((module) => ({ default: module.AgentDiff })));
const AgentPatch = lazy(() => import("./diff").then((module) => ({ default: module.AgentPatch })));

export type AgentToolData = AcpTool;

export type AgentPlanEntry = { content: string; status: string };

export function running(status: string): boolean {
  return status === "in_progress" || status === "pending";
}

export function ToolDot({ status }: { status: string }) {
  const Icon = running(status) ? LoaderCircleIcon : CircleCheckIcon;
  return <Icon data-tool-dot data-dot-state={running(status) ? "running" : status === "failed" ? "failed" : "done"} className={running(status) ? "size-4 motion-safe:animate-spin" : "size-4"} aria-hidden />;
}

// Reasoning pattern: upstream uses a text label and disclosure, no
// decorative icon. Explicit user state wins over the streaming default.
export function AgentReasoning({ text, isStreaming }: { text: string; isStreaming: boolean }) {
  const [open, setOpen] = useState<boolean>();
  return (
    <Trace data-chat-reasoning>
      <TraceCollapsible open={open ?? isStreaming} onOpenChange={setOpen}>
        <TraceCollapsibleTrigger data-reasoning-toggle>
          <TraceLine status={isStreaming ? "running" : undefined}>
            <TraceTitle>{isStreaming ? gui.chat_reasoning_active : gui.chat_reasoning}</TraceTitle>
          </TraceLine>
        </TraceCollapsibleTrigger>
        <TraceCollapsibleContent><div className="text-read"><Markdown text={text} className="text-muted-foreground" /></div></TraceCollapsibleContent>
      </TraceCollapsible>
    </Trace>
  );
}

export function AgentTool({ tool, active = true, open, onOpenChange, onOpenFile }: {
  tool: AgentToolData;
  active?: boolean;
  open: boolean;
  onOpenChange: (open: boolean) => void;
  onOpenFile: (path: string) => void;
}) {
  const diffs = tool.diffs ?? (tool.diff ? [tool.diff] : []), patches = tool.patches ?? [];
  const more = diffs.length > 0 || patches.length > 0 || tool.out !== null || tool.raw !== null;
  const live = active && running(tool.status), failed = tool.status === "failed";
  const icons: Record<string, typeof WrenchIcon> = { read: FileTextIcon, edit: FilePenLineIcon, search: SearchIcon, execute: TerminalIcon };
  const Icon = icons[tool.kind] ?? WrenchIcon;
  const line = <TraceLine status={live ? "running" : failed ? "error" : undefined}>
    <TraceIcon><Icon /></TraceIcon><TraceTitle title={tool.title}>{tool.title}</TraceTitle>
    {live && <TraceIcon><LoaderCircleIcon className="motion-safe:animate-spin" /></TraceIcon>}
  </TraceLine>;
  const raw = tool.raw && typeof tool.raw === "object" ? tool.raw as Record<string, unknown> : {};
  const command = typeof raw.command === "string" ? raw.command : null;
  const wrote = tool.kind === "edit" && tool.status === "completed" ? [tool.diff?.path, ...tool.where].find((path): path is string => !!path && path.startsWith("/")) : null;
  return (
    <Trace data-chat-tool data-tool-id={tool.id} data-tool-status={tool.status} data-tool-kind={tool.kind} data-tool-open={more ? (open ? "yes" : "no") : undefined}>
      {more ? <TraceCollapsible open={open} onOpenChange={onOpenChange}>
        <TraceCollapsibleTrigger data-tool-toggle title={tool.title} aria-label={tool.title}>{line}</TraceCollapsibleTrigger>
        <TraceCollapsibleContent>
          <div data-tool-body className="flex min-w-0 flex-col gap-row">
            {diffs.map((diff, i) => <Suspense key={`${diff.path}:${i}`} fallback={<div data-diff-loading className="h-16 rounded-md bg-muted" aria-hidden />}><AgentDiff diff={diff} wrote={tool.kind === "edit" && tool.status === "completed" && diff.path.startsWith("/") ? diff.path : wrote ?? null} onOpenFile={onOpenFile} /></Suspense>)}
            {patches.map((patch, i) => <Suspense key={`${patch.path}:${i}`} fallback={<div data-diff-loading className="h-16 rounded-md bg-muted" aria-hidden />}><AgentPatch path={patch.path} text={patch.text} onOpenFile={tool.status === "completed" && patch.path.startsWith("/") ? onOpenFile : undefined} /></Suspense>)}
            {diffs.length === 0 && patches.length === 0 && tool.raw !== null && <CodeBlock size="sm">
              <CodeBlockHeader><CodeBlockTitle>{command ? gui.key_group_commands : gui.chat_tool_input}</CodeBlockTitle><CodeBlockActions><CodeBlockCopyButton value={command ?? JSON.stringify(tool.raw, null, 2)} aria-label={gui.copy_text} /></CodeBlockActions></CodeBlockHeader>
              <CodeBlockContent data-tool-raw className="max-h-64">{command ?? JSON.stringify(tool.raw, null, 2)}</CodeBlockContent>
            </CodeBlock>}
            {tool.out !== null && <CodeBlock size="sm" variant={failed ? "destructive" : "default"}>
              <CodeBlockHeader><CodeBlockTitle>{failed ? word("failed") : gui.usage_axis_output}</CodeBlockTitle><CodeBlockActions><CodeBlockCopyButton value={tool.out} aria-label={gui.copy_text} /></CodeBlockActions></CodeBlockHeader>
              <CodeBlockContent data-tool-out className="max-h-64">{tool.out}</CodeBlockContent>
            </CodeBlock>}
          </div>
        </TraceCollapsibleContent>
      </TraceCollapsible> : line}
    </Trace>
  );
}

export function AgentPlan({ entries }: { entries: AgentPlanEntry[] }) {
  if (entries.length === 0) return null;
  const live = entries.some((entry) => entry.status === "in_progress");
  const done = entries.filter((entry) => entry.status === "completed").length;
  return <Trace data-chat-plan>
    <TraceCollapsible defaultOpen>
      <TraceCollapsibleTrigger><TraceLine status={live ? "running" : undefined}>
        <TraceIcon><RouteIcon /></TraceIcon><TraceTitle>{gui.chat_plan}</TraceTitle><TraceDescription>{done}/{entries.length}</TraceDescription>
      </TraceLine></TraceCollapsibleTrigger>
      <TraceCollapsibleContent><Trace>
        {entries.map((entry, i) => <TraceLine key={i} data-plan-entry data-plan-status={entry.status} status={entry.status === "in_progress" ? "running" : entry.status === "pending" ? "pending" : undefined}>
          <TraceIcon>{entry.status === "completed" ? <CircleCheckIcon /> : entry.status === "in_progress" ? <CircleDotIcon /> : <CircleIcon />}</TraceIcon>
          <TraceTitle>{entry.content}</TraceTitle>
        </TraceLine>)}
      </Trace></TraceCollapsibleContent>
    </TraceCollapsible>
  </Trace>;
}

export function AgentConfirmation({ title, options, answer, canAnswer, onAnswer }: {
  title: string; options: [string, string][]; answer: string | null;
  canAnswer: boolean; onAnswer: (option: string) => Promise<unknown>;
}) {
  const [pending, setPending] = useState<string | null>(null);
  const [submitted, setSubmitted] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const sending = useRef(false);
  const chosen = answer;
  if (chosen !== null) return <Trace data-chat-ask>
    <TraceCollapsible>
      <TraceCollapsibleTrigger title={title}><TraceLine>
        <TraceIcon><WrenchIcon /></TraceIcon><TraceTitle data-ask-answer aria-live="polite">{options.find(([id]) => id === chosen)?.[1] ?? (chosen || gui.chat_request_closed)}</TraceTitle>
      </TraceLine></TraceCollapsibleTrigger>
      <TraceCollapsibleContent>{title}</TraceCollapsibleContent>
    </TraceCollapsible>
  </Trace>;
  return <Item data-chat-ask data-ask-active aria-busy={pending !== null} variant="outline" size="sm">
    <ItemMedia variant="icon"><WrenchIcon /></ItemMedia>
    <ItemContent className="min-w-0">
      <ItemTitle>{gui.chat_confirmation}</ItemTitle>
      <ItemDescription className="line-clamp-none [overflow-wrap:anywhere]">{title}</ItemDescription>
      {pending !== null && <p data-ask-pending data-ask-phase={submitted ? "waiting" : "sending"} role="status" className="text-callout text-muted-foreground">{submitted ? gui.chat_confirmation_waiting : gui.chat_confirmation_sending}</p>}
      {error && <p role="alert" className="text-callout text-destructive">{error}</p>}
    </ItemContent>
    {canAnswer && <ItemActions className="flex-wrap">
      {options.map(([id, label]) => <Button key={id} variant="outline" size="sm" disabled={pending !== null} className="h-auto min-h-touch whitespace-normal pointer-fine:min-h-ctl-sm" data-ask-option={id} data-ask-chosen={pending === id || undefined} onClick={async () => {
        if (sending.current) return;
        sending.current = true; setPending(id); setError(null);
        try { await onAnswer(id); setSubmitted(true); }
        catch (why) { sending.current = false; setPending(null); setSubmitted(false); setError(why instanceof Error ? why.message : String(why)); }
      }}>{label}</Button>)}
    </ItemActions>}
  </Item>;
}
