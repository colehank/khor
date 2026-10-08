// Lab only (lane ux-q2, 2026-10-08): the 额度 list row and the account's
// page, redrawn with fake data in the real list, faces, dial and bar.
// Nothing here talks to a node. `?dark` / `?light` picks the theme; on a
// phone the list opens a page and ‹ comes back, as the shell does.
//
// The owner (2026-10-08): 「左下写清楚多久重置，右上干脆只写百分比，然后
// 右侧一个沾满高度的环……最右还是上状态下时间，最右的左侧贴环」;
// 「点击去后，窗口简洁一点，小时用H，天用D……点击去不要环和bar换行」;
// 历史默认 30D; 中位线看得清; 非订阅的 model 各有颜色。
import { useEffect, useState, type ReactNode } from "react";
import ReactDOM from "react-dom/client";

import "@/styles/tokens.css";
import "@/styles/app.css";
import "./quota.css";

import { Bar } from "@/components/Bar";
import { sessionMark } from "@/components/BrandIcons";
import { DetailFrame } from "@/components/DetailFrame";
import { DIAL_CARD_SIZE, DIAL_CARD_STROKE, Face, type Pip } from "@/components/Face";
import { IconBack, IconReload } from "@/components/icons";
import { providerKind } from "@/components/SessionFace";
import { SideList, type SideRow } from "@/components/SideList";
import { TooltipProvider } from "@/components/ui/tooltip";
import { NARROW_QUERY } from "@/hooks/use-narrow";
import { gaugeColor } from "@/lib/vitals";
import { cn } from "@/lib/utils";
import type { Strain } from "@/api";

import { boardBands } from "./quota-bands";
import { UsageHalf, type UsageRow } from "./quota-usage";

// ---- fixtures -------------------------------------------------------------

const NOW = Date.now();
const MIN = 60_000;
const H = 60 * MIN;
const D = 24 * H;

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
  turing: av("#2a4f6b", "#8abeb7", "#81a2be"),
  hinton: av("#6b3a2a", "#f0c674", "#cc6666"),
  lecun: av("#4a4a4a", "#b294bb", "#969896"),
  MacLab: av("#5f6b2a", "#c5c8a6", "#a6b87a"),
};

/** One line on the account's page: a window, a budget or extra usage. */
type Line = { id: string; word: string; pct: number; resets: number | null };

type Acct = {
  id: string;
  name: string;
  provider: string;
  kind: "sub" | "bill";
  home: string;
  plan: string | null;
  /** How old the reading is, ms; `null` = read here, now. */
  age: number | null;
  lines: Line[];
  /** What a line cannot say: money, as figures under the lines. */
  money?: { word: string; value: string }[];
  /** No fraction at all (a wallet): the row's right-top says this. */
  balance?: string;
  trouble?: string;
  usage?: UsageRow[];
};

/** Deterministic noise, so the screenshots do not change between runs. */
function noise(seed: number) {
  let s = seed;
  return () => {
    s = (s * 1103515245 + 12345) % 2147483648;
    return s / 2147483648;
  };
}
const dayOf = (back: number) => new Date(NOW - back * D).toISOString().slice(0, 10);

function spend(models: { id: string; base: number; from: number; to?: number }[], seed: number, days = 64): UsageRow[] {
  const r = noise(seed);
  const out: UsageRow[] = [];
  for (let back = days; back >= 0; back -= 1) {
    const weekend = new Date(NOW - back * D).getDay() % 6 === 0;
    for (const m of models) {
      if (back > m.from || back < (m.to ?? 0)) continue;
      if (r() < (weekend ? 0.55 : 0.12)) continue;
      const v = m.base * (0.25 + 1.6 * r() * r());
      out.push({
        day: dayOf(back),
        model: m.id,
        input: Math.round(v * 0.08),
        output: Math.round(v * 0.04),
        cached_input: Math.round(v * 0.8),
        cache_write: Math.round(v * 0.08),
      });
    }
  }
  return out;
}

