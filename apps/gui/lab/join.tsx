// Lab only (lanes join-a → join-c): the whole 添加机器 flow drawn with
// fake data, one screen at a time, in the real dialog, buttons, fields,
// switch and faces. Nothing here talks to a node. `#1` … `#8` picks the
// screen (`#3b` and the like are the SSH form's own pages), `?dark` /
// `?light` the theme. Served by the 1460 lab vite.
//
// Round 3 (the owner: 「整体ui太丑了」): every screen is a form dialog, so
// on a phone it is the bottom sheet (`ui/dialog`, `app.css`), and the
// lab stamps `data-narrow` the way the shell does. Forms are lists on
// the one ground with hairlines — a row per answer, its value at the
// trailing end, a chevron where a choice opens — not a two-column web
// form. A choice is a page of its own inside the sheet with a check on
// what is picked, as iOS's settings do.
import { useEffect, useState, type ReactNode } from "react";
import ReactDOM from "react-dom/client";

import "@/styles/tokens.css";
import "@/styles/app.css";

import { MachineAvatar } from "@/components/Avatar";
import { IconCheck, IconChevronRight, IconClose, IconDevices } from "@/components/icons";
import { Switch } from "@/components/settings/Switch";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { NARROW_QUERY } from "@/hooks/use-narrow";
import { cn } from "@/lib/utils";

// ---- fixtures -------------------------------------------------------------

const av = (c1: string, c2: string, c3: string) => ({
  canvas: 80,
  background: c1,
  radius_ratio: 0.5,
  mark: { ink: "light", veil: 0 },
  paint: {
    variant: "marble",
    blur: 7,
    veins: [
      { vein: "shard", scale: 1.2, rotate: 20, dx: 3, dy: -3, overlay: false, color: c2 },
      { vein: "sweep", scale: 1.4, rotate: 200, dx: -4, dy: 6, overlay: true, color: c3 },
    ],
  },
});
// eslint-disable-next-line @typescript-eslint/no-explicit-any
const faces: Record<string, any> = {
  MacLab: av("#5f6b2a", "#c5c8a6", "#a6b87a"),
  turing: av("#2a4f6b", "#8abeb7", "#81a2be"),
  hinton: av("#6b3a2a", "#f0c674", "#cc6666"),
  lecun: av("#4a4a4a", "#b294bb", "#969896"),
  "gpu-box": av("#3a5a40", "#dad7cd", "#a3b18a"),
  bastion: av("#5a4a6b", "#d7cdda", "#a38ab1"),
  "lab-gw": av("#6b5a2a", "#dad2a6", "#b8a67a"),
  "gpu-box-2": av("#2a6b5a", "#a6dad2", "#7ab8a6"),
  "aliyun-bj": av("#6b2a4a", "#dab0c8", "#b87a9a"),
};

/** Host lines of the user's ssh config (read-only; patterns skipped). */
const sshHosts = ["gpu-box", "gpu-box-2", "bastion", "lab-gw", "hinton", "lecun", "aliyun-bj"];
const khorMachines = ["turing", "hinton", "lecun"];
/** A routed add's key files: the names in that machine's ~/.ssh. */
const farKeys = ["id_ed25519", "id_rsa", "lab_key"];

/** The SSH form's answers, kept above the screens so a page of choices
    and the form it returns to share them. */
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
const blank: Form = { host: "gpu-box", port: "", via: "bastion", auth: "key", key: null, install: true };

const go = (to: string) => (location.hash = `#${to}`);
const viaKhor = (via: string) => (via.startsWith("khor:") ? via.slice(5) : null);

// ---- pieces ---------------------------------------------------------------

function Face({ name, className }: { name: string; className?: string }) {
  return <MachineAvatar face={faces[name] ?? null} className={cn("size-ctl-md flex-none", className)} />;
}

/**
 * **Proposed glyph (not in the set yet): an SSH host.** The owner asked
 * for the two first-screen choices at one icon weight; the family's
 * terminal is a bare `>_` (0.15 ink) and the machine is the rack, two
 * solid units — side by side the first read as a lighter, smaller mark.
 * This is the rack's own box on the 24 canvas, one unit tall: the window
 * is the solid piece and the prompt is punched through it with the
 * family's line, the keyboard's construction. If the owner keeps it, it
 * goes into `icons.tsx` as the 36th with its measured fit.
 */
