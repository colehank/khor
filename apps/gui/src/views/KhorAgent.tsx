import { useState, useRef, useEffect } from "react";
import { khorAgent, said, type KhorAgentView } from "@/api";
import type { CommandRecord } from "@/gen/bindings/CommandRecord";
import { gui } from "@/gen/catalog";
import { Button } from "@/components/ui/button";
import { ChoiceField } from "@/components/ui/choice-field";
import { PiAccountDialog, PiModelField } from "@/views/PiAccount";
import { Dialog, TaskOverlay, TaskActions, DialogHeader, DialogTitle, DialogDescription, DialogBody } from "@/components/ui/dialog";
import { Markdown } from "@/components/Markdown";
import { Bubble, BubbleContent } from "@/components/ui/ai/bubble";
import { AgentConfirmation } from "@/components/ai-elements/agent";
import { AgentTranscript } from "@/components/ai-elements/content";
import { FilePreview } from "@/views/chat-files";
import {ChatView} from "@/views/ChatView";
import { StatusWord } from "@/components/StatusWord";

export function KhorAgentSettings({ view, onChanged, disabled: running, openRequest=0, hideTrigger=false, session="wide/all" }: { session?:string;view: KhorAgentView | null; onChanged: (view: KhorAgentView) => void; disabled: boolean; openRequest?:number; hideTrigger?:boolean }) {
  const [open, setOpen] = useState(false), [account, setAccount] = useState(""), [model, setModel] = useState("");
  const [saving, setSaving] = useState(false), [error, setError] = useState<string | null>(null);
  const [setup, setSetup] = useState(false), [editing, setEditing] = useState<string | undefined>();
  const title = useRef<HTMLHeadingElement>(null);
  const draftStart = useRef("");
  const conversation = (view?.data as {conversation?:{home?:string}} | null)?.conversation;
  const accounts = session === 'wide/all' ? view?.accounts ?? [] : view?.accounts.filter(a=>a.home===conversation?.home||a.home===view.config?.home) ?? [];
  const picked = accounts.find((a) => a.id === account);
  const choose = (value: boolean) => {
    if (value) {
      const picked = accounts.find((a) => a.id === view?.config?.account) ?? accounts[0];
      setAccount(picked?.id ?? ""); setModel(view?.config?.model ?? picked?.default_model ?? ""); setError(null);
      draftStart.current=JSON.stringify([picked?.id ?? "",view?.config?.model ?? picked?.default_model ?? ""]);
    }
    setOpen(value);
  };
  useEffect(()=>{if(openRequest)choose(true);},[openRequest]);
  const save = async () => {
    if (!picked || !model.trim() || saving) return;
    setSaving(true); setError(null);
    try { onChanged(await khorAgent("configure", picked.home, { account, model: model.trim(), session })); setOpen(false); }
    catch (e) { setError(said(e)); }
    finally { setSaving(false); }
  };
  const add = (id?: string) => { setEditing(id); setOpen(false); setSetup(true); };
  return <>
    <Dialog open={open} onOpenChange={choose}>
      {!hideTrigger&&<Button data-khor-agent-settings variant="ghost" size="sm" disabled={!view} onClick={() => choose(true)}>{view?.config?.model ?? gui.agent_setup}</Button>}
      <TaskOverlay purpose="edit" data-khor-agent-config onOpenAutoFocus={(e) => { e.preventDefault(); title.current?.focus(); }}>
        <DialogHeader><DialogTitle ref={title} tabIndex={-1}>{gui.agent_model_session}</DialogTitle><DialogDescription>{running ? gui.agent_model_next_turn : gui.agent_setup_hint}</DialogDescription></DialogHeader>
        <div className="space-y-row">
          <div className="space-y-in"><label className="text-callout">{gui.agent_account}</label><ChoiceField label={gui.agent_account} value={picked ? `${picked.name} · ${picked.home_name}` : ""} selected={account} disabled={saving}
            options={accounts.map((a) => ({ id: a.id, label: a.name, aside: a.home_name }))}
            onPick={(id) => { setAccount(id); setModel(view?.accounts.find((a) => a.id === id)?.default_model ?? ""); }}
            actions={[{ id: "add", label: gui.agent_account_add, run: () => add() }]} /></div>
          {picked && picked.configured === false ? <Button variant="secondary" data-pi-enable-account onClick={() => add(picked.id)}>{gui.agent_account_auth}</Button> : <PiModelField account={picked} value={model} onChange={setModel} enabled={open} />}
          {picked?.configured && <Button variant="ghost" size="sm" onClick={() => add(picked.id)}>{gui.agent_model_account}</Button>}
          {error && <p role="alert" className="text-callout text-state-failed">{error}</p>}
        </div>
        <TaskActions dirty={JSON.stringify([account,model]) !== draftStart.current} busy={saving} cancel={<Button variant="ghost" onClick={() => setOpen(false)} disabled={saving}>{gui.delete_keep}</Button>} primary={<Button data-khor-agent-save disabled={!account || !model.trim() || picked?.configured === false || saving} onClick={() => void save()}>{gui.agent_save_and_use}</Button>}/>
      </TaskOverlay>
    </Dialog>
    <PiAccountDialog restoreFocus={false} open={setup} onOpenChange={(value) => { setSetup(value); if (!value) setOpen(true); }} account={view?.accounts.find((a) => a.id === editing)} onDone={(next, id) => {
      onChanged(session==='wide/all'?next:{...next,config:view?.config??null,data:view?.data??null}); const account = next.accounts.find((a) => a.id === id); setAccount(id); setModel(account?.default_model ?? ""); setOpen(true);
    }} />
  </>;
}

