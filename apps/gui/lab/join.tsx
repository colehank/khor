// Lab only (lane join-a, Stage 0): the whole 添加机器 flow drawn with fake
// data, one screen at a time, in the real dialog, buttons, fields and
// faces. Nothing here talks to a node. `#1` … `#8` picks the screen,
// `?dark` / `?light` the theme. Served by the 1460 lab vite.
import { useEffect, useState, type ReactNode } from "react";
import ReactDOM from "react-dom/client";

import "@/styles/tokens.css";
import "@/styles/app.css";

import { MachineAvatar } from "@/components/Avatar";
import { IconCheck, IconChevronRight, IconClose, IconDevices, IconTerminal } from "@/components/icons";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { FormGrid, FormLabel } from "@/components/ui/form-grid";
import { Input } from "@/components/ui/input";
import { PopupButton } from "@/components/ui/popup-button";
import { SegmentedControl } from "@/components/ui/segmented-control";
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
};

const sshHosts = ["gpu-box", "bastion", "lab-gw", "hinton", "lecun", "aliyun-bj"];
const khorMachines = ["turing", "hinton", "lecun"];


// ---- pieces ---------------------------------------------------------------

function Face({ name, className }: { name: string; className?: string }) {
  return <MachineAvatar face={faces[name] ?? null} className={cn("size-list-face flex-none", className)} />;
}

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

/** A choice on the first screen: a row you press, glyph · words · ›. */
function Choice({ icon, title, note, onPick }: { icon: ReactNode; title: string; note: string; onPick?: () => void }) {
  return (
    <button
      type="button"
      onClick={onPick}
      className="flex w-full items-center gap-row rounded-md px-row py-in text-left hover:bg-fill"
    >
      <span className="flex size-ctl-md flex-none items-center justify-center text-muted-foreground [&_svg]:size-icon-mark">
        {icon}
      </span>
      <span className="flex min-w-0 flex-1 flex-col">
        <span className="text-headline text-foreground">{title}</span>
        <span className="text-callout text-muted-foreground">{note}</span>
      </span>
      <IconChevronRight className="size-icon-mark flex-none text-muted-foreground" />
    </button>
  );
}

type StepState = "done" | "live" | "wait" | "failed" | "undone";
/** One line of the install: its mark, its verb, what it did. */
function Step({ state, word, note }: { state: StepState; word: string; note?: string }) {
  return (
    <li data-step={state} className="flex items-start gap-row py-xxs">
      <span className="flex h-(--text-body--line-height) w-icon-mark flex-none items-center justify-center">
        {state === "done" ? (
          <IconCheck className="size-icon-mark text-foreground" />
        ) : state === "failed" ? (
          <IconClose className="size-icon-mark text-destructive" />
        ) : state === "live" ? (
          <span className="size-2 animate-pulse rounded-full bg-foreground motion-reduce:animate-none" />
        ) : (
          <span className="size-2 rounded-full border border-muted-foreground" />
        )}
      </span>
      <span className="flex min-w-0 flex-1 flex-col">
        <span
          className={cn(
            "text-body",
            state === "wait" || state === "undone" ? "text-muted-foreground" : "text-foreground",
            state === "undone" && "line-through",
          )}
        >
          {word}
        </span>
        {note && <span className="text-callout text-muted-foreground">{note}</span>}
      </span>
    </li>
  );
}

// ---- the screens ----------------------------------------------------------

function Pick() {
  return (
    <>
      <DialogHeader>
        <DialogTitle>添加机器</DialogTitle>
      </DialogHeader>
      <div className="-mx-row flex flex-col">
        <Choice icon={<IconDevices />} title="khor 主机" note="已安装 khor，使用邀请码添加" onPick={() => (location.hash = "#2")} />
        <Choice icon={<IconTerminal />} title="SSH 主机" note="通过 SSH 添加，自动安装 khor" onPick={() => (location.hash = "#3")} />
      </div>
      <DialogFooter>
        <Button>取消</Button>
      </DialogFooter>
    </>
  );
}

function AtHand() {
  return (
    <>
      <DialogHeader>
        <DialogTitle>khor 主机</DialogTitle>
      </DialogHeader>
      <FakeQr />
      <Input readOnly value="khor1qxy7…（邀请码）" aria-label="邀请码" />
      <div className="text-callout text-muted-foreground">15 分钟内有效，仅可使用一次</div>
      <div className="flex flex-col gap-in border-t pt-row">
        <span className="text-callout text-muted-foreground">已有其他机器的邀请码</span>
        <div className="flex items-center gap-in">
          <Input placeholder="输入邀请码" aria-label="输入邀请码" />
          <Button className="flex-none">加入</Button>
        </div>
      </div>
      <DialogFooter>
        <Button>关闭</Button>
        <Button variant="primary">复制邀请码</Button>
      </DialogFooter>
    </>
  );
}

