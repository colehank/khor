import { useState } from "react";
import { khorAgent, said, type KhorAgentView } from "@/api";
import type { CommandRecord } from "@/gen/bindings/CommandRecord";
import { gui } from "@/gen/catalog";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { PopupButton } from "@/components/ui/popup-button";
import { Dialog, TaskOverlay, DialogHeader, DialogTitle, DialogDescription, DialogFooter } from "@/components/ui/dialog";
import { Markdown } from "@/components/Markdown";
import { StatusWord } from "@/components/StatusWord";

export function KhorAgentSettings({ view, onChanged, disabled }: { view: KhorAgentView | null; onChanged: (view: KhorAgentView) => void; disabled: boolean }) {
  const [open, setOpen] = useState(false), [account, setAccount] = useState(""), [model, setModel] = useState("");
  const [saving, setSaving] = useState(false), [error, setError] = useState<string | null>(null);
  const choose = (value: boolean) => {
    if (value) { setAccount(view?.config?.account ?? view?.accounts[0]?.id ?? ""); setModel(view?.config?.model ?? ""); setError(null); }
    setOpen(value);
  };
  const save = async () => {
    const picked = view?.accounts.find((a) => a.id === account);
    if (!picked || !model.trim() || saving) return;
    setSaving(true); setError(null);
    try { onChanged(await khorAgent("configure", picked.home, { account, model: model.trim() })); setOpen(false); }
    catch (e) { setError(said(e)); }
    finally { setSaving(false); }
  };
  return <Dialog open={open} onOpenChange={choose}>
    <Button data-khor-agent-settings variant="ghost" size="sm" disabled={disabled || !view} onClick={() => choose(true)}>{view?.config?.model ?? gui.agent_setup}</Button>
    <TaskOverlay purpose="edit" data-khor-agent-config>
      <DialogHeader><DialogTitle>{gui.agent_setup}</DialogTitle><DialogDescription>{gui.agent_setup_hint}</DialogDescription></DialogHeader>
      <div className="space-y-row">
        {!view?.accounts.length ? <p className="text-callout text-secondary-foreground">{gui.agent_add_key}</p> : <>
          <div className="space-y-in"><label className="text-callout">{gui.agent_account}</label><PopupButton label={gui.agent_account} value={account} onPick={setAccount} look="field" disabled={saving} choices={view.accounts.map((a) => ({ value: a.id, label: `${a.name} · ${a.home_name}` }))} mark={{ "data-khor-agent-account": "true" }} /></div>
          <label className="flex flex-col gap-in text-callout">{gui.agent_model}<Input data-khor-agent-model value={model} onChange={(e) => setModel(e.target.value)} disabled={saving} autoCapitalize="off" autoCorrect="off" spellCheck={false} onKeyDown={(e) => { if (e.key === "Enter" && !e.nativeEvent.isComposing) { e.preventDefault(); void save(); } }} /></label>
        </>}
        {error && <p role="alert" className="text-callout text-state-failed">{error}</p>}
      </div>
      <DialogFooter><Button variant="ghost" onClick={() => setOpen(false)} disabled={saving}>{gui.delete_keep}</Button><Button data-khor-agent-save disabled={!account || !model.trim() || saving} onClick={() => void save()}>{gui.watch_save}</Button></DialogFooter>
    </TaskOverlay>
  </Dialog>;
}

export function KhorAgentResult({ record: c }: { record: CommandRecord }) {
  const [error, setError] = useState<string | null>(null), [acting, setActing] = useState(false);
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
    <div className="flex justify-end"><div data-khor-agent-prompt className="max-w-[85%] whitespace-pre-wrap break-all rounded-lg bg-accent px-3 py-1.5 text-read-ink">{c.line}</div></div>
    <div className="flex items-baseline gap-in text-caption text-secondary-foreground"><span>khor</span><span className="min-w-0 flex-1 truncate">{c.agent?.model} · {c.owner_name}</span><StatusWord word={word} />{busy && <Button data-khor-agent-stop variant="ghost" size="sm" disabled={acting} onClick={() => void action("stop")}>{gui.page_stop}</Button>}</div>
    {c.stdout && <Markdown text={c.stdout} />}
    {c.stderr && <p className="text-callout text-secondary-foreground">{c.stderr}</p>}
    {ask && <div data-khor-agent-approval className="space-y-in border-t pt-in">
      <div className="text-callout">{gui.agent_approve_hint}</div><code className="text-callout">{ask.tool}</code><pre className="overflow-x-auto whitespace-pre-wrap break-all text-callout">{ask.arguments}</pre>
      <div className="flex gap-in"><Button data-khor-agent-allow size="sm" disabled={acting} onClick={() => void action("answer", { ask: ask.id, allow: true })}>{gui.approve}</Button><Button data-khor-agent-deny variant="ghost" size="sm" disabled={acting} onClick={() => void action("answer", { ask: ask.id, allow: false })}>{gui.agent_deny}</Button></div>
    </div>}
    {(c.error || error) && <p role="alert" className="whitespace-pre-wrap break-all text-callout text-state-failed">{error || c.error}</p>}
  </article>;
}
