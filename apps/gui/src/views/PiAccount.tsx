import { useEffect, useRef, useState } from "react";
import { fetchDevices, khorAgent, openLink, said, type KhorAgentView } from "@/api";
import type { DeviceRow } from "@/gen/bindings/DeviceRow";
import type { KhorAgentAccount } from "@/gen/bindings/KhorAgentAccount";
import { gui } from "@/gen/catalog";
import { ChoiceField } from "@/components/ui/choice-field";
import { Input } from "@/components/ui/input";
import { Button } from "@/components/ui/button";
import { PopupButton } from "@/components/ui/popup-button";
import { Dialog, TaskOverlay, DialogHeader, DialogTitle, DialogFooter } from "@/components/ui/dialog";
import { MachineAvatar } from "@/components/Avatar";
import { managePi, piJob, type PiJob, type PiModel, type PiProvider } from "@/lib/pi-accounts";

const modelCache = new Map<string, PiModel[]>();
export function PiModelField({ account, value, onChange, enabled = true, label = gui.agent_model_session }: { account: KhorAgentAccount | undefined; value: string; onChange: (value: string) => void; enabled?: boolean; label?: string }) {
  const [models, setModels] = useState<PiModel[]>([]), [error, setError] = useState<string | null>(null), [busy, setBusy] = useState(false), [revision, setRevision] = useState(0);
  useEffect(() => {
    if (!enabled || !account) return;
    const controller = new AbortController();
    setModels(modelCache.get(account.id) ?? []); setBusy(true); setError(null);
    void piJob(account.home, { operation: "models", account: account.id }, controller.signal).then((state) => {
      if (controller.signal.aborted) return;
      const models = state.result?.models ?? []; if (models.length || !state.result?.trouble) { modelCache.set(account.id, models); setModels(models); } setError(state.result?.trouble ?? null);
    }, (error) => { if (!controller.signal.aborted) setError(said(error)); }).finally(() => { if (!controller.signal.aborted) setBusy(false); });
    return () => controller.abort();
  }, [account?.id, account?.home, enabled, revision]);
  return <div className="space-y-in" data-pi-model-field>
    <div data-khor-agent-model><ChoiceField label={label} value={value} selected={value} onInput={onChange} onPick={onChange} placeholder={gui.agent_model_search} disabled={!account}
      options={models.map((m) => ({ id: m.id, label: m.name, terms: m.id, aside: [m.reasoning ? gui.agent_reasoning_capability : "", m.image ? gui.agent_image_capability : ""].filter(Boolean).join(" · ") }))} /></div>
    <div className="flex items-baseline gap-in text-caption text-secondary-foreground"><span className="min-w-0 flex-1">{busy ? gui.agent_model_loading : error ?? gui.agent_model_manual}</span><Button size="sm" variant="ghost" onClick={() => setRevision((n) => n + 1)} disabled={busy || !account}>{gui.agent_model_refresh}</Button></div>
  </div>;
}