/** 认证方式: 密钥 (the BatchMode road) or 密码 (kept in this machine's
    keychain only, handed to ssh through SSH_ASKPASS). */
function Auth({ auth, setAuth }: { auth: "key" | "password"; setAuth: (a: "key" | "password") => void }) {
  return (
    <>
      <FormLabel>认证方式</FormLabel>
      <SegmentedControl
        label="认证方式"
        value={auth}
        onChange={setAuth}
        options={[
          { key: "key", label: "密钥" },
          { key: "password", label: "密码" },
        ]}
      />
      {auth === "password" && (
        <>
          <FormLabel htmlFor="lab-pw">密码</FormLabel>
          <div className="flex min-w-0 flex-col gap-in">
            <Input id="lab-pw" type="password" defaultValue="placeholder" />
            <span className="text-callout text-muted-foreground">密码仅保存在本机钥匙串中</span>
          </div>
        </>
      )}
    </>
  );
}

function SshForm() {
  const [host, setHost] = useState("gpu-box");
  const [via, setVia] = useState("bastion");
  const [auth, setAuth] = useState<"key" | "password">("key");
  return (
    <>
      <DialogHeader>
        <DialogTitle>SSH 主机</DialogTitle>
      </DialogHeader>
      <FormGrid>
        <FormLabel htmlFor="lab-host">主机</FormLabel>
        <div className="flex min-w-0 flex-col gap-in">
          <Input id="lab-host" value={host} onChange={(e) => setHost(e.target.value)} placeholder="user@host 或 SSH 配置中的名称" />
          <div data-ssh-config-hosts className="flex flex-wrap gap-xxs">
            {sshHosts.map((h) => (
              <Button key={h} size="sm" variant={h === host ? "secondary" : "ghost"} onClick={() => setHost(h)}>
                {h}
              </Button>
            ))}
          </div>
        </div>
        <FormLabel htmlFor="lab-port">端口</FormLabel>
        <Input id="lab-port" inputMode="numeric" className="w-24" placeholder="22" aria-describedby="lab-port-note" />
        <FormLabel>跳板机</FormLabel>
        <PopupButton
          look="field"
          label="跳板机"
          value={via}
          onPick={(v) => (v === "?" ? (location.hash = "#4") : setVia(v))}
          choices={[
            { value: "", label: "直接连接" },
            ...khorMachines.map((m) => ({ value: `khor:${m}`, label: m, group: "通过 khor 主机" })),
            ...sshHosts.filter((h) => h !== host).map((h) => ({ value: h, label: h, group: "通过 SSH 跳板机" })),
            { value: "?", label: "检测哪些机器可以连接…", group: "连通性检测" },
          ]}
        />
        <FormLabel htmlFor="lab-name">名称</FormLabel>
        <Input id="lab-name" placeholder={host || "机器名称"} />
        <Auth auth={auth} setAuth={setAuth} />
      </FormGrid>
      <DialogFooter>
        <Button>取消</Button>
        <Button variant="primary">添加</Button>
      </DialogFooter>
    </>
  );
}

/** road-a's ReachReport, faked. */
type ReachRow = {
  id: string;
  shown: string;
  here: boolean;
  outcome: "reached" | "refused" | "timeout" | "no-name" | "unasked" | "predates";
  ms: number | null;
  addr: string | null;
  banner: string | null;
  host_key: string | null;
  why: string | null;
};
const report = {
  host: "gpu-box",
  port: 22,
  entrance: "c".repeat(64),
  disagree: true,
  known_as: null as string | null,
  rows: [
    { id: "a".repeat(64), shown: "MacLab", here: true, outcome: "timeout", ms: null, addr: null, banner: null, host_key: null, why: "连接超时" },
    { id: "c".repeat(64), shown: "turing", here: false, outcome: "reached", ms: 3, addr: "10.0.0.7:22", banner: "SSH-2.0-OpenSSH_9.6", host_key: "SHA256:q3Vf…9kLw", why: null },
    { id: "d".repeat(64), shown: "hinton", here: false, outcome: "reached", ms: 41, addr: "10.0.0.7:22", banner: "SSH-2.0-OpenSSH_9.6", host_key: "SHA256:q3Vf…9kLw", why: null },
    { id: "e".repeat(64), shown: "lecun", here: false, outcome: "reached", ms: 38, addr: "192.168.1.7:22", banner: "SSH-2.0-OpenSSH_8.9", host_key: "SHA256:Zt0p…aa1M", why: null },
  ] as ReachRow[],
};