export function KhorAgentResult({ record: c, session="wide/all" }: { session?:string;record: CommandRecord }) {
  const [error, setError] = useState<string | null>(null), [acting, setActing] = useState(false);
  const [file, setFile] = useState<string | null>(null);
  const [full,setFull]=useState<{id:string;binding:string}|null>(null),[fullBusy,setFullBusy]=useState(false);
  const mounted=useRef(true);
  useEffect(()=>{mounted.current=true;return()=>{mounted.current=false;};},[]);
  const readFull=async()=>{
    if(fullBusy)return;setFullBusy(true);setError(null);
    try{const answer=await khorAgent('history',c.owner,{run:c.id,session});
      const source=answer.data as {id?:unknown;binding?:unknown}|null;
      if(!source||typeof source.id!=='string'||typeof source.binding!=='string')throw new Error('完整记录不可用');
      if(mounted.current)setFull({id:source.id,binding:source.binding});
    }catch(failure){if(mounted.current)setError(said(failure));}finally{if(mounted.current)setFullBusy(false);}
  };
  const busy = c.status === "running" || c.status === "blocked", ask = c.agent?.approval;
  const action = async (name: string, extra: Record<string, unknown> = {}) => {
    if (acting) return;
    setActing(true); setError(null);
    try { await khorAgent(name, c.owner, { run: c.id, ...extra, session }); }
    catch (e) { setError(said(e)); }
    finally { setActing(false); }
  };
  const word = ({ running: "busy", blocked: "blocked", done: "done", failed: "failed", interrupted: "errored", cancelled: "idle" } as Record<string, string>)[c.status] ?? "idle";
  return <article data-khor-agent-turn={c.id} data-khor-agent-status={c.status} aria-busy={busy} className="min-w-0 space-y-row text-read">
    <Bubble align="end" variant="muted"><BubbleContent data-khor-agent-prompt className="text-read"><Markdown text={c.line} /></BubbleContent></Bubble>
    <div className="flex items-baseline gap-in text-caption text-secondary-foreground"><span>khor</span><span className="min-w-0 flex-1 truncate">{c.agent?.model} · {c.owner_name}</span><StatusWord word={word} />{busy && <Button data-khor-agent-stop variant="ghost" size="sm" disabled={acting} onClick={() => void action("stop")}>{gui.page_stop}</Button>}</div>
    <AgentTranscript updates={c.agent?.updates ?? []} busy={busy} fallback={c.stdout} onOpenFile={setFile} />
    <div className="flex items-center gap-in text-caption">{c.agent?.updates_truncated&&<span className="text-muted-foreground">{gui.agent_detail_cut}</span>}<Button size="sm" variant="ghost" disabled={fullBusy} onClick={()=>void readFull()}>查看完整内容</Button></div>
    <Dialog open={full!==null} onOpenChange={open=>{if(!open)setFull(null);}}><TaskOverlay purpose="preview" scrollable>
      <DialogHeader><DialogTitle>完整内容</DialogTitle></DialogHeader>
      <DialogBody className="flex min-h-0 flex-col">{full&&<ChatView id={full.id} still binding={full.binding} home={c.owner} machine={c.owner_name} category="khor" watch={busy}/>}</DialogBody>
    </TaskOverlay></Dialog>
    {c.stderr && <p className="text-callout text-secondary-foreground">{c.stderr}</p>}
    {ask && <div data-khor-agent-approval><AgentConfirmation key={ask.id} title={`${gui.agent_approve_hint}\n${ask.tool}\n${ask.arguments}`}
      options={[["allow", gui.approve], ["deny", gui.agent_deny]]} answer={null} canAnswer={c.status === "blocked"}
      onAnswer={(option) => khorAgent("answer", c.owner, { run: c.id, ask: ask.id, allow: option === "allow", session })} /></div>}
    {(c.error || error) && <p role="alert" className="whitespace-pre-wrap break-all text-callout text-state-failed">{error || c.error}</p>}
    {file !== null && <FilePreview machine={c.owner} shown={c.owner_name} here={false} path={file} onClose={() => setFile(null)} />}
  </article>;
}
