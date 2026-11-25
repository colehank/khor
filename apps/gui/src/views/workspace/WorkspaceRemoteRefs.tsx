import { useEffect, useRef, useState } from "react";
import { exploreGit, said, type GitExploreResult } from "@/api";
import { IconBranch, IconChevronRight } from "@/components/icons";
import { RegionState } from "@/components/RegionState";
import { Button } from "@/components/ui/button";
import { TaskActions } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { PopupButton } from "@/components/ui/popup-button";
import { gui } from "@/gen/catalog";
import { useGitResource, versionName, type GitLocation, type Version } from "./git-versions";

type RemoteResults = Extract<GitExploreResult, { kind: "remote_refs" }>;

export function WorkspaceRemoteRefs({ onVersion, ...location }: GitLocation & { onVersion: (version: Version) => void }) {
  const remotes = useGitResource(location, { kind: "remotes" }, "remotes");
  const [selected, setSelected] = useState("");
  const [reply, setReply] = useState<{ remote: string; at: Date; result: RemoteResults } | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [auth, setAuth] = useState(false);
  const [user, setUser] = useState("");
  const [token, setToken] = useState("");
  const flight = useRef<symbol | null>(null);
  useEffect(() => () => { flight.current = null; }, []);

  if (!remotes.value) return <RegionState state={remotes.error ? "error" : "loading-never"} message={remotes.error} onRetry={remotes.retry} />;
  const list = remotes.value.items;
  const remote = list.find(item => item.name === selected) ?? list[0];
  if (!remote) return <RegionState state="empty" message={gui.git_remote_none} />;
  const result = reply?.remote === remote.name ? reply : null;
  const query = async () => {
    if (flight.current) return;
    const stamp = Symbol();
    flight.current = stamp;
    setBusy(true);
    setError(null);
    const access = auth && remote.https && remote.host ? { host: remote.host, user: user.trim(), token } : null;
    setToken("");
    try {
      const result = await exploreGit(location.machine, location.root, { kind: "remote_refs", remote: remote.name, access }, "remote_refs");
      if (flight.current === stamp) setReply({ remote: remote.name, at: new Date(), result });
    } catch (failure) {
      if (flight.current === stamp) setError(said(failure));
    } finally {
      if (flight.current === stamp) {
        flight.current = null;
        setBusy(false);
      }
    }
  };
  return <div className="flex min-w-0 flex-col gap-row">
    <div className="flex min-w-0 flex-col gap-in">
      <span className="text-caption text-muted-foreground">{gui.git_remote_choice}</span>
      <PopupButton look="field" label={gui.git_remote_choice} value={remote.name}
        choices={list.map(item => ({ value: item.name, label: item.name, note: item.host ?? undefined }))}
        disabled={busy} onPick={name => { setSelected(name); setError(null); setAuth(false); setUser(""); setToken(""); }} />
      {remote.host && <span className="break-all text-caption text-muted-foreground">{remote.host}</span>}
    </div>
    {auth && <div className="flex min-w-0 flex-col gap-in">
      <Input aria-label={gui.git_remote_user} placeholder={gui.git_remote_user} value={user}
        autoComplete="off" disabled={busy} onChange={event => setUser(event.target.value)} />
      <Input aria-label={gui.git_remote_token} placeholder={gui.git_remote_token} value={token} type="password"
        autoComplete="off" disabled={busy} onChange={event => setToken(event.target.value)} />
      <p className="text-caption text-muted-foreground">{gui.git_remote_access_info}</p>
    </div>}
    <TaskActions primary={<Button data-git-remote-query aria-busy={busy}
      onClick={() => void query()} disabled={busy || (auth && (!user.trim() || !token))}>{gui.git_remote_query}</Button>} />
    {remote.https && <div className="flex justify-end">
      <Button variant="secondary" aria-pressed={auth} disabled={busy} onClick={() => { setAuth(was => !was); setToken(""); }}>{gui.git_remote_auth}</Button>
    </div>}
    {busy && <RegionState state="loading-never" />}
    {error && <p role="alert" className="text-callout text-muted-foreground">{error}</p>}
    {result && <div data-git-live-remotes className="flex flex-col gap-in">
      <p className="text-caption text-muted-foreground">{gui.git_remote_queried} <time dateTime={result.at.toISOString()}>{result.at.toLocaleTimeString()}</time></p>
      {result.result.refs.length === 0 && <p className="text-callout">{gui.git_remote_empty}</p>}
      <div className="divide-y divide-border">
        {result.result.refs.map(row => {
          const name = versionName(row.reference);
          return <button key={row.reference} type="button" disabled={!row.available}
            data-git-live-ref={row.reference}
            onClick={() => onVersion({ reference: `live:${remote.name}/${name}`, commit: row.commit })}
            className="flex min-h-touch w-full items-center gap-in rounded-sm px-in py-in text-start hover:bg-fill disabled:hover:bg-transparent">
            <IconBranch className="size-affordance flex-none text-muted-foreground" />
            <span className="min-w-0 flex-1">
              <span className="block break-all text-callout">{name}</span>
              <span className="block text-caption text-muted-foreground">{row.commit.slice(0, 7)}
                {!row.available && <> · {gui.git_remote_not_local}</>}</span>
            </span>
            {row.available && <IconChevronRight className="size-affordance flex-none text-muted-foreground" />}
          </button>;
        })}
      </div>
      {result.result.more && <p className="text-caption text-muted-foreground">{gui.git_branch_limited}</p>}
    </div>}
  </div>;
}
