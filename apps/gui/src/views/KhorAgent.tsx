import { useState, useRef } from "react";
import { khorAgent, said, type KhorAgentView } from "@/api";
import type { CommandRecord } from "@/gen/bindings/CommandRecord";
import { gui } from "@/gen/catalog";
import { Button } from "@/components/ui/button";
import { ChoiceField } from "@/components/ui/choice-field";
import { PiAccountDialog, PiModelField } from "@/views/PiAccount";
import { Dialog, TaskOverlay, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { Markdown } from "@/components/Markdown";
import { Bubble, BubbleContent } from "@/components/ui/ai/bubble";
import { AgentConfirmation } from "@/components/ai-elements/agent";
import { AgentTranscript } from "@/components/ai-elements/content";
import { FilePreview } from "@/views/chat-files";
import { StatusWord } from "@/components/StatusWord";

export function KhorAgentSettings({ view, onChanged, disabled: running }: { view: KhorAgentView | null; onChanged: (view: KhorAgentView) => void; disabled: boolean }) {
  const [open, setOpen] = useState(false), [account, setAccount] = useState(""), [model, setModel] = useState("");
  const [saving, setSaving] = useState(false), [error, setError] = useState<string | null>(null);
  const [setup, setSetup] = useState(false), [editing, setEditing] = useState<string | undefined>();
  const title = useRef<HTMLHeadingElement>(null);
  const picked = view?.accounts.find((a) => a.id === account);
  const choose = (value: boolean) => {
    if (value) {
      const picked = view?.accounts.find((a) => a.id === view.config?.account) ?? view?.accounts[0];
      setAccount(picked?.id ?? ""); setModel(view?.config?.model ?? picked?.default_model ?? ""); setError(null);
    }
    setOpen(value);
  };
  const save = async () => {
    if (!picked || !model.trim() || saving) return;
    setSaving(true); setError(null);
    try { onChanged(await khorAgent("configure", picked.home, { account, model: model.trim() })); setOpen(false); }
    catch (e) { setError(said(e)); }
    finally { setSaving(false); }
  };
  const add = (id?: string) => { setEditing(id); setOpen(false); setSetup(true); };
  return <>
    <Dialog open={open} onOpenChange={choose}>
      <Button data-khor-agent-settings variant="ghost" size="sm" disabled={!view} onClick={() => choose(true)}>{view?.config?.model ?? gui.agent_setup}</Button>
      <TaskOverlay purpose="edit" data-khor-agent-config onOpenAutoFocus={(e) => { e.preventDefault(); title.current?.focus(); }}>
        <DialogHeader><DialogTitle ref={title} tabIndex={-1}>{gui.agent_model_session}</DialogTitle><DialogDescription>{running ? gui.agent_model_next_turn : gui.agent_setup_hint}</DialogDescription></DialogHeader>
        <div className="space-y-row">
          <div className="space-y-in"><label className="text-callout">{gui.agent_account}</label><ChoiceField label={gui.agent_account} value={picked ? `${picked.name} · ${picked.home_name}` : ""} selected={account} disabled={saving}
            options={(view?.accounts ?? []).map((a) => ({ id: a.id, label: a.name, aside: a.home_name }))}
            onPick={(id) => { setAccount(id); setModel(view?.accounts.find((a) => a.id === id)?.default_model ?? ""); }}
            actions={[{ id: "add", label: gui.agent_account_add, run: () => add() }]} /></div>
          {picked && picked.configured === false ? <Button variant="secondary" data-pi-enable-account onClick={() => add(picked.id)}>{gui.agent_account_auth}</Button> : <PiModelField account={picked} value={model} onChange={setModel} enabled={open} />}
          {picked?.configured && <Button variant="ghost" size="sm" onClick={() => add(picked.id)}>{gui.agent_model_account}</Button>}
          {error && <p role="alert" className="text-callout text-state-failed">{error}</p>}
        </div>
        <DialogFooter><Button variant="ghost" onClick={() => setOpen(false)} disabled={saving}>{gui.delete_keep}</Button><Button data-khor-agent-save disabled={!account || !model.trim() || picked?.configured === false || saving} onClick={() => void save()}>{gui.agent_save_and_use}</Button></DialogFooter>
      </TaskOverlay>
    </Dialog>
    <PiAccountDialog restoreFocus={false} open={setup} onOpenChange={(value) => { setSetup(value); if (!value) setOpen(true); }} account={view?.accounts.find((a) => a.id === editing)} onDone={(next, id) => {
      onChanged(next); const account = next.accounts.find((a) => a.id === id); setAccount(id); setModel(account?.default_model ?? ""); setOpen(true);
    }} />
  </>;
}

export function KhorAgentResult({ record: c }: { record: CommandRecord }) {
  const [error, setError] = useState<string | null>(null), [acting, setActing] = useState(false);
  const [file, setFile] = useState<string | null>(null);
  const busy = c.status === "running" || c.status === "blocked", ask = c.agent?.approval;
  const action = async (name: string, extra: Record<string, unknown> = {}) => {
    if (acting) return;
    setActing(true); setError(null);
    try { await khorAgent(name, c.owner, { run: c.id, ...extra }); }
    catch (e) { setError(said(e)); }
    finally { setActing(false); }
  };
  const word = ({ running: "busy", blocked: "blocked", done: "done", failed: "failed", interrupted: "errored", cancelled: "idle" } as Record<string, string>)[c.status] ?? "idle";
  return <article data-khor-agent-turn={c.id} data-khor-agent-status={c.status} aria-busy={busy} className="min-w-0 space-y-row text-read">
    <Bubble align="end" variant="muted"><BubbleContent data-khor-agent-prompt className="text-read"><Markdown text={c.line} /></BubbleContent></Bubble>
    <div className="flex items-baseline gap-in text-caption text-secondary-foreground"><span>khor</span><span className="min-w-0 flex-1 truncate">{c.agent?.model} · {c.owner_name}</span><StatusWord word={word} />{busy && <Button data-khor-agent-stop variant="ghost" size="sm" disabled={acting} onClick={() => void action("stop")}>{gui.page_stop}</Button>}</div>
    <AgentTranscript updates={c.agent?.updates ?? []} busy={busy} fallback={c.stdout} onOpenFile={setFile} />
    {c.agent?.updates_truncated && <p className="text-caption text-muted-foreground">{gui.agent_detail_cut}</p>}
    {c.stderr && <p className="text-callout text-secondary-foreground">{c.stderr}</p>}
    {ask && <div data-khor-agent-approval><AgentConfirmation key={ask.id} title={`${gui.agent_approve_hint}\n${ask.tool}\n${ask.arguments}`}
      options={[["allow", gui.approve], ["deny", gui.agent_deny]]} answer={null} canAnswer={c.status === "blocked"}
      onAnswer={(option) => khorAgent("answer", c.owner, { run: c.id, ask: ask.id, allow: option === "allow" })} /></div>}
    {(c.error || error) && <p role="alert" className="whitespace-pre-wrap break-all text-callout text-state-failed">{error || c.error}</p>}
    {file !== null && <FilePreview machine={c.owner} shown={c.owner_name} here={false} path={file} onClose={() => setFile(null)} />}
  </article>;
}