const ACCOUNTS: Acct[] = [
  {
    id: "max",
    name: "Claude Max 20x · 实验室共用订阅",
    provider: "claude",
    kind: "sub",
    home: "turing",
    plan: "Max 20x",
    age: 3 * MIN,
    lines: [
      { id: "5h", word: "5H", pct: 91, resets: NOW + 2 * H + 14 * MIN },
      { id: "7d", word: "7D", pct: 100, resets: NOW + 3 * D + 4 * H },
      { id: "7d-opus", word: "7D · Opus", pct: 64, resets: NOW + 3 * D + 4 * H },
      { id: "credits", word: "额外用量", pct: 25, resets: null },
    ],
    money: [{ word: "额外用量", value: "$12.40 / $50.00" }],
    usage: spend(
      [
        { id: "claude-opus-4-8", base: 9e7, from: 64, to: 20 },
        { id: "claude-opus-5", base: 1.2e8, from: 20 },
        { id: "claude-fable-5-1", base: 6e7, from: 30 },
        { id: "claude-sonnet-5", base: 3e7, from: 64 },
        { id: "claude-haiku-4-5-20251001", base: 1e7, from: 64 },
      ],
      7,
    ),
  },
  {
    id: "codex",
    name: "Codex Pro",
    provider: "codex",
    kind: "sub",
    home: "hinton",
    plan: "Pro",
    age: 41 * MIN,
    lines: [
      { id: "5h", word: "5H", pct: 46, resets: NOW + 45 * MIN },
      { id: "7d", word: "7D", pct: 72, resets: NOW + 5 * D + 9 * H },
    ],
  },
  {
    id: "openrouter",
    name: "OpenRouter 研究组共享密钥（按月预算，别外传）",
    provider: "openrouter",
    kind: "bill",
    home: "lecun",
    plan: null,
    age: 12 * MIN,
    lines: [{ id: "budget", word: "本月预算", pct: 71, resets: NOW + 23 * D + 6 * H }],
    money: [
      { word: "已花", value: "$142.30 / $200.00" },
      { word: "余额", value: "$57.70" },
    ],
    usage: spend(
      [
        { id: "deepseek/deepseek-chat-v3.2", base: 4e7, from: 64 },
        { id: "openai/gpt-5.1-codex", base: 6e7, from: 50 },
        { id: "moonshotai/kimi-k2-0905", base: 2.5e7, from: 64 },
        { id: "qwen/qwen3-coder-plus", base: 1.5e7, from: 40 },
      ],
      11,
    ),
  },
  {
    id: "deepseek",
    name: "DeepSeek 个人钱包",
    provider: "deepseek",
    kind: "bill",
    home: "MacLab",
    plan: null,
    age: 2 * H,
    lines: [],
    balance: "¥42.10",
    money: [{ word: "余额", value: "¥42.10" }],
  },
  {
    id: "moonshot",
    name: "Moonshot",
    provider: "moonshot",
    kind: "bill",
    home: "MacLab",
    plan: null,
    age: 6 * H,
    lines: [],
    trouble: "无法获取余量",
  },
];

// Colours over the whole board, so the same id is the same colour on
// every page (`lab/quota-bands`).
const BANDS = boardBands(ACCOUNTS.flatMap((a) => (a.usage ?? []).map((r) => r.model)));

// ---- the words this draft proposes ---------------------------------------

// WORDS-1008 (the owner: 「MDH 这个只在需要缩写的地方……普通的还是 2 秒前
// 这样。重置如 2 小时 13 分后重置」): times in words with their direction;
// M / H / D only in the window names. Drafts of what lane afix-a's shared
// formatter will give.

/** How long until a moment: two units where the second matters, never
    rounded down to one — 「2 小时 13 分」「3 天 4 小时」; minutes alone are
    「45 分钟」 (the lead, 2026-10-08). */
function ahead(ms: number): string {
  const m = Math.max(1, Math.floor(ms / MIN));
  if (m < 60) return `${m} 分钟`;
  const h = Math.floor(m / 60);
  if (h < 24) return m % 60 ? `${h} 小时 ${m % 60} 分` : `${h} 小时`;
  const d = Math.floor(h / 24);
  return h % 24 ? `${d} 天 ${h % 24} 小时` : `${d} 天`;
}

