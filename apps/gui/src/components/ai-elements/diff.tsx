import { useMemo, useState } from "react";
import { agentDiff, agentDiffContext, agentPatch, type AgentDiffLine } from "@/lib/agent-diff";
import { baseName } from "@/lib/paths";
import { gui } from "@/gen/catalog";
import { Button } from "@/components/ui/ai/button";
import { CodeBlock, CodeBlockHeader, CodeBlockTitle, CodeBlockActions, CodeBlockCopyButton, CodeBlockContent, CodeBlockLine, CodeBlockFooter } from "@/components/ui/ai/code-block";
import { FileTextIcon } from "./icons";

function DiffLine({ row }: { row: AgentDiffLine }) {
  return <CodeBlockLine variant={row.kind === "context" ? "default" : row.kind}
    data-diff-before={row.kind === "removed" || undefined} data-diff-after={row.kind === "added" || undefined} data-diff-context={row.kind === "context" || undefined}
    data-old-line={row.before ?? undefined} data-new-line={row.after ?? undefined}>
    <span className="grid grid-cols-[3ch_3ch_1fr] gap-row">
      <span aria-hidden className="select-none text-end text-muted-foreground" title={gui.chat_diff_before}>{row.before}</span>
      <span aria-hidden className="select-none text-end text-muted-foreground" title={gui.chat_diff_after}>{row.after}</span>
      <span>{row.text || "\n"}{row.noNewline && <span data-diff-no-newline className="ml-row select-none font-sans text-muted-foreground">{gui.chat_diff_no_newline}</span>}</span>
    </span>
  </CodeBlockLine>;
}

export function AgentPatch({ path, text, onOpenFile }: { path: string; text: string; onOpenFile?: (path: string) => void }) {
  const rows = useMemo(() => agentPatch(text), [text]);
  return <CodeBlock data-tool-patch size="sm">
    <CodeBlockHeader><CodeBlockTitle title={path}><FileTextIcon />{baseName(path)}</CodeBlockTitle><CodeBlockActions>
      {onOpenFile && <Button variant="ghost" size="sm" data-tool-file={path} onClick={() => onOpenFile(path)}>{gui.web_open}</Button>}<CodeBlockCopyButton value={text} aria-label={gui.copy_text} />
    </CodeBlockActions></CodeBlockHeader>
    <CodeBlockContent className="max-h-64">{rows.map((row, i) => row.kind === "heading" ? <CodeBlockLine key={i} className="text-muted-foreground" data-diff-hunk>{row.text}</CodeBlockLine> : <DiffLine key={i} row={row} />)}</CodeBlockContent>
  </CodeBlock>;
}

export function AgentDiff({ diff, wrote, onOpenFile }: {
  diff: { path: string; old: string | null; next: string };
  wrote: string | null;
  onOpenFile: (path: string) => void;
}) {
  const lines = useMemo(() => agentDiff(diff.old, diff.next), [diff.old, diff.next]);
  const [expanded, setExpanded] = useState<ReadonlySet<number>>(new Set());
  const [full, setFull] = useState(false);
  const rows = agentDiffContext(lines, expanded, full);
  const hasGaps = agentDiffContext(lines, new Set()).some((r) => r.kind === "gap");
  const changed = lines.some((r) => r.kind !== "context");
  return <CodeBlock data-tool-diff size="sm">
    <CodeBlockHeader><CodeBlockTitle title={diff.path}><FileTextIcon />{baseName(diff.path)}</CodeBlockTitle>
      <CodeBlockActions>{wrote && <Button variant="ghost" size="sm" data-tool-file={wrote} onClick={() => onOpenFile(wrote)}>{gui.web_open}</Button>}<CodeBlockCopyButton value={diff.next} aria-label={gui.copy_text} /></CodeBlockActions>
    </CodeBlockHeader>
    {changed || full ? <CodeBlockContent className="max-h-64">
      {rows.map((row, n) => row.kind === "gap" ? <Button key={`gap-${row.at}`} data-diff-expand variant="ghost" size="sm" className="justify-start font-sans" onClick={() => setExpanded((now) => new Set([...now, row.at]))}>{gui.diff_unchanged(row.count)}</Button> :
        <DiffLine key={n} row={row} />)}
    </CodeBlockContent> : <CodeBlockFooter className="pt-in">{gui.chat_diff_no_changes}</CodeBlockFooter>}
    {hasGaps && <CodeBlockFooter><Button data-diff-expand variant="ghost" size="sm" onClick={() => { setFull(!full); setExpanded(new Set()); }}>{full ? gui.chat_diff_changes : gui.chat_diff_full}</Button></CodeBlockFooter>}
  </CodeBlock>;
}