function IconSshHost({ className }: { className?: string }) {
  // `devices`' fit (square share): the same box, so the same painted size.
  const f = { x: 1.45, y: 1.45, side: 21.11 };
  const line = 0.072 * f.side;
  return (
    <svg
      className={className}
      viewBox={`${f.x} ${f.y} ${f.side} ${f.side}`}
      fill="none"
      stroke="currentColor"
      strokeWidth={line}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden="true"
    >
      <mask id="lab-ssh-cut">
        <rect x={f.x} y={f.y} width={f.side} height={f.side} fill="white" stroke="none" />
        <path d="M7.7 8.9 10.8 12l-3.1 3.1M12.6 15.1h3.8" stroke="black" strokeWidth={line} />
      </mask>
      <rect data-body mask="url(#lab-ssh-cut)" x="3.5" y="4.6" width="17" height="14.8" rx="2.6" />
    </svg>
  );
}

/**
 * A list on the one ground: rows with a hairline between them that
 * starts where the words start (macOS and iOS), an optional group name
 * above in `group` / `tertiary`, and an optional note below in `callout`
 * / `secondary`. No card, no fill: the sheet's ground and its lines.
 */
function Group({ head, foot, children, mark }: { head?: string; foot?: ReactNode; children: ReactNode; mark?: string }) {
  return (
    <section data-join-group={mark} className="flex flex-col gap-xxs">
      {head && <h3 className="text-group text-tertiary">{head}</h3>}
      <div className="-mx-row flex flex-col [&>*:first-child_[data-row-body]]:border-t-0">{children}</div>
      {foot && <div className="text-callout text-muted-foreground">{foot}</div>}
    </section>
  );
}

/**
 * One row: a leading seat (a glyph, a face, a step's mark), the title and
 * its note, a value in `secondary`, then the trailing control — a check
 * on the picked choice, a chevron where a choice opens, a switch.
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
  /** A row that cannot be picked (an unreachable machine): words in
      `secondary`, no press. */
  dim?: boolean;
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
    <button type="button" onClick={onPress} className={cn(cls, "hover:bg-fill")}>
      {body}
    </button>
  ) : (
    <div className={cls}>{body}</div>
  );
}

/** A field that is its row: no fill of its own, the row's hairlines
    around it. */
const bare = "h-auto min-h-0 rounded-none bg-transparent px-0";

/** A pretend QR code: a fixed scatter of modules in the real one's frame. */
function FakeQr() {
  const n = 29;
  let seed = 7;
  const rnd = () => ((seed = (seed * 1103515245 + 12345) & 0x7fffffff) / 0x7fffffff);
  let d = "";
  const finder = (x: number, y: number) => {
    d += `M${x} ${y}h7v7h-7zM${x + 1} ${y + 1}v5h5v-5zM${x + 2} ${y + 2}h3v3h-3z`;
  };
  for (let y = 0; y < n; y++)
    for (let x = 0; x < n; x++) {
      const inFinder = (x < 8 && y < 8) || (x > n - 9 && y < 8) || (x < 8 && y > n - 9);
      if (!inFinder && rnd() > 0.52) d += `M${x + 4} ${y + 4}h1v1h-1z`;
    }
  finder(4, 4);
  finder(n - 3, 4);
  finder(4, n - 3);
  return (
    <svg viewBox={`0 0 ${n + 8} ${n + 8}`} shapeRendering="crispEdges" className="mx-auto size-60 flex-none text-foreground">
      <path d={d} fill="currentColor" fillRule="evenodd" />
    </svg>
  );
}

/** A sheet's title. On the phone it shares the first row with the
    footer's buttons (`app.css`, UX-10). */
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

// ---- 1 · the first choice ---------------------------------------------------

function Pick() {
  return (
    <>
      <Head title="添加机器" />
      <Body>
        <Group mark="pick">
          <Row
            lead={<IconDevices className="size-nav-glyph text-foreground" />}
            title="khor 主机"
            note="已安装 khor，使用邀请码添加"
            chevron
            onPress={() => go("2")}
          />
          <Row
            lead={<IconSshHost className="size-nav-glyph text-foreground" />}
            title="SSH 主机"
            note="通过 SSH 添加"
            chevron
            onPress={() => go("3")}
          />
        </Group>
      </Body>
      <DialogFooter>
        <Button>取消</Button>
      </DialogFooter>
    </>
  );
}

// ---- 2 · khor 主机: the ticket, and a ticket from elsewhere -----------------