/** How long ago, one unit: 「2 秒前」「3 分钟前」「4 小时前」「2 天前」. */
function ago(ms: number): string {
  const s = Math.max(0, Math.floor(ms / 1000));
  if (s < 60) return `${s} 秒前`;
  const m = Math.floor(s / 60);
  if (m < 60) return `${m} 分钟前`;
  const h = Math.floor(m / 60);
  if (h < 24) return `${h} 小时前`;
  return `${Math.floor(h / 24)} 天前`;
}

/** 90 / 95, the node's thresholds (`khor_core::Strain`). */
const strainOf = (pct: number): Strain | null => (pct >= 95 ? "critical" : pct >= 90 ? "tight" : null);
/** The line the row speaks for: the fullest one with a reset (the one
    that stops the work first). */
const tightest = (a: Acct): Line | null =>
  a.lines.filter((l) => l.id !== "credits").reduce<Line | null>((b, l) => (!b || l.pct > b.pct ? l : b), null);

// ---- the row's ring -------------------------------------------------------

/**
 * **The row's ring, as tall as the row's two lines** — the owner's 「右侧
 * 一个沾满高度的环」. A whole circle from twelve o'clock (the lead: a full
 * ring, not the dial's 210°). The ink is the bar's and the
 * dial's (`gaugeColor`): grey, then `--strain` from 90%.
 */
function RowRing({ pct, strain }: { pct: number | null; strain: Strain | null }) {
  const S = 36;
  const W = 3.6;
  const R = S / 2 - W / 2 - 0.5;
  const f = pct === null ? null : Math.max(0, Math.min(100, pct));
  return (
    <span data-quota-row-ring className="flex size-(--quota-ring) flex-none">
      <svg viewBox={`0 0 ${S} ${S}`} className="size-full" aria-hidden="true">
        <circle cx={S / 2} cy={S / 2} r={R} fill="none" stroke="currentColor" strokeOpacity={0.16} strokeWidth={W} />
        {f !== null && f >= 100 && <circle cx={S / 2} cy={S / 2} r={R} fill="none" stroke={gaugeColor(strain)} strokeWidth={W} />}
        {f !== null && f > 0 && f < 100 && (
          <circle
            cx={S / 2}
            cy={S / 2}
            r={R}
            fill="none"
            stroke={gaugeColor(strain)}
            strokeWidth={W}
            strokeLinecap="round"
            pathLength={100}
            strokeDasharray={`${f} 100`}
            transform={`rotate(-90 ${S / 2} ${S / 2})`}
          />
        )}
      </svg>
    </span>
  );
}

const ROW_GLYPH = new URLSearchParams(location.search).get("row") === "glyph";

function rowOf(a: Acct, on: boolean, open: () => void): SideRow {
  const t = tightest(a);
  const strain = t ? strainOf(t.pct) : null;
  const over = !!t && t.pct >= 100;
  return {
    id: a.id,
    group: "",
    pinned: false,
    face: { machine: faces[a.home] ?? null },
    mark: providerKind(a.provider),
    title: a.name,
    name: a.name,
    // Left-bottom: how long until the line the row speaks for resets.
    // Two drafts for the owner to pick (`?row=glyph`): the words, or ↻
    // as on the account page.
    sub: t?.resets ? (
      ROW_GLYPH ? <Resets ms={t.resets - NOW} /> : <span className="tabular-nums">{`${ahead(t.resets - NOW)}后重置`}</span>
    ) : undefined,
    // Left of the rightmost column: the ring, both lines tall. A trouble
    // has no reading to draw.
    // A wallet has no fraction and draws none (the lead, 2026-10-08).
    trailing: a.trouble || !t ? undefined : <RowRing pct={t.pct} strain={strain} />,
    corners: {
      // Right-top: only the percentage (a wallet: its balance; a trouble:
      // the word). Over the limit, the strain ink and weight say so.
      top: a.trouble ? (
        <span>{a.trouble}</span>
      ) : t ? (
        // Kept whole (`data-corner-keep`) and as wide as 「100%」 in the over weight, so
        // the rings of every row stand on one line.
        <span data-corner-keep className={cn("inline-block min-w-[5ch] text-right tabular-nums", over ? "font-semibold text-strain" : "text-foreground")}>{`${t.pct}%`}</span>
      ) : (
        <span data-corner-keep className="tabular-nums text-foreground">{a.balance}</span>
      ),
      // Right-bottom: the reading's age, as every list's.
      // At least 「59 分钟前」 wide, so the column — and the ring beside it
      // — stands at one x on every row whatever the age says.
      bottom: a.age === null ? undefined : <span className="inline-block min-w-[4.6em] text-right">{ago(a.age)}</span>,
    },
    find: a.name,
    selected: on,
    menu: [],
    onOpen: open,
  };
}

