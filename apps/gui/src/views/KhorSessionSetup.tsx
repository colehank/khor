import {useEffect, useState} from 'react';
import {khorAgent, said, type KhorAgentView} from '@/api';
import {gui} from '@/gen/catalog';
import {Button} from '@/components/ui/button';
import {ChoiceField} from '@/components/ui/choice-field';
import {FormLabel} from '@/components/ui/form-grid';
import {PiAccountDialog, PiModelField} from '@/views/PiAccount';

export type KhorModelSelection = {home: string; account: string; model: string};

export function KhorSessionSetup({machine, disabled, onChange}: {
  machine: string;
  disabled: boolean;
  onChange: (selection: KhorModelSelection | null) => void;
}) {
  const [view, setView] = useState<KhorAgentView | null>(null);
  const [account, setAccount] = useState(''), [model, setModel] = useState('');
  const [error, setError] = useState<string | null>(null), [loading, setLoading] = useState(true);
  const [setup, setSetup] = useState(false), [revision, setRevision] = useState(0);
  const accounts = view?.accounts.filter(a => a.home === machine && a.configured) ?? [];
  const picked = accounts.find(a => a.id === account);

  useEffect(() => {
    let live = true;
    setLoading(true); setError(null); setView(null); setAccount(''); setModel(''); onChange(null);
    void khorAgent('read').then(next => {
      if (!live) return;
      const available = next.accounts.filter(a => a.home === machine && a.configured);
      const preferred = next.config?.home === machine ? next.config : null;
      const chosen = available.find(a => a.id === preferred?.account) ?? available[0];
      const model = preferred && chosen?.id === preferred.account ? preferred.model : chosen?.default_model ?? '';
      setView(next); setAccount(chosen?.id ?? ''); setModel(model);
      onChange(chosen && model.trim() ? {home: machine, account: chosen.id, model: model.trim()} : null);
    }, error => {if (live) setError(said(error));}).finally(() => {if (live) setLoading(false);});
    return () => {live = false;};
  }, [machine, revision, onChange]);

  const select = (id: string, model: string) => {
    setAccount(id); setModel(model);
    onChange(accounts.some(a => a.id === id) && model.trim() ? {home: machine, account: id, model: model.trim()} : null);
  };

  return <>
    <FormLabel>{gui.agent_account}</FormLabel>
    <div data-new-khor-account className="min-w-0 space-y-in">
      <ChoiceField label={gui.agent_account} value={picked?.name ?? ''} selected={account} disabled={disabled || loading}
        options={accounts.map(a => ({id: a.id, label: a.name, aside: a.provider}))}
        onPick={id => select(id, accounts.find(a => a.id === id)?.default_model ?? '')}
        actions={[{id: 'add', label: gui.agent_account_add, run: () => setSetup(true)}]} />
      {!loading && !accounts.length && !error && <p className="text-caption text-secondary-foreground">{gui.agent_provider_missing}</p>}
      {error && <div className="flex items-baseline gap-in"><p role="alert" className="min-w-0 flex-1 text-caption text-destructive">{error}</p><Button variant="ghost" size="sm" onClick={() => setRevision(n => n + 1)}>{gui.region_retry}</Button></div>}
    </div>
    <FormLabel>{gui.agent_model_session}</FormLabel>
    <div data-new-khor-model className="min-w-0"><PiModelField account={picked} value={model} enabled={!disabled} onChange={model => select(account, model)} disabled={disabled} /></div>
    <PiAccountDialog home={machine} open={setup} onOpenChange={setSetup} onDone={(next, id) => {
      const added = next.accounts.find(a => a.id === id && a.home === machine && a.configured);
      setView(next); setAccount(added?.id ?? ''); setModel(added?.default_model ?? '');
      onChange(added?.default_model.trim() ? {home: machine, account: added.id, model: added.default_model.trim()} : null);
    }} />
  </>;
}