function Reachers() {
  const best = report.rows.find((r) => r.id === report.entrance)!;
  const [pick, setPick] = useState(best.id);
  // The key most machines recorded; a row holding another is the odd one.
  const usual = "SHA256:q3Vf…9kLw";
  const chosen = report.rows.find((r) => r.id === pick)!;
  return (
    <>
      <DialogHeader>
        <DialogTitle>连通性检测</DialogTitle>
        <DialogDescription>每台在线的机器分别尝试连接 gpu-box 的 22 端口</DialogDescription>
      </DialogHeader>
      <ul data-reach className="-mx-row flex flex-col">
        {report.rows.map((r) => {
          const on = r.id === pick;
          const ok = r.outcome === "reached";
          const odd = report.disagree && r.host_key !== null && r.host_key !== usual;
          return (
            <li key={r.id}>
              <button
                type="button"
                disabled={!ok}
                onClick={() => setPick(r.id)}
                className={cn(
                  "grid w-full grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-x-row rounded-md px-row py-in text-left",
                  on ? "bg-fill-strong" : ok && "hover:bg-fill",
                )}
              >
                <Face name={r.shown} />
                <span className="flex min-w-0 flex-col">
                  <span className="truncate text-body text-foreground">
                    {r.shown}
                    {r.here && <span className={on ? "text-foreground" : "text-muted-foreground"}>（本机）</span>}
                  </span>
                  <span
                    className={cn(
                      "truncate text-caption",
                      ok && "font-mono",
                      odd ? "text-destructive" : on ? "text-foreground" : "text-muted-foreground",
                    )}
                  >
                    {ok ? r.host_key : r.why}
                  </span>
                </span>
                <span className={cn("text-callout tabular-nums", on ? "text-foreground" : "text-muted-foreground")}>
                  {ok ? `${r.ms} ms` : "无法连接"}
                  {r.id === report.entrance && <span className="ml-in">延迟最低</span>}
                </span>
              </button>
            </li>
          );
        })}
      </ul>
      {report.disagree && (
        <div className="text-callout text-destructive">lecun 记录的主机密钥与其他机器记录的不一致，可能不是同一台主机</div>
      )}
      <DialogFooter>
        <Button>返回</Button>
        <Button variant="primary">{chosen.here ? "直接连接" : `通过 ${chosen.shown}`}</Button>
      </DialogFooter>
    </>
  );
}

function Installing({ fallback = false }: { fallback?: boolean }) {
  return (
    <>
      <DialogHeader>
        <DialogTitle>添加 gpu-box</DialogTitle>
        <DialogDescription>通过 bastion</DialogDescription>
      </DialogHeader>
      {fallback ? (
        <>
          <ol data-steps className="flex flex-col">
            <Step state="done" word="连接" note="通过 bastion，34 ms" />
            <Step state="undone" word="安装 khor" note="已移除" />
            <Step state="failed" word="启动" note="没有 systemd --user，也没有 tmux" />
            <Step state="done" word="作为 SSH 主机添加" note="khor 未安装，操作通过 SSH 执行" />
          </ol>
          <div data-fallback-why className="text-body text-foreground">
            无法在 gpu-box 上持续运行 khor，已移除安装的文件，改为通过 SSH 连接
          </div>
        </>
      ) : (
        <ol data-steps className="flex flex-col">
          <Step state="done" word="连接" note="通过 bastion，34 ms" />
          <Step state="done" word="安装 khor" note="0.1.14，由目标主机自行下载" />
          <Step state="live" word="启动" note="systemd --user，退出登录后继续运行" />
          <Step state="wait" word="配对" />
        </ol>
      )}
      <DialogFooter>{fallback ? <Button>完成</Button> : <Button>取消</Button>}</DialogFooter>
    </>
  );
}