// ---- the account's page ---------------------------------------------------

/** **↻ and the time** in secondary ink at the text's size; the words
    stay for a reader that cannot see the glyph. */
function Resets({ ms }: { ms: number }) {
  const said = `${ahead(ms)}后重置`;
  return (
    <span data-quota-resets role="img" aria-label={said} title={said} className="inline-flex items-center gap-xxs text-callout text-muted-foreground tabular-nums">
      <IconReload className="size-[1em] flex-none" />
      <span aria-hidden="true">{ahead(ms)}</span>
    </span>
  );
}

function Lines({ a }: { a: Acct }) {
  if (a.lines.length === 0) return null;
  // The owner (2026-10-08): 「重置写在 5H 这种左侧的标题旁边」, then 「重制
  // 用一个刷新 icon，不必写俩字」 — ↻ and the time ride the window's title in
  // secondary ink; no reset column, and no column heads (a bar filled to
  // the number beside it says 「used」).
  return (
    <div data-quota-lines className="grid min-w-0 flex-1 grid-cols-[auto_minmax(1.5rem,1fr)_auto] items-center gap-x-in gap-y-in">
      {a.lines.map((l) => {
        const s = strainOf(l.pct);
        const over = l.pct >= 100;
        return (
          <div key={l.id} data-quota-line={l.id} className="contents">
            <span data-quota-line-title className="flex items-baseline gap-in whitespace-nowrap">
              <span className="text-body">{l.word}</span>
              {l.resets && <Resets ms={l.resets - NOW} />}
            </span>
            <Bar pct={l.pct} strain={l.id === "credits" ? null : s} />
            <span className={cn("text-right text-body tabular-nums whitespace-nowrap", over && "font-semibold text-strain")}>
              {`${l.pct}%`}
            </span>
          </div>
        );
      })}
    </div>
  );
}