function AtHand() {
  return (
    <>
      <Head title="khor 主机" />
      <Body>
        <div className="flex flex-col gap-in">
          <FakeQr />
          <Input readOnly value="khor1qxy7…（邀请码）" aria-label="邀请码" className="font-mono text-mono" />
          <span className="text-callout text-muted-foreground">15 分钟内有效，仅可使用一次</span>
        </div>
        <Group head="已有邀请码">
          <Row>
            <Input placeholder="输入邀请码" aria-label="输入邀请码" className={bare} />
            <Button className="flex-none">加入</Button>
          </Row>
        </Group>
      </Body>
      <DialogFooter>
        <Button>关闭</Button>
        <Button variant="primary">复制邀请码</Button>
      </DialogFooter>
    </>
  );
}

// ---- 3 · SSH 主机: the form and its pages of choices -----------------------

function HostRow({ form, set, suggest }: { form: Form; set: (f: Partial<Form>) => void; suggest?: boolean }) {
  const [focused, setFocused] = useState(false);
  const open = suggest || focused;
  const typed = form.host.split("@").pop() ?? "";
  const hits = sshHosts.filter((h) => h.startsWith(typed));
  const list = hits.length > 0 ? hits : sshHosts;
  return (
    <div className="relative">
      <Row>
        <label htmlFor="lab-host" className="flex-none text-body text-foreground">
          主机
        </label>
        <Input
          id="lab-host"
          role="combobox"
          aria-expanded={open}
          autoComplete="off"
          spellCheck={false}
          value={form.host}
          onChange={(e) => set({ host: e.target.value })}
          onFocus={() => setFocused(true)}
          onBlur={() => setFocused(false)}
          placeholder="user@host"
          className={cn(bare, "flex-1 text-right")}
        />
        <span aria-hidden className="h-ctl-sm w-px flex-none bg-border" />
        <Input
          aria-label="端口"
          inputMode="numeric"
          placeholder="22"
          value={form.port}
          onChange={(e) => set({ port: e.target.value.replace(/\D/g, "") })}
          className={cn(bare, "w-[5ch] flex-none text-right tabular-nums")}
        />
      </Row>
      {open && (
        // The ssh config's hosts, offered while the field is focused —
        // `PathField`'s list: floats on the wide face, in the flow on
        // the phone's sheet.
        <ul
          role="listbox"
          data-ssh-config-hosts
          className="absolute inset-x-0 top-full z-20 flex flex-col rounded-lg border bg-popover p-xxs shadow-(--shadow-float) [[data-narrow]_&]:static [[data-narrow]_&]:rounded-none [[data-narrow]_&]:border-0 [[data-narrow]_&]:border-t [[data-narrow]_&]:bg-transparent [[data-narrow]_&]:px-0 [[data-narrow]_&]:shadow-none"
        >
          <li className="px-in pt-xxs pb-xxs text-group text-tertiary [[data-narrow]_&]:px-row">SSH 配置中的主机</li>
          {list.slice(0, 5).map((h) => (
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

const viaWord = (via: string) => (via === "" ? "直接连接" : (viaKhor(via) ?? via));
const keyName = (path: string) => path.split("/").pop() ?? path;
const authWord = (f: Form) => (f.auth === "password" ? "密码" : f.key ? `密钥 · ${keyName(f.key)}` : "密钥");

function SshForm({ form, set, suggest }: { form: Form; set: (f: Partial<Form>) => void; suggest?: boolean }) {
  const host = form.host.trim() || "该主机";
  const sponsor = viaKhor(form.via);
  return (
    <>
      <Head title="SSH 主机" />
      <Body>
        <Group
          mark="ssh"
          foot={
            form.auth === "password"
              ? "密码仅保存在本机钥匙串中"
              : sponsor
                ? `添加在 ${sponsor} 上执行，由 ${sponsor} 通过 SSH 连接 ${host}`
                : undefined
          }
        >
          <HostRow form={form} set={set} suggest={suggest} />
          <Row title="跳板机" value={viaWord(form.via)} chevron onPress={() => go("3b")} />
          <Row title="认证方式" value={authWord(form)} chevron onPress={() => go("3c")} />
          {form.auth === "password" && (
            <Row>
              <label htmlFor="lab-pw" className="flex-none text-body text-foreground">
                密码
              </label>
              <Input id="lab-pw" type="password" defaultValue="placeholder" className={cn(bare, "flex-1 text-right")} />
            </Row>
          )}
        </Group>
        <Group mark="install">
          <Row
            title={`在 ${host} 上安装 khor`}
            note="关闭则仅通过 SSH 连接，不在该主机上留下文件"
            trail={<Switch on={form.install} label={`在 ${host} 上安装 khor`} onToggle={() => set({ install: !form.install })} />}
          />
        </Group>
      </Body>
      <DialogFooter>
        <Button>取消</Button>
        <Button variant="primary" onClick={() => go(form.install ? "5" : "6s")}>
          添加
        </Button>
      </DialogFooter>
    </>
  );
}

/** 跳板机: one page of choices, a check on the one in use. */
function JumpPage({ form, set }: { form: Form; set: (f: Partial<Form>) => void }) {
  const pick = (via: string) => {
    set({ via, ...(viaKhor(via) ? { auth: "key" as const, key: null } : {}) });
    go("3");
  };
  return (
    <>
      <Head title="跳板机" />
      <Body>
        <Group>
          <Row title="直接连接" checked={form.via === ""} onPress={() => pick("")} />
        </Group>
        <Group head="通过 khor 主机" foot="添加在该机器上执行">
          {khorMachines.map((m) => (
            <Row key={m} lead={<Face name={m} />} title={m} checked={form.via === `khor:${m}`} onPress={() => pick(`khor:${m}`)} />
          ))}
          <Row title="复制 SSH 配置…" note="不添加机器，只经 khor 主机用 ssh 连接" chevron onPress={() => go("3e")} />
        </Group>
        <Group head="通过 SSH 跳板机">
          {sshHosts
            .filter((h) => h !== form.host && !khorMachines.includes(h))
            .slice(0, 4)
            .map((h) => (
              <Row key={h} title={h} checked={form.via === h} onPress={() => pick(h)} />
            ))}
        </Group>
        <Group>
          <Row title="检测哪些机器可以连接…" chevron onPress={() => go("4")} />
        </Group>
      </Body>
      <DialogFooter>
        <Button onClick={() => go("3")}>返回</Button>
      </DialogFooter>
    </>
  );
}

/** 认证方式: 密钥 (默认, or a key file) or 密码. A routed add is key
    only — a password or a passphrase would have to cross the wire. */
function AuthPage({ form, set }: { form: Form; set: (f: Partial<Form>) => void }) {
  const sponsor = viaKhor(form.via);
  const [picked, setPicked] = useState<string | null>(form.key);
  const pickKey = (key: string | null) => {
    setPicked(key);
    set({ auth: "key", key });
  };
  return (
    <>
      <Head title="认证方式" />
      <Body>
        {!sponsor && (
          <Group>
            <Row title="密钥" checked={form.auth === "key"} onPress={() => set({ auth: "key" })} />
            <Row title="密码" checked={form.auth === "password"} onPress={() => set({ auth: "password", key: null })} />
          </Group>
        )}
        {form.auth === "key" && (
          <Group
            head={sponsor ? `${sponsor} 上的密钥文件` : "密钥文件"}
            foot={sponsor ? `通过 ${sponsor} 添加时只能使用密钥。khor 只记录密钥文件的路径` : "khor 只记录密钥文件的路径，不复制密钥"}
          >
            <Row title="默认" note="SSH 配置和 ssh-agent 中的密钥" checked={form.key === null} onPress={() => pickKey(null)} />
            {sponsor ? (
              farKeys.map((k) => (
                <Row key={k} title={`~/.ssh/${k}`} checked={form.key === `~/.ssh/${k}`} onPress={() => pickKey(`~/.ssh/${k}`)} />
              ))
            ) : (
              <>
                {picked && <Row title={picked} checked={form.key === picked} onPress={() => pickKey(picked)} />}
                {/* The file picker of the machine that runs ssh; faked here. */}
                <Row title="选择密钥文件…" onPress={() => pickKey("~/.ssh/id_lab")} />
              </>
            )}
          </Group>
        )}
      </Body>
      <DialogFooter>
        <Button onClick={() => go("3")}>返回</Button>
      </DialogFooter>
    </>
  );
}

/** road-a's `proxyLine(machine, host, port)` → `ProxyLine`, faked: for
    a person who only wants to ssh through a khor machine. khor never
    writes the ssh config; it hands over the line and the command. */
function ProxyPage({ form }: { form: Form }) {
  const via = viaKhor(form.via) ?? "turing";
  const line = { config_line: `ProxyCommand khor pipe --via ${via} %h %p`, command: `ssh -o ProxyCommand='khor pipe --via ${via} %h %p' ${form.host}` };
  const Copyable = ({ text }: { text: string }) => (
    <Row>
      <span data-selectable className="min-w-0 flex-1 font-mono text-mono [overflow-wrap:anywhere] text-foreground">
        {text}
      </span>
      <Button className="flex-none">复制</Button>
    </Row>
  );
  return (
    <>
      <Head title={`通过 ${via} 连接`} />
      <Body>
        <Group head="SSH 配置中的一行" foot={`加在 Host ${form.host} 下。khor 不修改你的 SSH 配置`}>
          <Copyable text={line.config_line} />
        </Group>
        <Group head="或直接运行">
          <Copyable text={line.command} />
        </Group>
      </Body>
      <DialogFooter>
        <Button onClick={() => go("3b")}>返回</Button>
      </DialogFooter>
    </>
  );
}

// ---- 4 · 连通性检测 ----------------------------------------------------------

/** road-a's ReachReport, faked. */
type ReachRow = {
  id: string;
  shown: string;
  here: boolean;
  /** `silent`: connected, but no `SSH-` first line — kept with its ms
      and address, never the entrance, drawn as 无法连接 with its why. */
  outcome: "reached" | "refused" | "timeout" | "no-name" | "unasked" | "predates" | "silent";
  ms: number | null;
  addr: string | null;
  banner: string | null;
  host_key: string | null;
  why: string | null;
};
type Report = {
  host: string;
  port: number;
  /** When keys disagree, only among rows holding this machine's own
      key; `null` when this machine holds none — the person picks. */
  entrance: string | null;
  disagree: boolean;
  known_as: string | null;
  rows: ReachRow[];
};
/** The key this machine's known_hosts holds for the host (lab-only: the
    real report says it through `entrance`). */
const mineKey = "SHA256:q3Vf…9kLw";
const report: Report = {
  host: "gpu-box",
  port: 22,
  entrance: "c".repeat(64),
  disagree: true,
  known_as: null,
  rows: [
    { id: "a".repeat(64), shown: "MacLab", here: true, outcome: "timeout", ms: null, addr: null, banner: null, host_key: null, why: "连接超时" },
    { id: "c".repeat(64), shown: "turing", here: false, outcome: "reached", ms: 3, addr: "10.0.0.7:22", banner: "OpenSSH_9.6", host_key: "SHA256:q3Vf…9kLw", why: null },
    { id: "d".repeat(64), shown: "hinton", here: false, outcome: "reached", ms: 41, addr: "10.0.0.7:22", banner: "OpenSSH_9.6", host_key: "SHA256:q3Vf…9kLw", why: null },
    { id: "e".repeat(64), shown: "lecun", here: false, outcome: "reached", ms: 38, addr: "192.168.1.7:22", banner: "OpenSSH_8.9", host_key: "SHA256:Zt0p…aa1M", why: null },
    { id: "f".repeat(64), shown: "aliyun", here: false, outcome: "silent", ms: 2, addr: "198.18.0.12:22", banner: null, host_key: null, why: "已连接，但没有 SSH 应答，可能是代理返回的虚假地址" },
  ],
};
/** The same probe from a machine that never logged in to the host: the
    keys disagree and none is this machine's, so nothing is the entrance. */
const unsureReport: Report = { ...report, entrance: null };

function Reachers({ form, set, report }: { form: Form; set: (f: Partial<Form>) => void; report: Report }) {
  // Expanded: the one row whose address, banner and key are shown.
  const [open, setOpen] = useState<string | null>(null);
  // Without an entrance there is no key to call right: every reached row
  // shows its key, none in the failed ink, and the person picks.
  const unsure = report.disagree && report.entrance === null;
  const odd = unsure ? undefined : report.rows.find((r) => report.disagree && r.host_key !== null && r.host_key !== mineKey);
  return (
    <>
      <Head title="连通性检测" />
      <Body>
        <p className="text-body text-muted-foreground">
          每台在线的机器分别尝试连接 {report.host}:{report.port}
        </p>
        <Group
          mark="reach"
          foot={
            unsure
              ? "各机器记录的主机密钥不一致，本机没有该主机的记录。核对密钥后选择一台机器"
              : odd && `${odd.shown} 记录的主机密钥与其他机器记录的不一致，可能不是同一台主机`
          }
        >
          {report.rows.map((r) => {
            const ok = r.outcome === "reached";
            const isOdd = r === odd;
            const via = r.here ? "" : `khor:${r.shown}`;
            const shown = open === r.id;
            return (
              <Row
                key={r.id}
                lead={<Face name={r.shown} className="size-ctl-sm" />}
                dim={!ok}
                checked={ok && form.via === via}
                onPress={ok ? () => (shown ? (set({ via }), go("3")) : setOpen(r.id)) : undefined}
                title={
                  <>
                    {r.shown}
                    {r.here && <span className="text-tertiary">（本机）</span>}
                  </>
                }
                note={
                  !ok ? (
                    r.why
                  ) : isOdd || shown || unsure ? (
                    <span className="font-mono text-caption">
                      {shown && `${r.addr} · ${r.banner} · `}
                      <span className={isOdd ? "text-destructive" : undefined}>{r.host_key}</span>
                    </span>
                  ) : undefined
                }
                value={
                  ok ? (
                    <>
                      {r.id === report.entrance && <span className="me-in text-foreground">延迟最低</span>}
                      {r.ms} ms
                    </>
                  ) : (
                    "无法连接"
                  )
                }
              />
            );
          })}
        </Group>
      </Body>
      <DialogFooter>
        <Button onClick={() => go("3b")}>返回</Button>
      </DialogFooter>
    </>
  );
}

// ---- 5 / 6 · installing, and the fallback ----------------------------------

type StepState = "done" | "live" | "wait" | "failed";
/** One line of the install: its mark in the leading seat, its verb, what
    it did under it. */
function Step({ state, word, note }: { state: StepState; word: string; note?: string }) {
  return (
    <Row
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
      title={word}
      note={note}
    />
  );
}

function Installing({ form }: { form: Form }) {
  const via = viaWord(form.via);
  return (
    <>
      <Head title={`添加 ${form.host}`} />
      <Body>
        <Group mark="steps">
          <Step state="done" word="连接" note={form.via ? `通过 ${via}，34 ms` : "34 ms"} />
          <Step state="done" word="安装 khor" note="0.1.14，由目标主机自行下载" />
          <Step state="live" word="启动" note="systemd --user，退出登录后继续运行" />
          <Step state="wait" word="配对" />
        </Group>
      </Body>
      <DialogFooter>
        <Button>取消</Button>
      </DialogFooter>
    </>
  );
}

/** Checked and the install failed: the automatic fallback. `alone`: the
    switch was off, so it was an SSH host from the start. */
function Fallback({ form, alone = false }: { form: Form; alone?: boolean }) {
  const via = viaWord(form.via);
  return (
    <>
      <Head title={`添加 ${form.host}`} />
      <Body>
        <Group
          mark="steps"
          foot={alone ? undefined : `未能在 ${form.host} 上安装 khor，已移除安装的文件`}
        >
          <Step state="done" word="连接" note={form.via ? `通过 ${via}，34 ms` : "34 ms"} />
          {!alone && <Step state="failed" word="安装 khor" note="无法在该主机上持续运行 khor：没有 systemd --user，也没有 tmux" />}
          <Step state="done" word="作为 SSH 主机添加" note="操作通过 SSH 执行" />
        </Group>
      </Body>
      <DialogFooter>
        {/* The phone's sheet spreads its footer edge to edge; the one
            act keeps the trailing edge, as iOS's Done does. */}
        <span aria-hidden />
        <Button variant="primary">完成</Button>
      </DialogFooter>
    </>
  );
}

// ---- 7 · the key was refused; 7b · a key that asks for its passphrase ------

function AuthChoice({ form }: { form: Form }) {
  const [way, setWay] = useState<"copy" | "password">("copy");
  const jump = form.via && !viaKhor(form.via) ? `-o ProxyJump=${form.via} ` : "";
  return (
    <>
      <Head title={`登录 ${form.host}`} />
      <Body>
        <p className="text-body text-muted-foreground">{form.host} 不接受本机的密钥</p>
        <Group
          foot={
            way === "copy" ? (
              <span data-selectable className="font-mono text-mono">
                ssh-copy-id {jump}
                {form.host}
              </span>
            ) : (
              "密码仅保存在本机钥匙串中"
            )
          }
        >
          <Row title="上传公钥" note="在终端中运行 ssh-copy-id，之后使用密钥登录" checked={way === "copy"} onPress={() => setWay("copy")} />
          <Row title="使用密码" checked={way === "password"} onPress={() => setWay("password")} />
          {way === "password" && (
            <Row>
              <label htmlFor="lab-pw7" className="flex-none text-body text-foreground">
                密码
              </label>
              <Input id="lab-pw7" type="password" autoFocus className={cn(bare, "flex-1 text-right")} />
            </Row>
          )}
        </Group>
      </Body>
      <DialogFooter>
        <Button>取消</Button>
        <Button variant="primary">{way === "copy" ? "上传公钥" : "添加"}</Button>
      </DialogFooter>
    </>
  );
}

function Passphrase({ form }: { form: Form }) {
  const key = form.key ?? "~/.ssh/id_lab";
  return (
    <>
      <Head title={`${keyName(key)} 需要口令`} />
      <Body>
        <Group foot="口令仅保存在本机钥匙串中">
          <Row>
            <label htmlFor="lab-pp" className="flex-none text-body text-foreground">
              口令
            </label>
            <Input id="lab-pp" type="password" className={cn(bare, "flex-1 text-right")} />
          </Row>
        </Group>
      </Body>
      <DialogFooter>
        <Button>取消</Button>
        <Button variant="primary">添加</Button>
      </DialogFooter>
    </>
  );
}

// ---- 8 · the machine page's road -------------------------------------------

/** road-a's `Road.broken`: which leg of the road failed. A jump host
    is not a machine in the table, so its end is `null`. */
type Leg = { from: string | null; to: string | null; leg: "to-sponsor" | "to-jump" | "jump-to-target" | "ssh" | "auth" };

/** The leg's place on the drawn road (which dash), and its sentence. */
function legOf(b: Leg, hops: string[], jump: string | undefined): { at: number; why: string } {
  const last = hops.length - 1;
  switch (b.leg) {
    case "to-sponsor":
      return { at: 1, why: `本机无法连接 ${b.to}` };
    case "to-jump":
      return { at: hops.indexOf(jump ?? ""), why: `本机无法连接跳板机 ${jump}` };
    case "jump-to-target":
      return { at: last, why: `跳板机无法连接目标主机：${jump} 无法连接 ${b.to}` };
    case "ssh":
      return { at: last, why: `${b.from} 无法通过 SSH 连接 ${b.to}` };
    case "auth":
      return { at: last, why: `${b.to} 不接受 ${b.from} 的密钥` };
  }
}

/** The road a machine is reached on, hop by hop; a broken leg is the
    one dashed in the failed ink. The sentence under it stays calm, with
    ssh's own words after it. */
function Road({ machine, road, hops, jump, broken, said, sat }: {
  machine: string;
  road: string;
  hops: string[];
  jump?: string;
  broken?: Leg;
  said?: string;
  sat?: boolean;
}) {
  const leg = broken && legOf(broken, hops, jump);
  return (
    <section data-road-machine={machine} className="flex flex-col gap-in border-t pt-base first:border-t-0 first:pt-0">
      <div className="flex min-h-ctl-lg items-center gap-row">
        <Face name={machine} className="size-avatar" />
        <span className="min-w-0 flex-1 truncate text-headline text-foreground">{machine}</span>
        {sat && <Button>安装 khor…</Button>}
      </div>
      <div className="flex items-baseline gap-row text-body">
        <span className="w-16 flex-none text-muted-foreground">连接</span>
        <span className="text-foreground tabular-nums">{road}</span>
      </div>
      <div className="flex items-baseline gap-row text-body">
        <span className="w-16 flex-none text-muted-foreground">线路</span>
        <span data-road className="flex min-w-0 flex-wrap items-center gap-in">
          {hops.map((h, i) => (
            <span key={h} className="flex items-center gap-in">
              {i > 0 && (
                <span
                  aria-hidden
                  className={cn("h-0 w-6 flex-none border-t", leg?.at === i ? "border-dashed border-destructive" : "border-tertiary")}
                />
              )}
              <span className={leg && i >= leg.at ? "text-muted-foreground" : "text-foreground"}>{h}</span>
            </span>
          ))}
        </span>
      </div>
      {leg && (
        <div data-road-broken={broken?.leg} className="ps-[calc(4rem+var(--gap-row))] text-callout text-muted-foreground">
          {leg.why}
          {said && `（${said}）`}
        </div>
      )}
    </section>
  );
}

function MachinePage() {
  return (
    <main className="mx-auto flex max-w-detail flex-col gap-base px-pane pt-24 pb-pane [[data-narrow]_&]:pt-56">
      <Road machine="hinton" road="直连 · 31 ms" hops={["本机", "hinton"]} />
      <Road machine="lecun" road="中继 · 140 ms" hops={["本机", "中继 北京", "lecun"]} />
      <Road machine="turing" road="通过 hinton" hops={["本机", "hinton", "turing"]} />
      <Road
        machine="gpu-box"
        road="SSH 主机 · 通过 bastion"
        sat
        hops={["本机", "bastion", "gpu-box"]}
        jump="bastion"
        broken={{ from: null, to: "gpu-box", leg: "jump-to-target" }}
        said="Connection timed out"
      />
      <Road
        machine="lab-gw"
        road="SSH 主机 · 通过 bastion"
        sat
        hops={["本机", "bastion", "lab-gw"]}
        jump="bastion"
        broken={{ from: "本机", to: null, leg: "to-jump" }}
        said="Connection refused"
      />
      <Road
        machine="gpu-box-2"
        road="SSH 主机 · 由 turing 连接"
        hops={["本机", "turing", "gpu-box-2"]}
        broken={{ from: "turing", to: "gpu-box-2", leg: "auth" }}
      />
      <Road
        machine="aliyun-bj"
        road="SSH 主机 · 由 hinton 连接"
        hops={["本机", "hinton", "aliyun-bj"]}
        broken={{ from: "本机", to: "hinton", leg: "to-sponsor" }}
      />
    </main>
  );
}

// ---- the lab shell --------------------------------------------------------

type Screen = [key: string, word: string, draw: (f: Form, set: (p: Partial<Form>) => void) => ReactNode];
const screens: Screen[] = [
  ["1", "选择", () => <Pick />],
  ["2", "khor 主机", () => <AtHand />],
  ["3", "SSH 主机", (f, s) => <SshForm form={f} set={s} />],
  ["3a", "主机建议", (f, s) => <SshForm form={f} set={s} suggest />],
  ["3b", "跳板机", (f, s) => <JumpPage form={f} set={s} />],
  ["3c", "认证方式", (f, s) => <AuthPage form={f} set={s} />],
  ["3d", "通过 khor 主机", (f, s) => <AuthPage form={{ ...f, via: "khor:turing", auth: "key" }} set={s} />],
  ["3e", "SSH 配置", (f) => <ProxyPage form={{ ...f, via: "khor:turing" }} />],
  ["4", "连通性检测", (f, s) => <Reachers form={f} set={s} report={report} />],
  ["4b", "密钥不一致", (f, s) => <Reachers form={f} set={s} report={unsureReport} />],
  ["5", "安装中", (f) => <Installing form={f} />],
  ["6", "安装未成功", (f) => <Fallback form={f} />],
  ["6s", "仅 SSH", (f) => <Fallback form={f} alone />],
  ["7", "密钥被拒", (f) => <AuthChoice form={f} />],
  ["7b", "密钥口令", (f) => <Passphrase form={{ ...f, key: f.key ?? "~/.ssh/id_lab" }} />],
  ["8", "机器页", () => null],
];

function Lab() {
  const [at, setAt] = useState(() => location.hash.slice(1) || "1");
  const [form, setForm] = useState<Form>(blank);
  const set = (p: Partial<Form>) => setForm((f) => ({ ...f, ...p }));
  useEffect(() => {
    const on = () => setAt(location.hash.slice(1) || "1");
    window.addEventListener("hashchange", on);
    return () => window.removeEventListener("hashchange", on);
  }, []);
  // The shell's breakpoint, stamped on the root as `App` does, so the
  // phone's sheet is the real one.
  useEffect(() => {
    const m = window.matchMedia(NARROW_QUERY);
    const stamp = () => document.documentElement.toggleAttribute("data-narrow", m.matches);
    stamp();
    m.addEventListener("change", stamp);
    return () => m.removeEventListener("change", stamp);
  }, []);
  const screen = screens.find(([k]) => k === at) ?? screens[0];
  const theme = () => {
    const r = document.documentElement;
    r.setAttribute("data-theme", r.getAttribute("data-theme") === "dark" ? "light" : "dark");
  };
  return (
    <div className="min-h-dvh bg-background text-foreground">
      <nav
        data-lab-bar
        style={{ zIndex: 1000, pointerEvents: "auto" }}
        className="fixed inset-x-0 top-0 flex flex-wrap items-center gap-xxs border-b bg-background px-row py-xxs text-caption"
      >
        {screens.map(([k, word]) => (
          <Button key={k} size="sm" variant={k === at ? "secondary" : "ghost"} onClick={() => go(k)}>
            {k} {word}
          </Button>
        ))}
        <Button size="sm" variant="ghost" onClick={theme}>
          深浅
        </Button>
      </nav>
      {at === "8" ? (
        <MachinePage />
      ) : (
        <Dialog open onOpenChange={() => {}}>
          <DialogContent form data-add-machine={at}>
            {screen[2](form, set)}
          </DialogContent>
        </Dialog>
      )}
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(<Lab />);
