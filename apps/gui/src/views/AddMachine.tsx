// 添加机器 (round join-d): the one entry under ＋ that brings another
// machine in, drawn as the owner approved it on the lab (redesign-join
// cb5dbb54, `lab/join.tsx`). Two ways in:
//
// - **khor 主机** — it already runs khor: this machine's invitation code
//   (QR and text), and a field for a code made elsewhere. Pairing has no
//   direction (docs/NET.md), so both ends of the one act sit on one page.
// - **SSH 主机** — over ssh, installing khor there unless the switch is
//   off (`khor add`; the node falls back to an SSH 主机 on its own when
//   khor cannot stay, and removes what it put there).
//
// Every page is a form dialog, so on a phone it is the bottom sheet.
// Forms are lists on the one ground with hairlines — a row per answer,
// its value at the trailing end, a chevron where a choice opens — and a
// choice is a page of its own inside the sheet with a check on what is
// picked, on the wide face too (the lead's recommendation the owner
// took: no popup menus in this dialog).
import { useEffect, useRef, useState, type ReactNode } from "react";

import {
  addRead,
  addStart,
  fetchLs,
  fetchScannerHere,
  fetchSshConfigHosts,
  invite,
  openCameraSettings,
  openCopyId,
  pair,
  pickKeyFile,
  promoteStart,
  proxyLine,
  reachProbe,
  said,
  scanTicket,
  type Ticket,
} from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import { IconCheck, IconChevronRight, IconClose, IconDevices, IconSshHost } from "@/components/icons";
import { Switch } from "@/components/settings/Switch";
import { Button } from "@/components/ui/button";
import { Dialog, TaskOverlay, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import type { AddRun } from "@/gen/bindings/AddRun";
import type { AddStep } from "@/gen/bindings/AddStep";
import type { DeviceRow } from "@/gen/bindings/DeviceRow";
import type { ProxyLine } from "@/gen/bindings/ProxyLine";
import type { ReachReport } from "@/gen/bindings/ReachReport";
import { cli, gui, msg } from "@/gen/catalog";
import { feel } from "@/lib/haptics";
import { keptGet, keptSet, useKept } from "@/lib/kept";
import { YIELD_MS } from "@/lib/tempo";
import { cn } from "@/lib/utils";
import { ScanLayer } from "@/views/PairDialogs";
import { latency } from "@/words";

/** The pages of the sheet. `run` is an add (or 安装 khor) under way. */
type Page = "pick" | "ticket" | "ssh" | "jump" | "auth" | "proxy" | "reach" | "run" | "login" | "passphrase";

/** The SSH form's answers, kept above the pages so a page of choices and
    the form it returns to share them. */
type Form = {
  host: string;
  port: string;
  /** "" = 直接连接; "khor:<name>" = run the add there; else an ssh jump. */
  via: string;
  auth: "key" | "password";
  /** null = 默认 (ssh config and agent); else the key file's path. */
  key: string | null;
  install: boolean;
};
const blank: Form = { host: "", port: "", via: "", auth: "key", key: null, install: true };

const viaKhor = (via: string) => (via.startsWith("khor:") ? via.slice(5) : null);
const keyName = (path: string) => path.split("/").pop() ?? path;
const portOf = (f: Form) => (f.port ? Number(f.port) : null);

/** How long the ssh config's hosts stay after the host field lets go
    (`HostRow`): longer than a press, shorter than a glance. */
const SUGGEST_LINGER_MS = 250;

/** How often a running add is read (`add_read`). */
const READ_EVERY_MS = 500;

/** Where the add as it stands is kept across the dialog's remount
    (`lib/kept`, round join-e): the shell draws its dialogs in both its
    trees, so a window crossing the narrow breakpoint mounts a new one.
    `up` says the dialog was open, so the new one carries on instead of
    starting over; `token` is the run to read again. Never the secret. */
const KEPT = {
  up: "add-machine:up",
  page: "add-machine:page",
  form: "add-machine:form",
  run: "add-machine:run",
  runOf: "add-machine:run-of",
  error: "add-machine:error",
  token: "add-machine:token",
} as const;

// ---- rows -------------------------------------------------------------------

/**
 * A list on the one ground: rows with a hairline between them that starts
 * where the words start, an optional group name above in `group` /
 * `tertiary`, an optional note below in `callout` / `secondary`. No card,
 * no fill: the sheet's ground and its lines.
 */
function Group({ head, foot, children, mark }: { head?: string; foot?: ReactNode; children: ReactNode; mark?: string }) {
  return (
    <section data-add-group={mark} className="flex flex-col gap-xxs">
      {head && <h3 className="text-group text-tertiary">{head}</h3>}
      <div className="-mx-row flex flex-col [&>*:first-child_[data-row-body]]:border-t-0">{children}</div>
      {foot && (
        <div data-add-group-foot className="text-callout text-muted-foreground">
          {foot}
        </div>
      )}
    </section>
  );
}

/**
 * One row: a leading seat (a glyph, a face, a step's mark), the title and
 * its note, a value in `secondary`, then the trailing control — a check on
 * the picked choice, a chevron where a choice opens, a switch.
 */
function Row({
  lead,
  title,
  note,
  value,
  trail,
  chevron,
  checked,
  onPress,
  dim,
  mark,
  children,
}: {
  lead?: ReactNode;
  title?: ReactNode;
  note?: ReactNode;
  value?: ReactNode;
  trail?: ReactNode;
  chevron?: boolean;
  checked?: boolean;
  onPress?: () => void;
  /** A row that cannot be picked: words in `secondary`, no press. */
  dim?: boolean;
  mark?: string;
  /** In place of title / note / value: the row's own body (a field). */
  children?: ReactNode;
}) {
  const body = (
    <>
      {lead && <span className="flex flex-none items-center justify-center self-center">{lead}</span>}
      <span
        data-row-body
        className="flex min-h-[max(var(--touch),calc(var(--ctl-md)+2*var(--gap-in)))] min-w-0 flex-1 items-center gap-row border-t py-in"
      >
        {children ?? (
          <>
            <span className="flex min-w-0 flex-1 flex-col">
              <span className={cn("text-body", dim ? "text-muted-foreground" : "text-foreground")}>{title}</span>
              {note && <span className="text-callout text-muted-foreground">{note}</span>}
            </span>
            {value !== undefined && (
              <span className="min-w-0 flex-none truncate text-body text-muted-foreground tabular-nums">{value}</span>
            )}
          </>
        )}
        {trail}
        {checked !== undefined && (
          <span className="flex w-icon-mark flex-none justify-center">
            {checked && <IconCheck className="size-icon-mark text-foreground" />}
          </span>
        )}
        {chevron && <IconChevronRight className="size-affordance flex-none text-tertiary" />}
      </span>
    </>
  );
  const cls = "flex w-full items-stretch gap-row rounded-md px-row text-left";
  return onPress && !dim ? (
    <button type="button" data-add-row={mark} onClick={onPress} className={cn(cls, "hover:bg-fill")}>
      {body}
    </button>
  ) : (
    <div data-add-row={mark} className={cls}>
      {body}
    </div>
  );
}

/** A field that is its row: no fill of its own, the row's lines around it. */
const bare = "h-auto min-h-0 rounded-none bg-transparent px-0";

/** A labelled field row (主机's port aside): the label leading, the field
    filling the rest, its text at the trailing edge. */
function FieldRow({
  id,
  label,
  value,
  onChange,
  type = "text",
  autoFocus,
  onEnter,
}: {
  id: string;
  label: string;
  value: string;
  onChange: (v: string) => void;
  type?: string;
  autoFocus?: boolean;
  onEnter?: () => void;
}) {
  return (
    <Row mark={id}>
      <label htmlFor={id} className="flex-none text-body text-foreground">
        {label}
      </label>
      <Input
        id={id}
        type={type}
        autoFocus={autoFocus}
        autoComplete="off"
        spellCheck={false}
        value={value}
        onChange={(e) => onChange(e.target.value)}
        onKeyDown={(e) => e.key === "Enter" && onEnter?.()}
        className={cn(bare, "flex-1 text-right")}
      />
    </Row>
  );
}

function Head({ title }: { title: string }) {
  return (
    <DialogHeader>
      <DialogTitle>{title}</DialogTitle>
    </DialogHeader>
  );
}

/** The sheet's parts, `gap-group` apart. */
function Body({ children }: { children: ReactNode }) {
  return <div className="flex min-w-0 flex-col gap-group">{children}</div>;
}

function Face({ row, className }: { row?: DeviceRow; className?: string }) {
  return <MachineAvatar face={row?.face ?? null} className={cn("size-ctl-md flex-none", className)} />;
}

function Failure({ text }: { text: string | null }) {
  return text ? (
    <div data-dialog-error className="text-callout text-destructive">
      {text}
    </div>
  ) : null;
}

/** A copy button that says for a beat what the copy did. */
function CopyButton({ text, label }: { text: string; label: string }) {
  const [copied, setCopied] = useState<"copied" | "failed" | null>(null);
  useEffect(() => {
    if (!copied) return;
    const t = window.setTimeout(() => setCopied(null), YIELD_MS);
    return () => window.clearTimeout(t);
  }, [copied]);
  return (
    <Button
      className="flex-none"
      data-said={copied ?? undefined}
      onClick={() =>
        navigator.clipboard.writeText(text).then(
          () => setCopied("copied"),
          () => setCopied("failed"),
        )
      }
    >
      {copied === "copied" ? gui.copied : copied === "failed" ? gui.copy_failed : label}
    </Button>
  );
}

// ---- the dialog ---------------------------------------------------------------

export function AddMachineDialog({
  open,
  onOpenChange,
  machines,
  start,
  promote,
  onAdded,
  onOpenSession,
}: {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  /** The machine table as the lists read it. */
  machines: DeviceRow[];
  /** The page to open on: `ticket` for the alone note's press. */
  start?: "pick" | "ticket";
  /** 安装 khor on this SSH 主机 (the machine page's act) instead of an add. */
  promote?: string | null;
  /** A machine came in or changed: the lists read the table again. */
  onAdded?: () => void;
  /** 上传公钥 opened a terminal session: show it. */
  onOpenSession?: (id: string) => void;
}) {
  const [page, setPage] = useKept<Page>(KEPT.page, "pick");
  const [form, setForm] = useKept<Form>(KEPT.form, blank);
  const set = (p: Partial<Form>) => setForm((f) => ({ ...f, ...p }));
  /** The password typed on the form or the login page, or the key's
      passphrase. Held only until the add starts, and never kept. */
  const [secret, setSecret] = useState("");
  const [run, setRun] = useKept<AddRun | null>(KEPT.run, null);
  /** What `run` is: an add of `form`, or 安装 khor on a machine. */
  const [runOf, setRunOf] = useKept<{ host: string; promote: boolean } | null>(KEPT.runOf, null);
  const [error, setError] = useKept<string | null>(KEPT.error, null);
  const [hosts, setHosts] = useState<string[]>([]);
  /** The run's token. Made here, so pressing 添加 twice starts nothing
      new; kept, so a remounted dialog reads the same run. */
  const token = useRef<string | null>(keptGet<string | null>(KEPT.token) ?? null);

  useEffect(() => {
    if (!open) {
      keptSet(KEPT.up, false);
      return;
    }
    let live = true;
    fetchSshConfigHosts()
      .then((h) => live && setHosts(h))
      .catch(() => live && setHosts([]));
    // Mounted again while open (the window crossed the narrow
    // breakpoint): the kept page, form and run carry on, and the run's
    // effect below reads the kept token again. Nothing starts twice.
    if (keptGet<boolean>(KEPT.up)) {
      return () => {
        live = false;
      };
    }
    keptSet(KEPT.up, true);
    keptSet(KEPT.token, null);
    token.current = null;
    setForm(blank);
    setSecret("");
    setRun(null);
    setRunOf(null);
    setError(null);
    setPage(promote ? "run" : (start ?? "pick"));
    if (promote) begin(promote, true);
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [open]);

  // **The add under way**: read every half second until it ends. A run
  // that had already ended when the dialog remounted is not read again
  // (its end was felt once).
  useEffect(() => {
    if (page !== "run" || !token.current || run?.done) return;
    const t = token.current;
    let live = true;
    let timer = 0;
    const read = () =>
      addRead(t)
        .then((r) => {
          if (!live) return;
          setRun(r);
          if (!r.done) {
            timer = window.setTimeout(read, READ_EVERY_MS);
            return;
          }
          const end = r.done;
          if (end.id) {
            feel("success");
            onAdded?.();
          } else feel("warning");
          // A refused login goes on to the page that offers the way on.
          if (end.refusal === "key" || end.refusal === "password") {
            setPage("login");
            setError(end.refusal === "password" ? end.error : null);
          } else if (end.refusal === "locked" || end.refusal === "passphrase") {
            setPage("passphrase");
            setError(end.refusal === "passphrase" ? end.error : null);
          }
        })
        .catch((e) => live && setError(said(e)));
    read();
    return () => {
      live = false;
      window.clearTimeout(timer);
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [page, runOf]);

  const begin = (what: string, isPromote: boolean, auth?: { password?: string; passphrase?: string }) => {
    const t = crypto.randomUUID();
    token.current = t;
    keptSet(KEPT.token, t);
    setRun(null);
    setError(null);
    setRunOf({ host: what, promote: isPromote });
    setPage("run");
    const go = isPromote
      ? promoteStart(t, what)
      : addStart(t, {
          host: form.host.trim(),
          port: portOf(form),
          jump: form.via && !viaKhor(form.via) ? form.via : null,
          on: viaKhor(form.via),
          identity: form.auth === "key" ? form.key : null,
          secret: auth?.password ?? auth?.passphrase ?? (form.auth === "password" && secret ? secret : null),
          install: form.install,
        });
    // The secret has gone to the node; the face keeps no copy.
    setSecret("");
    go.catch((e) => {
      setError(said(e));
      setPage(isPromote ? "run" : "ssh");
    });
  };

  const add = () => {
    if (!form.host.trim()) {
      setError(msg.add_need_host);
      document.querySelector<HTMLElement>("#add-host")?.focus();
      return;
    }
    begin(form.host.trim(), false);
  };

  const close = () => onOpenChange(false);
  const khorMachines = machines.filter((m) => !m.me && !m.sponsor && !m.paused);
  const shownHost = form.host.trim() || gui.add_this_host;

  let content: ReactNode;
  switch (page) {
    case "pick":
      content = (
        <>
          <Head title={gui.add_machine} />
          <Body>
            <Group mark="pick">
              <Row
                mark="khor"
                lead={<IconDevices className="size-nav-glyph text-foreground" />}
                title={gui.add_khor_host}
                note={gui.add_khor_host_note}
                chevron
                onPress={() => setPage("ticket")}
              />
              <Row
                mark="ssh"
                lead={<IconSshHost className="size-nav-glyph text-foreground" />}
                title={gui.add_ssh_host}
                note={gui.add_ssh_host_note}
                chevron
                onPress={() => setPage("ssh")}
              />
            </Group>
          </Body>
          <DialogFooter>
            <Button data-dialog-cancel onClick={close}>
              {gui.delete_keep}
            </Button>
          </DialogFooter>
        </>
      );
      break;
    case "ticket":
      content = <TicketPage machines={machines} onBack={() => setPage("pick")} onClose={close} onJoined={onAdded} />;
      break;
    case "ssh":
      content = (
        <>
          <Head title={gui.add_ssh_host} />
          <Body>
            <Group
              mark="ssh"
              foot={
                form.auth === "password"
                  ? gui.add_password_kept
                  : viaKhor(form.via)
                    ? gui.add_run_on(viaKhor(form.via)!, shownHost)
                    : undefined
              }
            >
              <HostRow form={form} set={set} hosts={hosts} onEnter={add} />
              <Row
                mark="jump"
                title={gui.add_jump}
                value={form.via === "" ? gui.add_jump_none : (viaKhor(form.via) ?? form.via)}
                chevron
                onPress={() => setPage("jump")}
              />
              <Row
                mark="auth"
                title={gui.add_auth}
                value={
                  form.auth === "password"
                    ? gui.add_auth_password
                    : form.key
                      ? gui.add_auth_key_file(keyName(form.key))
                      : gui.accounts_key
                }
                chevron
                onPress={() => setPage("auth")}
              />
              {form.auth === "password" && (
                <FieldRow id="add-password" type="password" label={gui.add_auth_password} value={secret} onChange={setSecret} onEnter={add} />
              )}
            </Group>
            <Group mark="install">
              <Row
                mark="install"
                title={gui.add_install(shownHost)}
                note={gui.add_install_note}
                trail={<Switch on={form.install} label={gui.add_install(shownHost)} onToggle={() => set({ install: !form.install })} />}
              />
            </Group>
            <Failure text={error} />
          </Body>
          <DialogFooter>
            <Button data-dialog-cancel onClick={() => setPage("pick")}>
              {gui.back}
            </Button>
            <Button variant="primary" data-add-go onClick={add}>
              {gui.watch_add}
            </Button>
          </DialogFooter>
        </>
      );
      break;
    case "jump":
      content = (
        <JumpPage
          form={form}
          set={set}
          hosts={hosts}
          khorMachines={khorMachines}
          onDone={() => setPage("ssh")}
          onProxy={() => setPage("proxy")}
          onReach={() => setPage("reach")}
        />
      );
      break;
    case "auth":
      content = <AuthPage form={form} set={set} onDone={() => setPage("ssh")} />;
      break;
    case "proxy":
      content = <ProxyPage form={form} khorMachines={khorMachines} onBack={() => setPage("jump")} />;
      break;
    case "reach":
      content = (
        <Reachers
          form={form}
          machines={machines}
          onPick={(via) => {
            set(viaKhor(via) ? { via, auth: "key", key: null } : { via });
            setPage("ssh");
          }}
          onBack={() => setPage("jump")}
        />
      );
      break;
    case "run":
      content = (
        <Running
          run={run}
          host={runOf?.host ?? form.host}
          via={runOf?.promote ? "" : form.via}
          install={runOf?.promote || form.install}
          error={error}
          onClose={close}
          onBack={runOf?.promote ? undefined : () => setPage("ssh")}
        />
      );
      break;
    case "login":
      content = (
        <LoginPage
          form={form}
          error={error}
          onCancel={close}
          onCopyId={() =>
            openCopyId({
              host: form.host.trim(),
              port: portOf(form),
              jump: form.via && !viaKhor(form.via) ? form.via : null,
              identity: form.key,
            })
              .then((id) => {
                close();
                onOpenSession?.(id);
              })
              .catch((e) => setError(said(e)))
          }
          onPassword={(password) => {
            set({ auth: "password", key: null });
            begin(form.host.trim(), false, { password });
          }}
        />
      );
      break;
    case "passphrase":
      content = (
        <PassphrasePage
          form={form}
          error={error}
          onCancel={close}
          onGo={(passphrase) => begin(form.host.trim(), false, { passphrase })}
        />
      );
      break;
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <TaskOverlay purpose="edit" data-add-machine={page}>
        {content}
      </TaskOverlay>
    </Dialog>
  );
}

// ---- khor 主机: the invitation code, and one made elsewhere -----------------

function TicketPage({
  machines,
  onBack,
  onClose,
  onJoined,
}: {
  machines: DeviceRow[];
  onBack: () => void;
  onClose: () => void;
  onJoined?: () => void;
}) {
  const [ticket, setTicket] = useState<Ticket | null>(null);
  const [before] = useState(() => new Set(machines.map((m) => m.id)));
  const [error, setError] = useState<string | null>(null);
  const [theirs, setTheirs] = useState("");
  const [joining, setJoining] = useState(false);
  const [scanner, setScanner] = useState(false);
  const [scanning, setScanning] = useState(false);
  /** Why the camera gave nothing — said, never a dead button. */
  const [camera, setCamera] = useState<"denied" | "failed" | null>(null);

  // Minted on opening this page, not on mounting the dialog: a code
  // burns on use, and one per glance at ＋ leaves a pile of live ones.
  useEffect(() => {
    let live = true;
    invite()
      .then((t) => live && setTicket(t))
      .catch((e) => live && setError(said(e)));
    fetchScannerHere().then((here) => live && setScanner(here));
    return () => {
      live = false;
    };
  }, []);
  // Somebody used it (UX-8): the code goes and the line says who.
  const joined = before.size > 0 && ticket ? machines.find((m) => !before.has(m.id)) : undefined;
  const joinedId = joined?.id;
  useEffect(() => {
    if (joinedId) feel("success");
  }, [joinedId]);

  const joinWith = (text: string) => {
    const t = text.trim();
    if (joining) return;
    if (!t) {
      setError(gui.need_ticket);
      document.querySelector<HTMLElement>("[data-ticket-input]")?.focus();
      return;
    }
    setJoining(true);
    setError(null);
    pair(t)
      .then(() => {
        feel("success");
        onClose();
        onJoined?.();
      })
      .catch((e) => {
        feel("warning");
        setError(said(e));
      })
      .finally(() => setJoining(false));
  };
  const scan = () => {
    setError(null);
    setCamera(null);
    void scanTicket(() => setScanning(true)).then((read) => {
      setScanning(false);
      if ("ticket" in read) {
        setTheirs(read.ticket);
        joinWith(read.ticket);
      } else if ("refused" in read) {
        setCamera(read.refused);
      }
    });
  };

  return (
    <>
      <Head title={gui.add_khor_host} />
      <Body>
        <div className="flex flex-col gap-in">
          {joined ? (
            <div data-ticket-used role="status" className="text-callout text-foreground">
              {gui.ticket_used_by(joined.shown)}
            </div>
          ) : (
            <>
              {ticket?.qr ? (
                <svg
                  data-ticket-qr
                  role="img"
                  aria-label={gui.ticket_qr}
                  viewBox={`0 0 ${ticket.qr.span} ${ticket.qr.span}`}
                  shapeRendering="crispEdges"
                  className="mx-auto size-60 flex-none text-foreground"
                >
                  <path d={ticket.qr.path} fill="currentColor" />
                </svg>
              ) : (
                !ticket && <div aria-hidden className="mx-auto size-60 flex-none" />
              )}
              <Input
                data-ticket
                aria-label={gui.ticket}
                readOnly
                value={ticket?.token ?? ""}
                onFocus={(e) => e.currentTarget.select()}
                className="font-mono text-mono"
              />
              {ticket && (
                <span data-ticket-window className="text-callout text-muted-foreground">
                  {cli.invite_window(ticket.minutes)}
                </span>
              )}
            </>
          )}
        </div>
        <Group head={gui.have_a_ticket} mark="their-ticket">
          <Row mark="their-ticket">
            <Input
              data-ticket-input
              aria-label={gui.paste_a_ticket}
              placeholder={gui.paste_a_ticket}
              value={theirs}
              onChange={(e) => setTheirs(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && joinWith(theirs)}
              className={bare}
            />
            {scanner && (
              <Button data-scan-ticket className="flex-none" disabled={joining || scanning} onClick={scan}>
                {gui.scan_a_ticket}
              </Button>
            )}
            <Button
              data-join
              className="flex-none"
              disabled={joining}
              aria-busy={joining || undefined}
              onClick={() => joinWith(theirs)}
            >
              {gui.join}
            </Button>
          </Row>
        </Group>
        {joining && (
          <div data-join-dialing role="status" className="text-callout text-muted-foreground">
            {gui.join_dialing}
          </div>
        )}
        <Failure text={error} />
        {camera === "denied" && (
          <div data-scan-denied className="flex flex-col items-start gap-in text-callout text-destructive">
            <span>{gui.scan_camera_denied}</span>
            <Button size="sm" data-scan-settings className="pointer-coarse:min-h-touch" onClick={() => void openCameraSettings()}>
              {gui.scan_open_settings}
            </Button>
          </div>
        )}
        {camera === "failed" && (
          <div data-scan-failed className="text-callout text-destructive">
            {gui.scan_no_camera}
          </div>
        )}
      </Body>
      <DialogFooter>
        <Button data-dialog-cancel onClick={onBack}>
          {gui.back}
        </Button>
        {ticket && !joined && <CopyButton text={ticket.token} label={gui.copy_ticket} />}
      </DialogFooter>
      {scanning && <ScanLayer />}
    </>
  );
}

// ---- SSH 主机: the host row and the pages of choices --------------------------

function HostRow({
  form,
  set,
  hosts,
  onEnter,
}: {
  form: Form;
  set: (f: Partial<Form>) => void;
  hosts: string[];
  onEnter: () => void;
}) {
  const [focused, setFocused] = useState(false);
  // **The list lingers a beat after the field lets go.** On the phone's
  // sheet the list is in the flow and the sheet stands on the bottom
  // edge, so the list going at blur moved the sheet's head — and 添加 in
  // it — between the press and the release, and the press did nothing.
  const linger = useRef(0);
  const typed = form.host.split("@").pop() ?? "";
  const hits = hosts.filter((h) => h.startsWith(typed) && h !== typed);
  // The ssh config's hosts, offered while the field is focused —
  // `PathField`'s list: floats on the wide face, in the flow on the
  // phone's sheet. Nothing to offer, nothing drawn.
  const open = focused && hits.length > 0;
  return (
    <div className="relative">
      <Row mark="host">
        <label htmlFor="add-host" className="flex-none text-body text-foreground">
          {gui.add_host}
        </label>
        <Input
          id="add-host"
          role="combobox"
          aria-expanded={open}
          autoFocus
          autoComplete="off"
          autoCapitalize="off"
          spellCheck={false}
          value={form.host}
          onChange={(e) => set({ host: e.target.value })}
          onFocus={() => {
            window.clearTimeout(linger.current);
            setFocused(true);
          }}
          onBlur={() => {
            linger.current = window.setTimeout(() => setFocused(false), SUGGEST_LINGER_MS);
          }}
          onKeyDown={(e) => e.key === "Enter" && onEnter()}
          placeholder={gui.add_host_hint}
          className={cn(bare, "flex-1 text-right")}
        />
        <span aria-hidden className="h-ctl-sm w-px flex-none bg-border" />
        <Input
          data-add-port
          aria-label={gui.add_port}
          inputMode="numeric"
          placeholder="22"
          value={form.port}
          onChange={(e) => set({ port: e.target.value.replace(/\D/g, "").slice(0, 5) })}
          className={cn(bare, "w-[5ch] flex-none text-right tabular-nums")}
        />
      </Row>
      {open && (
        <ul
          role="listbox"
          data-ssh-config-hosts
          className="absolute inset-x-0 top-full z-20 flex flex-col rounded-lg border bg-popover p-xxs shadow-(--shadow-float) [[data-narrow]_&]:static [[data-narrow]_&]:rounded-none [[data-narrow]_&]:border-0 [[data-narrow]_&]:border-t [[data-narrow]_&]:bg-transparent [[data-narrow]_&]:px-0 [[data-narrow]_&]:shadow-none"
        >
          <li className="px-in pt-xxs pb-xxs text-group text-tertiary [[data-narrow]_&]:px-row">{gui.add_config_hosts}</li>
          {hits.slice(0, 5).map((h) => (
            <li key={h}>
              <button
                type="button"
                role="option"
                aria-selected={false}
                onMouseDown={(e) => {
                  e.preventDefault();
                  set({ host: h });
                }}
                className="flex min-h-touch w-full items-center rounded-sm px-in text-left text-body hover:bg-fill [[data-narrow]_&]:px-row"
              >
                {h}
              </button>
            </li>
          ))}
        </ul>
      )}
    </div>
  );
}

/** 跳板机: one page of choices, a check on the one in use. */
function JumpPage({
  form,
  set,
  hosts,
  khorMachines,
  onDone,
  onProxy,
  onReach,
}: {
  form: Form;
  set: (f: Partial<Form>) => void;
  hosts: string[];
  khorMachines: DeviceRow[];
  onDone: () => void;
  onProxy: () => void;
  onReach: () => void;
}) {
  const names = new Set(khorMachines.map((m) => m.name));
  const jumps = hosts.filter((h) => h !== form.host.trim() && !names.has(h));
  // A jump typed by hand stays on the page as its own checked row.
  const typedJump = form.via && !viaKhor(form.via) && !jumps.includes(form.via) ? form.via : "";
  const [other, setOther] = useState("");
  const pick = (via: string) => {
    // A routed add is a key login only (the secret would cross the wire).
    set(viaKhor(via) ? { via, auth: "key", key: null } : { via });
    onDone();
  };
  return (
    <>
      <Head title={gui.add_jump} />
      <Body>
        <Group mark="jump-none">
          <Row mark="direct" title={gui.add_jump_none} checked={form.via === ""} onPress={() => pick("")} />
        </Group>
        {khorMachines.length > 0 && (
          <Group head={gui.add_jump_khor} foot={gui.add_jump_khor_note} mark="jump-khor">
            {khorMachines.map((m) => (
              <Row
                key={m.id}
                mark={`khor:${m.name}`}
                lead={<Face row={m} />}
                title={m.shown}
                checked={form.via === `khor:${m.name}`}
                onPress={() => pick(`khor:${m.name}`)}
              />
            ))}
            <Row mark="proxy" title={gui.add_copy_config} note={gui.add_copy_config_note} chevron onPress={onProxy} />
          </Group>
        )}
        <Group head={gui.add_jump_ssh} mark="jump-ssh">
          {[...jumps.slice(0, 4), ...(typedJump ? [typedJump] : [])].map((h) => (
            <Row key={h} mark={h} title={h} checked={form.via === h} onPress={() => pick(h)} />
          ))}
          <Row mark="other-jump">
            <Input
              aria-label={gui.add_jump_other}
              placeholder={gui.add_jump_other}
              autoComplete="off"
              autoCapitalize="off"
              spellCheck={false}
              value={other}
              onChange={(e) => setOther(e.target.value)}
              onKeyDown={(e) => e.key === "Enter" && other.trim() && pick(other.trim())}
              className={bare}
            />
          </Row>
        </Group>
        {form.host.trim() && khorMachines.length > 0 && (
          <Group mark="jump-reach">
            <Row mark="reach" title={gui.add_probe} chevron onPress={onReach} />
          </Group>
        )}
      </Body>
      <DialogFooter>
        <Button data-dialog-cancel onClick={() => (other.trim() ? pick(other.trim()) : onDone())}>
          {gui.back}
        </Button>
      </DialogFooter>
    </>
  );
}

/** 认证方式: 密钥 (默认, or a key file) or 密码. A routed add is key only. */
function AuthPage({ form, set, onDone }: { form: Form; set: (f: Partial<Form>) => void; onDone: () => void }) {
  const sponsor = viaKhor(form.via);
  const [picked, setPicked] = useState<string | null>(form.key);
  /** A routed add's key files: the names in that machine's `~/.ssh`
      that have a `.pub` beside them, listed read only. */
  const [farKeys, setFarKeys] = useState<string[] | null>(null);
  useEffect(() => {
    if (!sponsor) return;
    let live = true;
    fetchLs(sponsor, "")
      .then((home) => fetchLs(sponsor, `${home.path.replace(/\/$/, "")}/.ssh`))
      .then((dir) => {
        const names = new Set(dir.entries.map((e) => e.name));
        const keys = dir.entries.filter((e) => !e.dir && names.has(`${e.name}.pub`)).map((e) => `~/.ssh/${e.name}`);
        if (live) setFarKeys(keys);
      })
      .catch(() => live && setFarKeys([]));
    return () => {
      live = false;
    };
  }, [sponsor]);
  const pickKey = (key: string | null) => {
    setPicked(key);
    set({ auth: "key", key });
  };
  const choose = () =>
    pickKeyFile()
      .then((path) => path && pickKey(path))
      .catch(() => {});
  return (
    <>
      <Head title={gui.add_auth} />
      <Body>
        {!sponsor && (
          <Group mark="auth-way">
            <Row mark="key" title={gui.accounts_key} checked={form.auth === "key"} onPress={() => set({ auth: "key" })} />
            <Row
              mark="password"
              title={gui.add_auth_password}
              checked={form.auth === "password"}
              onPress={() => set({ auth: "password", key: null })}
            />
          </Group>
        )}
        {form.auth === "key" && (
          <Group
            mark="key-files"
            head={sponsor ? gui.add_key_files_on(sponsor) : gui.add_key_files}
            foot={sponsor ? gui.add_key_only_on(sponsor) : gui.add_key_path_only}
          >
            <Row
              mark="default-key"
              title={gui.add_key_default}
              note={gui.add_key_default_note}
              checked={form.key === null}
              onPress={() => pickKey(null)}
            />
            {sponsor ? (
              (farKeys ?? []).map((k) => <Row key={k} mark={k} title={k} checked={form.key === k} onPress={() => pickKey(k)} />)
            ) : (
              <>
                {picked && <Row mark="picked-key" title={picked} checked={form.key === picked} onPress={() => pickKey(picked)} />}
                <Row mark="pick-key" title={gui.add_key_pick} onPress={() => void choose()} />
              </>
            )}
          </Group>
        )}
      </Body>
      <DialogFooter>
        <Button data-dialog-cancel onClick={onDone}>
          {gui.back}
        </Button>
      </DialogFooter>
    </>
  );
}

/** For somebody who only wants to ssh through a khor machine: the config
    line and the command (`proxyLine`). khor never writes the ssh config. */
function ProxyPage({ form, khorMachines, onBack }: { form: Form; khorMachines: DeviceRow[]; onBack: () => void }) {
  const via = viaKhor(form.via) ?? khorMachines[0]?.name ?? "";
  const host = form.host.trim() || gui.add_host_hint;
  const [line, setLine] = useState<ProxyLine | null>(null);
  useEffect(() => {
    let live = true;
    proxyLine(via, host, portOf(form))
      .then((l) => live && setLine(l))
      .catch(() => {});
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [via, host]);
  const Copyable = ({ text, mark }: { text: string; mark: string }) => (
    <Row mark={mark}>
      <span data-selectable className="min-w-0 flex-1 font-mono text-mono [overflow-wrap:anywhere] text-foreground">
        {text}
      </span>
      <CopyButton text={text} label={gui.copy_text} />
    </Row>
  );
  return (
    <>
      <Head title={gui.add_via_title(khorMachines.find((m) => m.name === via)?.shown ?? via)} />
      <Body>
        {line && (
          <>
            <Group mark="config-line" head={gui.add_config_line} foot={gui.add_config_line_note(host)}>
              <Copyable mark="config-line" text={line.config_line} />
            </Group>
            <Group mark="command" head={gui.add_or_run}>
              <Copyable mark="command" text={line.command} />
            </Group>
          </>
        )}
      </Body>
      <DialogFooter>
        <Button data-dialog-cancel onClick={onBack}>
          {gui.back}
        </Button>
      </DialogFooter>
    </>
  );
}

// ---- 连通性检测 -----------------------------------------------------------------

function Reachers({
  form,
  machines,
  onPick,
  onBack,
}: {
  form: Form;
  machines: DeviceRow[];
  onPick: (via: string) => void;
  onBack: () => void;
}) {
  const [report, setReport] = useState<ReachReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  /** The one row whose address, banner and key are shown. */
  const [open, setOpen] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    reachProbe(form.host.trim(), portOf(form))
      .then((r) => live && setReport(r))
      .catch((e) => live && setError(said(e)));
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  const port = portOf(form) ?? 22;
  // Without an entrance there is no key to call right: every reached row
  // shows its key, none in the failed ink, and the person picks.
  const unsure = !!report && report.disagree && report.entrance === null;
  const rightKey = report?.rows.find((r) => r.id === report.entrance)?.host_key ?? null;
  const odd = report && !unsure && report.disagree ? report.rows.find((r) => r.host_key !== null && r.host_key !== rightKey) : undefined;
  const nameOf = (id: string) => machines.find((m) => m.id === id)?.name;
  return (
    <>
      <Head title={gui.reach_title} />
      <Body>
        <p className="text-body text-muted-foreground">{gui.reach_lead(form.host.trim(), String(port))}</p>
        {!report && !error && (
          <div data-reach-busy role="status" className="text-callout text-muted-foreground">
            {gui.reach_busy}
          </div>
        )}
        {report && (
          <Group
            mark="reach"
            foot={unsure ? gui.reach_unsure : odd ? gui.reach_odd(odd.shown) : undefined}
          >
            {report.rows.map((r) => {
              const ok = r.outcome === "reached";
              const isOdd = r === odd;
              const shown = open === r.id;
              const via = r.here ? "" : `khor:${nameOf(r.id) ?? r.shown}`;
              return (
                <Row
                  key={r.id}
                  mark={r.id}
                  lead={<Face row={machines.find((m) => m.id === r.id)} className="size-ctl-sm" />}
                  dim={!ok}
                  checked={ok ? form.via === via : undefined}
                  onPress={ok ? () => (shown ? onPick(via) : setOpen(r.id)) : undefined}
                  title={
                    <>
                      {r.shown}
                      {r.here && <span className="text-tertiary">{gui.reach_here}</span>}
                    </>
                  }
                  note={
                    !ok ? (
                      r.why
                    ) : isOdd || shown || unsure ? (
                      <span className="font-mono text-caption">
                        {shown && `${r.addr ?? ""} · ${r.banner ?? ""} · `}
                        <span className={isOdd ? "text-destructive" : undefined}>{r.host_key}</span>
                      </span>
                    ) : undefined
                  }
                  value={
                    ok ? (
                      <>
                        {r.id === report.entrance && <span className="me-in text-foreground">{gui.reach_best}</span>}
                        {latency(r.ms ?? 0)}
                      </>
                    ) : (
                      gui.accounts_trouble_unreachable
                    )
                  }
                />
              );
            })}
          </Group>
        )}
        <Failure text={error} />
      </Body>
      <DialogFooter>
        <Button data-dialog-cancel onClick={onBack}>
          {gui.back}
        </Button>
      </DialogFooter>
    </>
  );
}

// ---- the add under way, and how it ended -----------------------------------------

const STEP_WORD: Record<AddStep, string> = {
  connect: cli.add_step_connect,
  place: cli.add_step_place,
  start: cli.add_step_start,
  pair: cli.add_step_pair,
  undo: cli.add_step_undo,
  satellite: cli.add_step_satellite,
};

type StepState = "done" | "live" | "wait" | "failed";
/** One line of the add: its mark in the leading seat, its verb, what it
    did under it. */
function Step({ step, state, note }: { step: AddStep; state: StepState; note?: string }) {
  return (
    <Row
      mark={`step-${step}`}
      lead={
        <span data-step={state} className="flex w-icon-mark justify-center">
          {state === "done" ? (
            <IconCheck className="size-icon-mark text-foreground" />
          ) : state === "failed" ? (
            <IconClose className="size-icon-mark text-destructive" />
          ) : state === "live" ? (
            <span className="size-2 animate-pulse rounded-full bg-foreground motion-reduce:animate-none" />
          ) : (
            <span className="size-2 rounded-full border border-tertiary" />
          )}
        </span>
      }
      dim={state === "wait"}
      title={STEP_WORD[step]}
      note={note || undefined}
    />
  );
}

function Running({
  run,
  host,
  via,
  install,
  error,
  onClose,
  onBack,
}: {
  run: AddRun | null;
  host: string;
  via: string;
  install: boolean;
  error: string | null;
  onClose: () => void;
  onBack?: () => void;
}) {
  const marks = run?.marks ?? [];
  const end = run?.done ?? null;
  const fellBack = marks.some((m) => m.step === "satellite") && install;
  // The steps still to come, drawn waiting until the add ends or turns
  // into an SSH 主机. The removal is said under the group, not as a line.
  const plan: AddStep[] = install ? ["connect", "place", "start", "pair"] : ["connect", "satellite"];
  const seen = new Set(marks.map((m) => m.step));
  const waiting = end || fellBack ? [] : plan.filter((s) => !seen.has(s));
  const viaWord = via ? (viaKhor(via) ?? via) : "";
  return (
    <>
      <Head title={gui.add_adding(host)} />
      <Body>
        <Group
          mark="steps"
          foot={fellBack && end?.id ? gui.add_fell_back(host) : end?.keeper ?? undefined}
        >
          {marks
            .filter((m) => m.step !== "undo")
            .map((m) => (
              <Step
                key={m.step}
                step={m.step}
                state={m.state}
                note={
                  m.note ||
                  (m.step === "connect" && viaWord && m.state !== "failed"
                    ? gui.add_step_via(viaWord)
                    : m.step === "satellite" && m.state === "done"
                      ? gui.add_satellite_note
                      : "")
                }
              />
            ))}
          {waiting.map((s) => (
            <Step key={s} step={s} state="wait" />
          ))}
        </Group>
        {end?.why && (
          <div data-add-why className="text-callout text-muted-foreground">
            {end.why}
          </div>
        )}
        <Failure text={error ?? (end && !end.id && !marks.some((m) => m.state === "failed") ? end.error : null)} />
      </Body>
      <DialogFooter>
        {end && !end.id && onBack ? (
          <Button data-dialog-cancel onClick={onBack}>
            {gui.back}
          </Button>
        ) : (
          !end && (
            <Button data-dialog-cancel onClick={onClose}>
              {gui.close}
            </Button>
          )
        )}
        {end && (
          <Button variant="primary" data-add-done data-footer-trailing onClick={onClose}>
            {gui.edit_done}
          </Button>
        )}
      </DialogFooter>
    </>
  );
}

// ---- the key was refused; a key that needs its passphrase --------------------

function LoginPage({
  form,
  error,
  onCancel,
  onCopyId,
  onPassword,
}: {
  form: Form;
  error: string | null;
  onCancel: () => void;
  onCopyId: () => void;
  onPassword: (password: string) => void;
}) {
  const host = form.host.trim();
  // A routed add cannot take a password (it would cross the wire).
  const routed = !!viaKhor(form.via);
  const [way, setWay] = useState<"copy" | "password">(error && !routed ? "password" : "copy");
  const [password, setPassword] = useState("");
  const jump = form.via && !routed ? `-o ProxyJump=${form.via} ` : "";
  const port = form.port ? `-p ${form.port} ` : "";
  const key = form.key ? `-i ${form.key} ` : "";
  const go = () => (way === "copy" ? onCopyId() : password && onPassword(password));
  return (
    <>
      <Head title={gui.login_title(host)} />
      <Body>
        <p className="text-body text-muted-foreground">{gui.login_key_refused(host)}</p>
        <Group
          mark="login"
          foot={
            way === "copy" ? (
              <span data-selectable className="font-mono text-mono">
                ssh-copy-id {port}
                {key}
                {jump}
                {host}
              </span>
            ) : (
              gui.add_password_kept
            )
          }
        >
          <Row mark="copy-id" title={gui.login_copy_id} note={gui.login_copy_id_note} checked={way === "copy"} onPress={() => setWay("copy")} />
          {!routed && (
            <Row mark="use-password" title={gui.login_use_password} checked={way === "password"} onPress={() => setWay("password")} />
          )}
          {way === "password" && (
            <FieldRow id="add-login-password" type="password" autoFocus label={gui.add_auth_password} value={password} onChange={setPassword} onEnter={go} />
          )}
        </Group>
        <Failure text={error} />
      </Body>
      <DialogFooter>
        <Button data-dialog-cancel onClick={onCancel}>
          {gui.delete_keep}
        </Button>
        <Button variant="primary" data-login-go onClick={go}>
          {way === "copy" ? gui.login_copy_id : gui.watch_add}
        </Button>
      </DialogFooter>
    </>
  );
}

function PassphrasePage({
  form,
  error,
  onCancel,
  onGo,
}: {
  form: Form;
  error: string | null;
  onCancel: () => void;
  onGo: (passphrase: string) => void;
}) {
  const [passphrase, setPassphrase] = useState("");
  const go = () => passphrase && onGo(passphrase);
  return (
    <>
      <Head title={gui.passphrase_title(keyName(form.key ?? ""))} />
      <Body>
        <Group mark="passphrase" foot={gui.passphrase_kept}>
          <FieldRow id="add-passphrase" type="password" autoFocus label={gui.passphrase} value={passphrase} onChange={setPassphrase} onEnter={go} />
        </Group>
        <Failure text={error} />
      </Body>
      <DialogFooter>
        <Button data-dialog-cancel onClick={onCancel}>
          {gui.delete_keep}
        </Button>
        <Button variant="primary" data-passphrase-go onClick={go}>
          {gui.watch_add}
        </Button>
      </DialogFooter>
    </>
  );
}
