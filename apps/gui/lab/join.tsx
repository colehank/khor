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

/** One machine's answer to 「够得着 gpu-box:22 吗」 (road-a's probe; the
    field names are a draft until agreed through the lead). */
type Reach = { machine: string; me?: boolean; ok: boolean; ms?: number; key?: string; why?: string; odd?: boolean };
const reach: Reach[] = [
  { machine: "MacLab", me: true, ok: false, why: "超时" },
  { machine: "turing", ok: true, ms: 3, key: "SHA256:q3Vf…9kLw" },
  { machine: "hinton", ok: true, ms: 41, key: "SHA256:q3Vf…9kLw" },
  { machine: "lecun", ok: true, ms: 38, key: "SHA256:Zt0p…aa1M", odd: true },
];

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
        <Choice
          icon={<IconDevices />}
          title="手边的机器"
          note="它已经装了 khor：扫码，或者输入邀请码"
          onPick={() => (location.hash = "#2")}
        />
        <Choice
          icon={<IconTerminal />}
          title="ssh 主机"
          note="经 ssh 给它装上 khor，不用走到它跟前"
          onPick={() => (location.hash = "#3")}
        />
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
        <DialogTitle>手边的机器</DialogTitle>
      </DialogHeader>
      <FakeQr />
      <Input readOnly value="khor1qxy7…（邀请码，一次有效）" aria-label="邀请码" />
      <div className="text-callout text-muted-foreground">15 分钟内有效，用过一次就作废</div>
      <div className="flex flex-col gap-in border-t pt-row">
        <span className="text-callout text-muted-foreground">反过来，对方给了你邀请码</span>
        <div className="flex items-center gap-in">
          <Input placeholder="粘贴邀请码" aria-label="粘贴邀请码" />
          <Button className="flex-none">加入</Button>
        </div>
      </div>
      <DialogFooter>
        <Button>关闭</Button>
        <Button variant="primary">拷贝邀请码</Button>
      </DialogFooter>
    </>
  );
}

function SshForm() {
  const [host, setHost] = useState("gpu-box");
  const [via, setVia] = useState("bastion");
  const [kind, setKind] = useState<"member" | "satellite">("member");
  return (
    <>
      <DialogHeader>
        <DialogTitle>ssh 主机</DialogTitle>
      </DialogHeader>
      <FormGrid>
        <FormLabel htmlFor="lab-host">主机</FormLabel>
        <div className="flex min-w-0 flex-col gap-in">
          <Input id="lab-host" value={host} onChange={(e) => setHost(e.target.value)} placeholder="user@host 或 ssh 配置里的名字" />
          {/* The user's own ssh config, as names to press. */}
          <div data-ssh-config-hosts className="flex flex-wrap gap-xxs">
            {sshHosts.map((h) => (
              <Button key={h} size="sm" variant={h === host ? "secondary" : "ghost"} onClick={() => setHost(h)}>
                {h}
              </Button>
            ))}
          </div>
        </div>
        <FormLabel>经由</FormLabel>
        <PopupButton
          look="field"
          label="经由"
          value={via}
          onPick={(v) => (v === "?" ? (location.hash = "#4") : setVia(v))}
          choices={[
            { value: "", label: "直接连" },
            ...khorMachines.map((m) => ({ value: `khor:${m}`, label: m, group: "从 khor 机器添加" })),
            ...sshHosts.filter((h) => h !== host).map((h) => ({ value: h, label: h, group: "经 ssh 跳板" })),
            { value: "?", label: "问问每台机器谁够得着…", group: "不知道" },
          ]}
        />
        <FormLabel htmlFor="lab-name">名字</FormLabel>
        <Input id="lab-name" placeholder={host || "机器名"} />
        <FormLabel>方式</FormLabel>
        <div className="flex min-w-0 flex-col gap-in">
          <SegmentedControl
            label="方式"
            value={kind}
            onChange={setKind}
            options={[
              { key: "member", label: "成员" },
              { key: "satellite", label: "卫星" },
            ]}
          />
          <span className="text-callout text-muted-foreground">
            {kind === "member"
              ? "装上 khor，它自己连网；留不住时改作卫星"
              : "什么都不装，本机经 ssh 替它跑"}
          </span>
        </div>
      </FormGrid>
      <DialogFooter>
        <Button>取消</Button>
        <Button variant="primary">添加</Button>
      </DialogFooter>
    </>
  );
}