/** The key was refused: pick how to sign in. */
function AuthChoice() {
  const [auth, setAuth] = useState<"key" | "password">("key");
  return (
    <>
      <DialogHeader>
        <DialogTitle>gpu-box 不接受本机的密钥</DialogTitle>
      </DialogHeader>
      <FormGrid>
        <Auth auth={auth} setAuth={setAuth} />
      </FormGrid>
      {auth === "key" && (
        <>
          <div className="text-body text-foreground">将本机的公钥上传到 gpu-box，之后登录不再需要密码</div>
          <div data-selectable className="rounded-sm bg-fill px-row py-in font-mono text-mono text-foreground">
            ssh-copy-id -o ProxyJump=bastion gpu-box
          </div>
        </>
      )}
      <DialogFooter>
        <Button>取消</Button>
        <Button variant="primary">{auth === "key" ? "在终端中上传公钥" : "添加"}</Button>
      </DialogFooter>
    </>
  );
}

/** The machine page's 连接 cell and, when a road is broken, the segment. */
function Road({ machine, road, hops, broken, why, sat }: {
  machine: string;
  road: string;
  hops: string[];
  broken?: number;
  why?: string;
  sat?: boolean;
}) {
  return (
    <section className="flex flex-col gap-row border-b pb-base">
      <div className="flex items-center gap-row">
        <Face name={machine} className="size-avatar" />
        <span className="flex-1 text-title-sm text-foreground">{machine}</span>
        {sat && <Button>安装 khor…</Button>}
      </div>
      <dl className="grid grid-cols-4 gap-x-row">
        {[
          ["系统", "Linux"],
          ["连接", road],
          ["已运行", "12 天"],
          ["读数", "40 秒前"],
        ].map(([k, v]) => (
          <div key={k} className="flex min-w-0 flex-col border-l pl-row first:border-l-0 first:pl-0">
            <dt className="text-caption text-muted-foreground">{k}</dt>
            <dd className="truncate text-body text-foreground">{v}</dd>
          </div>
        ))}
      </dl>
      <div data-road className="flex flex-wrap items-center gap-in text-callout">
        {hops.map((h, i) => (
          <span key={h} className="flex items-center gap-in">
            {i > 0 && (
              <span
                aria-hidden
                className={cn("h-px w-8 flex-none", broken === i ? "border-t border-dashed border-destructive" : "bg-muted-foreground")}
              />
            )}
            <span className={broken !== undefined && i >= broken ? "text-destructive" : "text-foreground"}>{h}</span>
          </span>
        ))}
      </div>
      {why && <div className="text-callout text-destructive">{why}</div>}
    </section>
  );
}

function MachinePage() {
  return (
    <main className="mx-auto flex max-w-[44rem] flex-col gap-base px-pane pt-16 pb-pane">
      <Road machine="hinton" road="直连 · 31 ms" hops={["本机", "hinton"]} />
      <Road machine="lecun" road="中继 · 140 ms" hops={["本机", "中继 北京", "lecun"]} />
      <Road machine="turing" road="通过 hinton" hops={["本机", "hinton", "turing"]} />
      <Road
        machine="gpu-box"
        road="通过 bastion"
        sat
        hops={["本机", "bastion", "gpu-box"]}
        broken={2}
        why="跳板机无法连接目标主机：bastion 无法连接 gpu-box（Connection timed out）"
      />
      <Road
        machine="lab-gw"
        road="通过 bastion"
        sat
        hops={["本机", "bastion", "lab-gw"]}
        broken={1}
        why="本机无法连接跳板机 bastion（Connection refused）"
      />
    </main>
  );
}

// ---- the lab shell --------------------------------------------------------

const screens: [string, string, () => ReactNode][] = [
  ["1", "选择", () => <Pick />],
  ["2", "khor 主机", () => <AtHand />],
  ["3", "SSH 主机", () => <SshForm />],
  ["4", "连通性检测", () => <Reachers />],
  ["5", "安装中", () => <Installing />],
  ["6", "改为 SSH 连接", () => <Installing fallback />],
  ["7", "认证方式", () => <AuthChoice />],
  ["8", "机器页", () => null],
];

function Lab() {
  const [at, setAt] = useState(() => location.hash.slice(1) || "1");
  useEffect(() => {
    const on = () => setAt(location.hash.slice(1) || "1");
    window.addEventListener("hashchange", on);
    return () => window.removeEventListener("hashchange", on);
  }, []);
  const screen = screens.find(([k]) => k === at) ?? screens[0];
  const form = at === "3" || at === "7";
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
          <Button key={k} size="sm" variant={k === at ? "secondary" : "ghost"} onClick={() => (location.hash = `#${k}`)}>
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
          <DialogContent form={form} data-add-machine={at}>
            {screen[2]()}
          </DialogContent>
        </Dialog>
      )}
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(<Lab />);