function Page({ a, narrow, onBack }: { a: Acct; narrow: boolean; onBack?: () => void }) {
  const t = tightest(a);
  const Mark = sessionMark("agent", a.provider);
  const caption = [a.plan, `${a.home} · ${a.age === null ? "刚刚" : ago(a.age)}`].filter(Boolean).join(" · ");
  const pips: Pip[] = a.lines.filter((l) => l.id !== "credits").map((l) => ({ word: l.word, v: l.pct }));
  // **No dial on a phone** (the lead's A, 2026-10-08): each window's own
  // line carries its number, and the dial's room is the bar's. Wide keeps
  // it — it has the room and it is the page's one picture.
  return (
    <div data-quota-page className="flex min-h-0 min-w-0 flex-1 flex-col">
      {/* R0, drawn plainly for the lab: the vendor's mark, the name, and
          under it the plan and where / when it was read. */}
      <header className="flex min-h-list-row flex-none items-center gap-row border-b px-page">
        {narrow && (
          <button type="button" onClick={onBack} className="-ms-in flex size-touch items-center justify-center" aria-label="返回">
            <IconBack className="size-icon" />
          </button>
        )}
        {Mark && <Mark className="block size-head-ctl flex-none" />}
        <span className="flex min-w-0 flex-col">
          <span className="truncate text-title-sm font-semibold">{a.name}</span>
          <span className="truncate text-callout text-muted-foreground tabular-nums">{caption}</span>
        </span>
      </header>
      <div className="min-h-0 flex-1 overflow-y-auto">
        <DetailFrame className="flex flex-col gap-base py-row">
          {a.trouble ? (
            <p className="text-body">{a.trouble}</p>
          ) : (
            <div data-quota-lead className="flex flex-nowrap items-center gap-x-row">
              {!narrow && (
                <Face
                  size={DIAL_CARD_SIZE}
                  stroke={DIAL_CARD_STROKE}
                  ratio={1}
                  crop
                  fraction={t ? t.pct / 100 : null}
                  strain={t ? strainOf(t.pct) : null}
                  pips={pips.length > 1 ? pips : []}
                >
                  {t ? `${t.pct}%` : a.balance}
                </Face>
              )}
              <Lines a={a} />
              {a.lines.length === 0 && a.money && (
                <dl className="grid grid-cols-[auto_1fr] gap-x-row gap-y-1 text-callout">
                  {a.money.map((m) => (
                    <div key={m.word} className="contents">
                      <dt className="text-muted-foreground">{m.word}</dt>
                      <dd className="tabular-nums">{m.value}</dd>
                    </div>
                  ))}
                </dl>
              )}
            </div>
          )}
          {a.lines.length > 0 && a.money && (
            <p className="flex flex-wrap gap-x-base text-callout text-muted-foreground tabular-nums">
              {a.money.map((m) => (
                <span key={m.word} className="whitespace-nowrap">
                  {m.word} <span className="text-foreground">{m.value}</span>
                </span>
              ))}
            </p>
          )}
          {a.usage && (
            <UsageHalf
              rows={a.usage}
              why={null}
              today={dayOf(0)}
              cacheKnown
              source={a.kind === "sub" ? "transcript" : "vendor"}
              bands={BANDS}
            />
          )}
        </DetailFrame>
      </div>
    </div>
  );
}

// ---- the lab --------------------------------------------------------------

function useNarrowStamp(): boolean {
  const [narrow, setNarrow] = useState(() => window.matchMedia(NARROW_QUERY).matches);
  useEffect(() => {
    const m = window.matchMedia(NARROW_QUERY);
    const stamp = () => {
      document.documentElement.toggleAttribute("data-narrow", m.matches);
      setNarrow(m.matches);
    };
    stamp();
    m.addEventListener("change", stamp);
    return () => m.removeEventListener("change", stamp);
  }, []);
  return narrow;
}

function Lab() {
  const narrow = useNarrowStamp();
  const [picked, setPicked] = useState<string>(() => location.hash.slice(1) || "max");
  const [screen, setScreen] = useState<"list" | "detail">(() => (location.hash.length > 1 ? "detail" : "list"));
  useEffect(() => {
    const on = () => {
      const h = location.hash.slice(1);
      if (h) setPicked(h);
      setScreen(h ? "detail" : "list");
    };
    window.addEventListener("hashchange", on);
    return () => window.removeEventListener("hashchange", on);
  }, []);
  const current = ACCOUNTS.find((a) => a.id === picked) ?? ACCOUNTS[0];
  const list: ReactNode = (
    <SideList
      rows={ACCOUNTS.map((a) => rowOf(a, !narrow && a.id === current.id, () => (location.hash = a.id)))}
      label={() => ""}
    />
  );
  if (narrow) {
    return (
      <div className="flex h-dvh flex-col bg-background text-foreground">
        {screen === "list" ? (
          <>
            <header className="flex min-h-list-row flex-none items-center px-group text-title-sm font-semibold">额度</header>
            <div className="min-h-0 flex-1 overflow-y-auto">{list}</div>
          </>
        ) : (
          <Page a={current} narrow onBack={() => history.back()} />
        )}
      </div>
    );
  }
  return (
    <div className="flex h-dvh bg-background text-foreground">
      <aside className="flex w-list flex-none flex-col border-e">
        <header className="flex min-h-list-row flex-none items-center px-row text-title-sm font-semibold">额度</header>
        <div className="min-h-0 flex-1 overflow-y-auto">{list}</div>
      </aside>
      <Page a={current} narrow={false} />
    </div>
  );
}

ReactDOM.createRoot(document.getElementById("root")!).render(
  <TooltipProvider>
    <Lab />
  </TooltipProvider>,
);