function Reachers() {
  const best = reach.filter((r) => r.ok).sort((a, b) => (a.ms ?? 0) - (b.ms ?? 0))[0];
  const [pick, setPick] = useState(best.machine);
  const odd = reach.find((r) => r.odd);
  return (
    <>
      <DialogHeader>
        <DialogTitle>谁够得着 gpu-box</DialogTitle>
        <DialogDescription>每台在线的机器各试一次 22 端口</DialogDescription>
      </DialogHeader>
      <ul data-reach className="-mx-row flex flex-col">
        {reach.map((r) => {
          const on = r.machine === pick;
          return (
            <li key={r.machine}>
              <button
                type="button"
                disabled={!r.ok}
                onClick={() => setPick(r.machine)}
                className={cn(
                  "grid w-full grid-cols-[auto_minmax(0,1fr)_auto] items-center gap-x-row rounded-md px-row py-in text-left",
                  on ? "bg-fill-strong" : r.ok && "hover:bg-fill",
                )}
              >
                <Face name={r.machine} />
                <span className="flex min-w-0 flex-col">
                  <span className="truncate text-body text-foreground">
                    {r.machine}
                    {r.me && <span className={on ? "text-foreground" : "text-muted-foreground"}>（本机）</span>}
                  </span>
                  <span
                    className={cn(
                      "truncate font-mono text-caption",
                      r.odd ? "text-destructive" : on ? "text-foreground" : "text-muted-foreground",
                    )}
                  >
                    {r.ok ? r.key : r.why}
                  </span>
                </span>
                <span className={cn("text-callout tabular-nums", on ? "text-foreground" : "text-muted-foreground")}>
                  {r.ok ? `${r.ms} ms` : "够不着"}
                  {r === best && <span className="ml-in">最快</span>}
                </span>
              </button>
            </li>
          );
        })}
      </ul>
      {odd && (
        <div className="text-callout text-destructive">{odd.machine} 认的钥匙和别的机器不同，它够到的可能不是同一台</div>
      )}
      <DialogFooter>
        <Button>返回</Button>
        <Button variant="primary">{pick === "MacLab" ? "直接连" : `经 ${pick}`}</Button>
      </DialogFooter>
    </>
  );
}

function Installing({ fallback = false }: { fallback?: boolean }) {
  return (
    <>
      <DialogHeader>
        <DialogTitle>添加 gpu-box</DialogTitle>
        <DialogDescription>经 bastion</DialogDescription>
      </DialogHeader>
      {fallback ? (
        <>
          <ol data-steps className="flex flex-col">
            <Step state="done" word="连接" note="经 bastion，34 ms" />
            <Step state="undone" word="放 khor" note="已撤掉" />
            <Step state="failed" word="启动" note="没有 systemd --user，也没有 tmux" />
            <Step state="done" word="作为卫星添加" note="本机经 ssh 替它跑" />
          </ol>
          <div data-fallback-why className="text-body text-foreground">
            gpu-box 留不住常驻的 khor，已撤掉装上的东西，改作卫星
          </div>
        </>
      ) : (
        <ol data-steps className="flex flex-col">
          <Step state="done" word="连接" note="经 bastion，34 ms" />
          <Step state="done" word="放 khor" note="0.1.14，它自己从 GitHub 下载" />
          <Step state="live" word="启动" note="systemd --user，退出登录后照常运行" />
          <Step state="wait" word="配对" />
        </ol>
      )}
      <DialogFooter>
        {fallback ? <Button>完成</Button> : <Button>取消</Button>}
      </DialogFooter>
    </>
  );
}

function PasswordOnly() {
  return (
    <>
      <DialogHeader>
        <DialogTitle>gpu-box 只认密码</DialogTitle>
      </DialogHeader>
      <div className="text-body text-foreground">khor 不经手密码。先把本机的钥匙放到 gpu-box 上，再回来添加</div>
      <div data-selectable className="rounded-sm bg-fill px-row py-in font-mono text-mono text-foreground">
        ssh-copy-id -J bastion gpu-box
      </div>
      <div className="text-callout text-muted-foreground">在终端里输一次密码，以后就不用了</div>
      <DialogFooter>
        <Button>取消</Button>
        <Button variant="primary">在终端里放钥匙</Button>
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
        {sat && <Button>转正…</Button>}
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
      {/* The road, machine by machine; a broken segment says so in ssh's words. */}
      <div data-road className="flex flex-wrap items-center gap-in text-callout">
        {hops.map((h, i) => (
          <span key={h} className="flex items-center gap-in">
            {i > 0 && (
              <span
                aria-hidden
                className={cn(
                  "h-px w-8 flex-none",
                  broken === i ? "border-t border-dashed border-destructive" : "bg-muted-foreground",
                )}
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
      <Road machine="turing" road="经 hinton" hops={["本机", "hinton", "turing"]} />
      <Road
        machine="gpu-box"
        road="经 bastion"
        sat
        hops={["本机", "bastion", "gpu-box"]}
        broken={2}
        why="跳板到目标不通：bastion 连不上 gpu-box（Connection timed out）"
      />
      <Road
        machine="lab-gw"
        road="经 bastion"
        sat
        hops={["本机", "bastion", "lab-gw"]}
        broken={1}
        why="本机到跳板不通：连不上 bastion（Connection refused）"
      />
    </main>
  );
}

// ---- the lab shell --------------------------------------------------------

const screens: [string, string, () => ReactNode][] = [
  ["1", "选择", () => <Pick />],
  ["2", "手边", () => <AtHand />],
  ["3", "ssh", () => <SshForm />],
  ["4", "谁够得着", () => <Reachers />],
  ["5", "安装中", () => <Installing />],
  ["6", "改作卫星", () => <Installing fallback />],
  ["7", "只认密码", () => <PasswordOnly />],
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
  const form = at === "3";
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