export function PiAccountForm({ account, onDone, onCancel }: { account?: KhorAgentAccount; onDone: (view: KhorAgentView, id: string) => void; onCancel: () => void }) {
  const [providers, setProviders] = useState<PiProvider[]>([]), [machines, setMachines] = useState<DeviceRow[]>([]);
  const [home, setHome] = useState(account?.home ?? ""), [providerId, setProviderId] = useState(account?.provider ?? ""), [name, setName] = useState(account?.name ?? "");
  const [base, setBase] = useState(account?.base_url ?? ""), [custom, setCustom] = useState(!!account?.base_url), [method, setMethod] = useState(account?.auth_type ?? "api_key");
  const [state, setState] = useState<PiJob | null>(null), [job, setJob] = useState(""), [answer, setAnswer] = useState("");
  const [error, setError] = useState<string | null>(null), [busy, setBusy] = useState(false), [responding, setResponding] = useState(false);
  const [ready, setReady] = useState<KhorAgentAccount | undefined>(account?.configured ? account : undefined), [model, setModel] = useState(account?.default_model ?? "");
  const operation = useRef<AbortController | null>(null);
  const target = useRef(account);
  useEffect(() => {
    let live = true;
    void Promise.all([khorAgent("catalog"), fetchDevices()]).then(([catalog, devices]) => {
      if (!live) return;
      setProviders((catalog.data as PiProvider[]).filter((p) => p.chat && p.methods.length));
      const computers = devices.filter((d) => d.hosts && d.sponsor === null); setMachines(computers);
      setHome((old) => old || computers.find((d) => d.me)?.id || computers[0]?.id || "");
    }, (error) => { if (live) setError(said(error)); });
    return () => { live = false; operation.current?.abort(); };
  }, []);
  useEffect(() => { setAnswer(""); }, [state?.ask]);
  const provider = providers.find((p) => p.id === providerId), machine = machines.find((d) => d.id === home);
  useEffect(() => { if (provider && !provider.methods.some((m) => m.id === method)) setMethod(provider.methods[0]?.id ?? "api_key"); }, [provider, method]);
  const selectProvider = (id: string) => { setProviderId(id); setMethod(providers.find((p) => p.id === id)?.methods[0]?.id ?? "api_key"); setBase(""); setCustom(id === "newapi"); setError(null); };
  const connect = async () => {
    if (!home || !provider || (providerId === "newapi" && !base.trim())) { setError(gui.agent_provider_missing); return; }
    const controller = new AbortController(); operation.current = controller; setBusy(true); setError(null);
    try {
      const result = await piJob(home, { operation: "login", account: target.current?.id, provider: providerId, name: name.trim() || provider.name, baseUrl: custom ? base.trim() : "", authType: method }, controller.signal, (state, id) => { setState(state); setJob(id); });
      const view = await khorAgent("read"), found = view.accounts.find((a) => a.id === result.account) ?? result.accountInfo;
      if (!found) throw new Error(gui.agent_auth_failed);
      target.current = found; setReady(found); setState(result); setModel(found.default_model);
    } catch (error) { if (!controller.signal.aborted) { setError(said(error)); setState(null); } }
    finally { if (!controller.signal.aborted) setBusy(false); }
  };
  const reply = async () => {
    if (!job || !state?.prompt || responding || (state.prompt.type !== "text" && !answer.trim())) return;
    setResponding(true); setError(null);
    try { await managePi(home, { operation: "reply", job, ask: state.ask, value: answer }); setState((s) => s ? { ...s, status: "running", prompt: undefined } : s); setAnswer(""); }
    catch (error) { setError(said(error)); }
    finally { setResponding(false); }
  };
  const finish = async () => {
    if (!ready || !model.trim() || busy) return;
    setBusy(true); setError(null);
    try { const result = await managePi(ready.home, { operation: "default", account: ready.id, model: model.trim() }); const view = await khorAgent("read"); const account = result.accountInfo ?? { ...ready, default_model: model.trim() }; onDone({ ...view, accounts: [...view.accounts.filter((a) => a.id !== account.id), account] }, ready.id); }
    catch (error) { setError(said(error)); }
    finally { setBusy(false); }
  };
  return <div className="space-y-group" data-pi-account-form>
    {!ready && !busy && <div className="space-y-row">
      <div className="space-y-in"><label className="text-callout">{gui.agent_credential_home}</label><ChoiceField label={gui.agent_credential_home} value={machine?.name ?? ""} selected={home} disabled={!!target.current} options={machines.map((m) => ({ id: m.id, label: m.name, icon: <MachineAvatar face={m.face} className="h-ctl-sm w-ctl-sm shrink-0" /> }))} onPick={setHome} /></div>
      <div className="space-y-in"><label className="text-callout">{gui.agent_provider}</label><ChoiceField label={gui.agent_provider} value={provider?.name ?? ""} selected={providerId} disabled={!!target.current} options={providers.map((p) => ({ id: p.id, label: p.id === "newapi" ? gui.agent_compatible_provider : p.name, terms: p.id }))} onPick={selectProvider} /></div>
      <label className="flex flex-col gap-in text-callout">{gui.agent_account_name}<Input value={name} onChange={(e) => setName(e.target.value)} placeholder={provider?.name} /></label>
      {provider && <div className="space-y-in"><span className="text-callout">{gui.add_auth}</span><PopupButton look="field" label={gui.add_auth} value={method} onPick={setMethod} choices={provider.methods.map((m) => ({ value: m.id, label: m.label }))} /></div>}
      {provider && (providerId === "newapi" || custom ? <label className="flex flex-col gap-in text-callout">{gui.agent_base_url}<Input value={base} onChange={(e) => setBase(e.target.value)} placeholder="https://" autoCapitalize="off" autoCorrect="off" spellCheck={false} /></label> : <Button variant="ghost" size="sm" onClick={() => setCustom(true)}>{gui.agent_base_override}</Button>)}
      <p className="text-callout text-secondary-foreground">{gui.agent_auth_local}</p>
    </div>}
    {!ready && busy && <div className="space-y-row" aria-busy={state?.status === "running"}>
      <p className="text-callout text-secondary-foreground">{gui.agent_auth_wait}</p>
      {(state?.events ?? []).map((event, index) => <div key={index} className="space-y-in text-callout">
        {(event.message || event.instructions) && <p className="whitespace-pre-wrap break-words">{event.message ?? event.instructions}</p>}
        {event.links?.map((link) => <Button key={link.url} variant="ghost" onClick={() => void openLink(link.url).catch((e) => setError(said(e)))}>{link.label ?? gui.agent_auth_open}</Button>)}
        {event.userCode && <p className="select-text font-mono text-read" data-selectable>{event.userCode}</p>}
        {(event.url || event.verificationUri) && <Button variant="secondary" onClick={() => void openLink(event.url ?? event.verificationUri!).catch((e) => setError(said(e)))}>{gui.agent_auth_open}</Button>}
      </div>)}
      {state?.prompt && <div className="space-y-in" key={state.ask}>
        <label className="text-callout" htmlFor="pi-auth-answer">{state.prompt.message}</label>
        {state.prompt.type === "select" ? <ChoiceField label={state.prompt.message} value={state.prompt.options?.find((o) => o.id === answer)?.label ?? ""} selected={answer} options={state.prompt.options ?? []} onPick={setAnswer} /> : <Input id="pi-auth-answer" data-pi-auth-answer type={state.prompt.type === "secret" ? "password" : "text"} autoComplete="off" autoCapitalize="off" autoCorrect="off" spellCheck={false} value={answer} placeholder={state.prompt.placeholder} onChange={(e) => setAnswer(e.target.value)} onKeyDown={(e) => { if (e.key === "Enter" && !e.nativeEvent.isComposing) { e.preventDefault(); void reply(); } }} />}
        <Button data-pi-auth-continue disabled={responding || (state.prompt.type !== "text" && !answer.trim())} onClick={() => void reply()}>{gui.agent_auth_continue}</Button>
      </div>}
    </div>}
    {ready && <div className="space-y-row"><div className="flex items-center gap-in"><p className="min-w-0 flex-1 text-callout text-secondary-foreground">{gui.agent_account_ready}</p><Button variant="ghost" size="sm" onClick={() => { setReady(undefined); setState(null); }}>{gui.agent_auth_retry}</Button></div><label className="text-callout">{gui.agent_default_model}</label><PiModelField account={ready} value={model} onChange={setModel} label={gui.agent_default_model} /></div>}
    {error && <p role="alert" className="whitespace-pre-wrap break-words text-callout text-state-failed">{error}</p>}
    <DialogFooter><Button variant="ghost" onClick={() => { operation.current?.abort(); onCancel(); }}>{gui.delete_keep}</Button>
      {ready ? <Button variant="primary" data-pi-save-default disabled={busy || !model.trim()} onClick={() => void finish()}>{gui.agent_model_save_default}</Button> : !busy && <Button variant="primary" disabled={!provider || !home} data-pi-connect onClick={() => void connect()}>{gui.agent_connect}</Button>}
    </DialogFooter>
  </div>;
}
export function PiAccountDialog({ open, onOpenChange, account, onDone, restoreFocus = true }: { open: boolean; onOpenChange: (open: boolean) => void; account?: KhorAgentAccount; onDone: (view: KhorAgentView, id: string) => void; restoreFocus?: boolean }) {
  const title = useRef<HTMLHeadingElement>(null);
  return <Dialog open={open} onOpenChange={onOpenChange}><TaskOverlay purpose="edit" scrollable data-pi-account-dialog onOpenAutoFocus={(e) => { e.preventDefault(); title.current?.focus(); }} onCloseAutoFocus={(e) => { if (!restoreFocus) e.preventDefault(); }}><DialogHeader><DialogTitle ref={title} tabIndex={-1}>{account ? gui.agent_model_account : gui.agent_account_add}</DialogTitle></DialogHeader>{open && <PiAccountForm key={account?.id ?? "new"} account={account} onCancel={() => onOpenChange(false)} onDone={(view, id) => { onDone(view, id); onOpenChange(false); }} />}</TaskOverlay></Dialog>;
}
