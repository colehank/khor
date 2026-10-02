import { useCallback, useEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from "react";

import {
  fetchDevices,
  fetchHooks,
  fetchSessions,
  fetchWorkspaces,
  installHooks,
  launchOn,
  closeSession,
  forgetDevice,
  markSeen,
  onFullscreen,
  onQuitRequested,
  pinDevice,
  quitNow,
  pinSession,
  setMachinePaused,
  systemUnlock,
  trafficLights,
  trafficRoom,
  uninstallHooks,
  wsAdd,
  type DeviceRow,
  type HooksState,
  type SessionRow,
  type WorkspaceRow,
} from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import {
  IconAppearance,
  IconBrowser,
  IconChevronRight,
  IconClose,
  IconDevices,
  IconFiles,
  IconGear,
  IconNetwork,
  IconPulse,
  IconQuota,
  IconSessions,
  IconShield,
} from "@/components/icons";
import { CloseDraftDialog } from "@/components/CloseDraftDialog";
import { DirPicker } from "@/components/DirPicker";
import { FloatNav, type NavSeat } from "@/components/FloatNav";
import { LandingStrip } from "@/components/LandingStrip";
import { ListPane } from "@/components/ListPane";
import { type OmniAxis } from "@/components/Omnibox";
import { useCodeWindow } from "@/components/CodeWindow";
import { HereHostsProvider, hereHosts } from "@/lib/here";
import { PaneBar, type PaneAction } from "@/components/PaneBar";
import { StatusBar } from "@/components/StatusBar";
import { GlassCircle } from "@/components/Glass";
import { TooltipProvider } from "@/components/ui/tooltip";
import { BrowseLanding } from "@/browse/BrowseLanding";
import { TabCount, TabSwitcher, type TabFace } from "@/browse/TabStrip";
import { Tabs } from "@/browse/Tabs";
import { useBrowseTabs } from "@/browse/use-browse-tabs";
import { gui } from "@/gen/catalog";
import { setDayShown, useDayShown } from "@/hooks/use-day-shown";
import { useDayStrips, useStripWidth } from "@/hooks/use-day-strips";
import { HIDDEN_MS, useHidden } from "@/hooks/use-hidden";
import { useWoken } from "@/hooks/use-woken";
import { LIST_PANE, roomBeside, useListWidth } from "@/hooks/use-list-width";
import { useSessionTabs } from "@/hooks/use-session-tabs";
import { tabsLimitOf } from "@/browse/tab-limit";
import { useLock } from "@/hooks/use-lock";
import { useStoredSettings } from "@/hooks/use-settings";
import { SettingsProvider, SettingsRereadProvider } from "@/settings";
import { speakLang, type LangPref } from "@/lang";
import { wearTheme, type ThemePref } from "@/theme";
import { useNarrow } from "@/hooks/use-narrow";
import { useStatusBar } from "@/hooks/use-status-bar";
import {
  arrange,
  rememberArrange,
  rememberWords,
  sessionAxes as axesOf,
  sessionOptions,
  storedArrange,
  storedWords,
} from "@/lib/session-filter";
import { draftCount, watchDrafts } from "@/lib/drafts";
import { NAV_SHOWN, useNavOrder } from "@/lib/nav-order";
import { cn } from "@/lib/utils";
import { DAY_ROW_ID, daySwitch, type Day } from "@/views/DayStrip";
import { DetailPane } from "@/views/DetailPane";
import { holding, LinkHomeProvider, useKnownFiles, type LinkHome } from "@/views/linkHome";
import { PAGES } from "@/views/PagesKind";
import { DevicesList, useDeviceFilter } from "@/views/DevicesList";
import { WorkspacePane } from "@/views/WorkspacePane";
import { WorkspacesList, useWorkspaceFilter } from "@/views/WorkspacesList";
import { WorkspacesNow } from "@/views/WorkspacesNow";
import { LockScreen } from "@/views/LockScreen";
import { moreName, morePageOrder, type MorePage } from "@/views/more/MorePage";
import { AppearancePage } from "@/views/more/AppearancePage";
import { DoctorPage } from "@/views/more/DoctorPage";
import { NetworkPage } from "@/views/more/NetworkPage";
import { IdentityPage } from "@/views/more/IdentityPage";
import { SecurityPage } from "@/views/more/SecurityPage";
import { SettingsPage } from "@/views/more/SettingsPage";
import { MachineCard, MachineVerbs } from "@/views/MachineCard";
import { MachineFiles } from "@/views/MachineFiles";
import { MachineWeb } from "@/views/MachineWeb";
import { RegionState } from "@/components/RegionState";
import { MandalaMap, waitingOn } from "@/views/MandalaMap";
import { MeshNow } from "@/views/MeshNow";
import { InviteDialog, JoinDialog } from "@/views/PairDialogs";
import { QuotaLanding } from "@/views/QuotaLanding";
import { SessionsList } from "@/views/SessionsList";
import { Region, RegionLead } from "@/components/Region";
import { WINDOW_DRAG } from "@/components/shell";
import { FIND_ALL, NarrowHead, SideHead, SideSeat, SideSeatsProvider, useSideSeats } from "@/components/SideHead";
import { SessionScreen } from "@/components/SessionBar";
import { useShortcuts, useTabHost, walkList } from "@/lib/shortcuts";
import { ImportDialog } from "@/views/ImportDialog";
import { NewSessionDialog } from "@/views/NewSessionDialog";

/**
 * The four landings, all present from the first day.
 *
 * Every one begins with the same question the devices pane answers —
 * which machine — and grows its own second step from there: a machine's
 * card, its disk, or its network. Files opens a directory; browser
 * borrows the network and opens a page through it (docs/NET.md 借网).
 */
type Landing = "sessions" | "devices" | "workspaces" | "browser" | "quota";

/*
 * **The mark's place is gone as a screen of its own** (2026-09-20).
 * It was a state beside `landing` — the mesh, reached first from the
 * app's mark at the head of the rail, then from this machine's face,
 * then from a row on the identity card. It was the body of the 网络
 * page from then until 2026-09-28, when the owner put the map on one
 * surface — the machines landing's overview (10-layout 「机器 → 那张
 * 图」) — and made 网络 a page of settings rows. `MarkPlace` and the
 * card it opened over the map went with that.
 */

/** Which of the "+" dialogs is up, if any. `tell` left on 2026-09-06:
    leaving a line is the wizard's first kind now (`NewSessionDialog`),
    one door for every kind of session. */
type PlusDialog = "new" | "import" | "invite" | "join" | "ws-add" | null;

const POLL_MS = 2000;
/**
 * The beat for a reading **this screen is not drawing the changes of**.
 *
 * ⚠️ **Three calls on one beat was the app's whole heartbeat until
 * 2026-09-11, and it is why the window felt heavy** (用户:「整体 app
 * 很卡顿,不如网页版的流畅」). Measured through the web face, which
 * runs the same `khor_gui_core` entry points the tauri commands do:
 *
 * ```
 * sessions      175–250 ms
 * devices       103–115 ms
 * hooks_state    19–24  ms
 * ```
 *
 * — about 330 ms of work every 2 s, and `top` agreed: **khor-gui 12–15 %
 * of a core while nobody touched it, WebContent 1–3 %**. So the cost was
 * never the rendering; it was three answers being fetched at the pace of
 * the fastest-changing one.
 *
 * **What splits them is not importance, it is what the screen draws.**
 * Away from the machines landing, `devices` feeds a face in the corner
 * and a name in the search words — neither of which moves — while the
 * vitals that do move are not on the glass at all.
 *
 * **It does not stop a landing from being wrong about itself**: a screen
 * that starts drawing a device's vitals without saying so gets them at
 * this pace and looks frozen. The seam is the landing, so a new reader
 * of a fast-moving field belongs on the fast beat or on its own.
 */
const UNWATCHED_MS = 10_000;
/**
 * The hooks answer's own beat, and the slowest here on purpose: it
 * changes when somebody runs `khor hooks install` in a terminal, which
 * is a thing that happens approximately never and never twice. It was on
 * the 2 s beat until 2026-09-11 for the reason above its state — a button
 * that only learns on mount would offer to do something already done —
 * and that reason survives a minute of lateness intact.
 */
const HOOKS_MS = 60_000;

/** Each landing's stored ticks, one entry per pane (`lib/session-filter`)
    — sessions, machines, workspaces and 额度 each tick their own axes.
    The workspace column keeps its own under a pane name of its own —
    see `WorkspacePane`. */
function storedWordsAll(): Record<Landing, string[]> {
  return {
    sessions: storedWords("sessions"),
    devices: storedWords("devices"),
    workspaces: storedWords("workspaces"),
    browser: storedWords("browser"),
    quota: storedWords("quota"),
  };
}

// In the order the user set on 2026-09-02 —「分别是会话、机器、文件、
// 用量、浏览器」— which is also the fold order: the narrow bar shows
// the first `NARROW_SHOWN` and folds the rest behind「…」.
//
// ⚠️ **A function, not a `const`** (批 D3). The names are catalog reads,
// and a module-level constant would capture whichever language the page
// happened to load in and keep it for the life of the tab — the one
// failure a language switch has, and an invisible one, because the rest
// of the screen would have changed around it. `check-type` refuses the
// shape.
const landings = (): { key: Landing; name: string; glyph: ReactNode }[] => [
  { key: "sessions", name: gui.sessions_tab, glyph: <IconSessions /> },
  { key: "devices", name: gui.devices_tab, glyph: <IconDevices /> },
  // 2026-09-11 user ruling:「工作区替代文件」. The glyph is still
  // `IconFiles` — the landing changed, the drawing has not been
  // redrawn yet, and a wrong-but-honest icon beats an invented one
  // that no gate has measured (`scripts/check-icons.mjs`).
  { key: "workspaces", name: gui.workspaces_tab, glyph: <IconFiles /> },
  // 额度: the registered API accounts as rings (`QuotaLanding`). Like
  // the browsing landings it is one region rather than a list beside a
  // detail — its list is its own, and the rail only opens it.
  { key: "quota", name: gui.accounts_tab, glyph: <IconQuota /> },
  { key: "browser", name: gui.browser_tab, glyph: <IconBrowser /> },
];

// ⚠️ 墓碑 (the redesign, 2026-10-02): the rail — a column of glyphs down
// the left on the wide face, a bar along the foot on the narrow one, with
// `RailItem`, `hasList` and `NARROW_SHOWN` — is gone. Its job is the
// floating navigation's (`components/FloatNav`): at most four seats, the
// first three landings in the user's order (`lib/nav-order`) and 更多.

const NO_QUERIES: Record<Landing, string> = {
  sessions: "",
  devices: "",
  quota: "",
  workspaces: "",
  browser: "",
};

/** The glyph beside each 「…」 page in its list (`identity` wears this
    machine's face instead, drawn where the list is). */
const MORE_GLYPH: Record<Exclude<MorePage, "identity">, (p: { className?: string }) => ReactNode> = {
  settings: IconGear,
  appearance: IconAppearance,
  network: IconNetwork,
  security: IconShield,
  doctor: IconPulse,
};

export default function App() {
  const narrow = useNarrow();
  /**
   * The app lock (docs/MORE.md 安全).
   *
   * **Held at the very top of the shell because of what a lock has to
   * be able to do**: return the lock screen *instead of* everything
   * below, so a locked face has no list, no conversation and no dialog
   * in its document at all (「卸掉不是盖住」). Every hook in this
   * component still runs — the early return is at the foot of the
   * function, where React can survive it — but nothing any of them
   * fetched is drawn.
   */
  const lock = useLock();
  // The two seats of the left block (docs/SHELL.md 第十三轮): where a
  // landing's head and its list column go. Made here, above every early
  // return, because they are a hook and the block is drawn below.
  const seats = useSideSeats();
  // 设置 (docs/MORE.md), read once here and handed down through a
  // context: the four keys this batch lands are read on four screens
  // that are not each other's children (`src/settings.ts`).
  const settings = useStoredSettings();
  // 外观 (批 D2). **The store's answer replaces the mirror's guess**,
  // and `wearTheme` is idempotent — on every launch but the first after
  // somebody changed the setting, this stamps exactly what `main.tsx`
  // already stamped. The window's own appearance rides on the same
  // call, which is why it happens here rather than inside 设置: a
  // machine that comes up dark must dress its title bar without
  // anybody opening a page.
  // 语言 (批 D3). **Resolved during the render, not in an effect**, and
  // that is the one thing about this that is not like 外观 above: a
  // theme is CSS and an attribute set afterwards repaints on its own,
  // but every word on screen is a property read off the catalog, taken
  // while this tree renders. An effect runs *after* the render that
  // would have used the new language, and nothing would render a second
  // time — the page would sit there in the old one until something else
  // happened to move.
  //
  // Idempotent, which is what makes it safe here: it sets a module
  // variable, `<html lang>` and a mirror, all to the same values on a
  // second call (StrictMode's double render does exactly that).
  speakLang((settings.rows?.find((r) => r.name === "lang")?.value ?? null) as LangPref | null);

  const themePref = settings.rows?.find((r) => r.name === "theme")?.value ?? null;
  useEffect(() => {
    if (themePref !== null) wearTheme(themePref as ThemePref);
  }, [themePref]);
  // **The breakpoint, published once.** `use-narrow` owns the number
  // and stylesheets need it too (the machine card's part rows drop
  // their shared grid narrow) — so the shell stamps the answer on the
  // root and CSS keys off the attribute, rather than a `@media`
  // repeating a number that can drift with nothing to catch it. The px
  // gate refuses the second copy out loud.
  useEffect(() => {
    const root = document.documentElement;
    if (narrow) root.setAttribute("data-narrow", "");
    else root.removeAttribute("data-narrow");
  }, [narrow]);
  // **The corner the window's own three buttons stand in** (第八轮 第 58
  // 条). Published on the root exactly the way the breakpoint above is,
  // and for the same reason: CSS cannot ask either question, so the one
  // side that can answer says it once and `tokens.css` reads it by name
  // (`--traffic`).
  //
  // **Full screen takes the gap back.** macOS hides the three buttons
  // with the title bar when the window goes full screen, and a gap left
  // behind for buttons that are not there is the 无效面积 this whole
  // condition exists to remove.
  const [fullscreen, setFullscreen] = useState(false);
  useEffect(() => onFullscreen(setFullscreen), []);
  // **The left block on the system's sidebar material** (docs/SHELL.md
  // 第十五轮 第 7 条, D4): only the macOS app has it — its window is
  // transparent over an `NSVisualEffectView` (`src-tauri` `build_the_window`)
  // — so only there is the root stamped, and `app.css` lets the block's
  // ground go clear under the stamp. Every other face keeps `--side`.
  // ⚠️ 墓碑 (2026-10-02, the redesign's one ground): the root was
  // stamped `data-material="sidebar"` here on the macOS app, and the left
  // block went clear over AppKit's sidebar material. There is one ground
  // now at every width, the Mac window included, so the page paints it.
  useEffect(() => {
    const root = document.documentElement;
    if (trafficLights && !fullscreen) root.setAttribute("data-traffic", "");
    else root.removeAttribute("data-traffic");
  }, [fullscreen]);
  // …and how wide that strip really is, **asked of the window** rather
  // than written into the stylesheet (docs/SHELL.md 第十一轮): the three
  // buttons are the SDK's size, so only the app can say. Once, at
  // startup; the token's own value is the first frame's.
  useEffect(() => {
    if (!trafficLights) return;
    let live = true;
    trafficRoom()
      .then((px) => {
        if (live && px > 0) document.documentElement.style.setProperty("--traffic", `${px}px`);
      })
      .catch(() => {});
    return () => {
      live = false;
    };
  }, []);
  // 退出时还有草稿要拦 (工作区第五轮 第 39 条). Two roads, one
  // condition, and the condition is `lib/drafts`'s register.
  //
  // **The app's road prevents the close and puts a dialog up.** Tauri
  // stops the window as soon as this page is listening at all
  // (`onQuitRequested`), so the listener is installed for the life of
  // the app and always answers — a listener that came and went is a
  // window that sometimes cannot be closed.
  //
  // **The browser's road is `beforeunload` and it is not the same
  // thing.** A page cannot put its own dialog in front of a tab
  // closing; all it can do is ask the browser to ask, in the browser's
  // own words, which is the whole of what the web gives. So there is no
  // dialog there and the wording is not khor's — what the two share is
  // the condition and nothing else.
  const [quitting, setQuitting] = useState<number | null>(null);
  useEffect(() => {
    const stop = onQuitRequested(() => {
      const n = draftCount();
      if (n === 0) return false;
      setQuitting(n);
      return true;
    });
    const leaving = (e: BeforeUnloadEvent) => {
      if (draftCount() === 0) return;
      e.preventDefault();
      // Deprecated and still required: some engines only ask when this
      // is set, and the string is never shown by any of them.
      e.returnValue = gui.watch_unsaved;
    };
    window.addEventListener("beforeunload", leaving);
    return () => {
      stop();
      window.removeEventListener("beforeunload", leaving);
    };
  }, []);
  // The dialog on screen counts again while it is up, so saving the last
  // draft from behind it takes it down rather than leaving a stale
  // number in front of somebody.
  useEffect(() => {
    if (quitting === null) return;
    return watchDrafts((n) => setQuitting(n === 0 ? null : n));
  }, [quitting]);

  // **The workspace page's three seams all ask the same question**
  // (2026-09-17 ruling 52): how much room is there beside the file,
  // rather than 「is this half the window yet」. Away from a workspace
  // `roomBeside` finds no middle column and answers the old ceiling, so
  // the list pane on every other landing is unchanged.
  // **`LIST_PANE`, not `[data-list]`**: this one seam sizes the shell's
  // list pane on every landing that has one, and each of them answers
  // to a name of its own (`components/ListPane`'s `mark`). Naming only
  // the sessions pane's meant the drag measured nothing at all on 额度
  // and 浏览 — see the constant.
  const listPane = useListWidth(narrow, "list", "left", LIST_PANE, roomBeside(LIST_PANE));
  /** The workspace's file tree, as a seam of its own — **held here
      rather than in the pane**, because the control that folds it now
      sits on the workspace's row in the list (2026-09-16:「再次点击工作
      区 list 的某个,也是收起和展开该工作区的文件 list」, landed as a
      chevron on the open row after the two-meanings trap was talked
      through), and the list and the pane are siblings: the one thing
      both can reach is this. */
  const treePane = useListWidth(
    narrow,
    "ws-tree",
    "left",
    "[data-ws-tree]",
    roomBeside("[data-ws-tree]"),
  );
  const [landing, setLanding] = useState<Landing>("sessions");
  const [rows, setRows] = useState<SessionRow[]>([]);
  const [devices, setDevices] = useState<DeviceRow[]>([]);
  /** Whether this machine can host — `false` on a phone, said by its own
      row (`lib/here`, docs/IOS.md 第二步). Handed down as a context; read
      here for the shell's own acts (the hooks, the editor, the lock). */
  const hosts = hereHosts(devices);
  // Whether either list has been answered once (components/RegionState):
  // until then an empty array is 「not read yet」, drawn as a skeleton,
  // and never as 「there are none」.
  const [rowsRead, setRowsRead] = useState(false);
  const [devicesRead, setDevicesRead] = useState(false);
  // The sessions a person has open, as tabs, and which is in front:
  // `selected` is the front tab (`useSessionTabs` — iTerm's arrangement,
  // asked for 2026-09-02). Opening a row opens or focuses its tab;
  // closing a tab is a view leaving, never a session ending.
  // 标签页上限 (docs/SHELL.md 第三轮 第 4 条): a pinned session's tab is
  // one a person said to keep. A ref kept in step with `rows`, read by
  // the hook at the moment a tab opens.
  // Read off the rows this component holds, not `useSetting`: the
  // provider is rendered below, so the context here is always empty
  // (`tabsLimitOf`, docs/SHELL.md 第五轮 第 11 条).
  const tabsLimit = tabsLimitOf(settings.rows?.find((r) => r.name === "tabs-limit")?.value ?? null);
  const pinnedRows = useRef(new Set<string>());
  pinnedRows.current = new Set(rows.filter((r) => r.pinned).map((r) => r.id));
  const sessionTabs = useSessionTabs(tabsLimit, (id) => pinnedRows.current.has(id));
  const { tabs: openTabs, drop: dropTabs } = sessionTabs;
  const selected = sessionTabs.active;
  // Whether the open tabs are up as a list to pick from (`TabSwitcher`)
  // instead of the front tab's content. The narrow face's only way to
  // another tab — a phone has no strip — and the wide face's answer
  // once the strip has stopped showing every tab (`TabStrip.onSwitch`,
  // which is the side that can measure that). Reset wherever the tabs
  // stop being the thing on screen: another landing, a 「…」 page,
  // or the last tab closing.
  const [switchingTabs, setSwitchingTabs] = useState(false);
  useEffect(() => {
    if (openTabs.length === 0) setSwitchingTabs(false);
  }, [openTabs.length]);
  /** The open ids that have actually been seen on the list — see the
      effect below for why "open" alone is not enough. */
  const wasListed = useRef<Set<string>>(new Set());
  // The open machine, kept apart from the open session for the same
  // reason each pane keeps its own query: they are two selections in two
  // places, and one variable would have opening a machine close the
  // session you were reading.
  const [selectedDevice, setSelectedDevice] = useState<string | null>(null);
  /** Whether the open machine's page shows its sessions rather than its
      dashboard — the head's 会话 switch (the redesign, 2026-10-02). Back
      to the dashboard whenever another machine opens. */
  // …and its files in place (2026-10-02, the owner: 「文件按钮点击后，不要
  // 跳转到工作区」). One value, so the two switches are exclusive: turning
  // one on turns the other off.
  const [machineBody, setMachineBody] = useState<"dashboard" | "sessions" | "files" | "web">("dashboard");
  /** The open file's way out on the narrow machine page, while there is
      one: the head's ‹ takes it first (`MachineFiles`). */
  const [fileBack, setFileBackState] = useState<{ go: () => void } | null>(null);
  const setFileBack = useCallback((back: (() => void) | null) => setFileBackState(back ? { go: back } : null), []);
  useEffect(() => setMachineBody("dashboard"), [selectedDevice]);
  // …and which workspace is open. **A path and not a row**, because
  // the two ways in do not agree on what they have: the list hands over
  // a checkout it already knows about, and the machine card's 看它的
  // 文件 hands over a machine and nothing else. `path` empty is that
  // machine's home, the `Ls` contract — which is how a directory that
  // is not a workspace at all opens in the same pane.
  //
  // It lives here rather than in the landing so that arriving from
  // somewhere else is a state already set, not a landing being told to
  // remember something after it mounts.
  const [openWs, setOpenWs] = useState<{
    device: string;
    machine: string;
    path: string;
  } | null>(null);
  // …and the browser landing's own tabs. **A second strip, not a
  // second mechanism** (浏览器接壳批): both browsing landings are the
  // same shell with a different kind, and the two stores are kept apart
  // by that kind alone (`useBrowseTabs` keys on it), so a page can
  // never turn up where a directory belongs.
  // **Watched while its landing is open** (`useBrowseTabs`'s `WATCH_MS`):
  // `.khor/tabs.json` has two writers now, and `khor tabs open` in a
  // terminal has to show up in the column without anybody reloading.
  const pageTabs = useBrowseTabs(PAGES.kind, landing === "browser");
  // Narrow face only: which single screen is up. Wide ignores it.
  const [screen, setScreen] = useState<"list" | "detail">("list");
  const [stale, setStale] = useState(false);
  // One query per pane: switching landings is not a way of clearing a
  // search, and a search that follows you across panes filters the wrong
  // list on arrival.
  const [queries, setQueries] = useState<Record<Landing, string>>(NO_QUERIES);
  // Per pane and remembered, unlike the query above: a search box is a
  // thing you are doing right now, a filter is a way you have decided to
  // look at this list.
  const [words, setWords] = useState<Record<Landing, string[]>>(storedWordsAll);
  const [arrangeBy, setArrangeBy] = useState<string>(storedArrange);
  const [dialog, setDialog] = useState<PlusDialog>(null);
  /** Which machine the wizard opens with picked — the machine card's
      开会话 names one (2026-09-06); the sessions pane's "+" names none,
      which the wizard reads as this machine. Set together with the
      dialog and never on its own, so a card's pick cannot outlive the
      opening it was made for. */
  const [newSessionOn, setNewSessionOn] = useState("");
  /** …and which directory. The workspace column's 「+」 names its
      project's (工作区批⑩); every other door names none, which the
      wizard reads as that machine's home. Set together with the dialog
      for `newSessionOn`'s reason. */
  const [newSessionAt, setNewSessionAt] = useState("");
  /** Bumped when this shell has changed the workspace table from
      outside the list — the pane bar's 「+」 lives up here, and the list
      owns every read it makes. One number rather than a callback,
      because what the list needs to know is 「something changed」 and
      not what: it re-reads a local table, which costs nothing. */
  const [wsAdded, setWsAdded] = useState(0);
  /** Open the wizard. `inPlace` is the workspace column's own 「+」
      (2026-09-16 ruling 34): the session it makes belongs to the
      project somebody is standing in, so when it opens, this shell
      stays where it is and hands the id down instead of walking to the
      sessions landing. Every other door keeps the old behaviour —
      「the freshly-opened session is what the person came to see」, and
      from those doors there is nowhere else it could be seen. */
  const openNewSession = useCallback((machine = "", dir = "", inPlace = false) => {
    setNewSessionOn(machine);
    setNewSessionAt(dir);
    setNewSessionInPlace(inPlace);
    setDialog("new");
  }, []);
  /** Whether the open wizard was started from a workspace's own column,
      and the id it left there. **Two pieces of state and not one**: the
      first is true while the dialog is up and the second outlives it,
      which is what the column reads. */
  const [newSessionInPlace, setNewSessionInPlace] = useState(false);
  const [openedInWs, setOpenedInWs] = useState<string | null>(null);
  /**
   * Which page behind 「…」 or the avatar is open, if any (docs/MORE.md).
   *
   * **One value for four pages, and it is not a `PlusDialog`.** Those three
   * are the "+" menu's dialogs and belong to a pane; these are places
   * that take the whole region beside the rail. It is also not a
   * `Landing`, for the reason `Mark` gives about itself: a landing is a
   * list beside a detail and is where you are, and one of these is
   * opened over wherever you were and closed back to it — which is why
   * the landing underneath is never overwritten.
   *
   * 设置 used to be a boolean here (`settings`) opening a dialog. Four
   * pages behind one menu is what replaced it (「点击...,显示设置、安全、
   * 自检...点击后是就地的菜单栏,进一步点击都是独立的界面」).
   */
  const [morePage, setMorePage] = useState<MorePage | null>(null);
  // **What the agents have cost is no longer asked for here at all.**
  // This used to hold a `usage` answer beside the quota — every day
  // there has ever been, eighteen seconds for the first pass on the
  // machine answering it (`khor_node::usage::Meters::tally`). Its last
  // reader was the network's-now 今天 line, removed 2026-08-29, and a
  // poll nobody reads is not free: it is that tally, on a timer, for a
  // number nothing draws. `khor usage` is where that question lives.
  // ⚠️ 墓碑: `quota`, this machine's probed subscription, polled here
  // for the 额度 list's row with no menu. Since 2026-09-22 a login found
  // here is a registration (`Node::adopt_subscriptions`, docs/SHELL.md
  // 第七轮 第 3 条) and comes in with the board like every other row.

  // This machine's hooks, polled with everything else rather than read
  // once when the card opens: `khor hooks install` in a terminal is the
  // other way this changes, and a button that only learns on mount would
  // sit there offering to do something already done.
  const [hooks, setHooks] = useState<HooksState | null>(null);
  const [hooksFailed, setHooksFailed] = useState(false);

  const hidden = useHidden();
  // Back from the background with the node caught up (docs/IOS.md 第三步):
  // the rows and the machines ask again at once, the way `hidden` makes
  // them (`use-woken`).
  const woken = useWoken();

  /* ⚠️ **Tombstone: these were one effect until 2026-09-11** — one
     `Promise.all` of three calls on the 2 s beat, under a comment that
     named the bargain it was not keeping:「Nobody looking, nobody
     paying: this is three calls every two seconds, and it is the app's
     whole heartbeat」. Nobody looking still paid for all three. The
     readings and the split are on `UNWATCHED_MS`.

     They stay three effects rather than one effect with three timers
     because each one's dependencies are what decide its beat, and React
     already does exactly this bookkeeping for an effect — a hand-rolled
     scheduler here would be the same thing, written again, with the
     re-ask-on-arrival behaviour to remember. */

  // **The session rows, and nothing else, are the app's heartbeat.**
  useEffect(() => {
    // …and it stops while this face is locked. Nothing draws these, so
    // the poll would be a locked window asking a node for conversations
    // every two seconds — the cost this app already paid once for
    // answers nobody was looking at (`UNWATCHED_MS`), plus the one thing
    // a lock is for.
    if (lock.locked) return;
    let live = true;
    const tick = () => {
      fetchSessions(arrangeBy)
        .then((rows) => {
          if (!live) return;
          setRows(rows);
          setRowsRead(true);
          setStale(false);
        })
        .catch(() => live && setStale(true));
    };
    tick();
    const t = setInterval(tick, hidden ? HIDDEN_MS : POLL_MS);
    return () => {
      live = false;
      clearInterval(t);
    };
    // Re-subscribed when the arrangement changes: the new order comes
    // from the node, so switching modes is a fetch, never a re-sort.
    //
    // **And when the window is looked at again**, which is the same
    // shape: this effect already asks once before it sets its interval,
    // so coming back is a fresh answer immediately rather than after a
    // wait — no extra wiring, and nothing to remember to clear.
    //
    // **It carries 「后端连不上」 for all three**, and that is a claim
    // about today's shape rather than a rule: the three answers travel
    // the same way to the same node, and this one asks most often, so it
    // is the one that notices first. **A road that could break for
    // `devices` alone would go unsaid here** — if one ever exists, the
    // sentence belongs where the failure is, not on the busiest caller.
  }, [arrangeBy, hidden, woken, lock.locked]);

  // **The machines, at the pace of whichever screen is up.** The vitals
  // in a row move every few seconds and the machines landing draws them;
  // everywhere else this answer supplies a face and a name, and those do
  // not move (`UNWATCHED_MS`).
  useEffect(() => {
    let live = true;
    const tick = () => {
      fetchDevices()
        .then((rows) => {
          if (!live) return;
          setDevices(rows);
          setDevicesRead(true);
        })
        // Silent on purpose: 「后端连不上」 belongs to the rows above,
        // which asks more often and would already be saying it.
        .catch(() => {});
    };
    tick();
    const t = setInterval(
      tick,
      hidden ? HIDDEN_MS : landing === "devices" ? POLL_MS : UNWATCHED_MS,
    );
    return () => {
      live = false;
      clearInterval(t);
    };
    // **Arriving at the machines landing is a fetch, not a wait**: the
    // landing is a dependency, so stepping onto it rebuilds this effect
    // and the ask above the interval runs immediately. That is the same
    // property the rows rely on for `hidden`, and it is why the slow
    // beat is not felt on arrival — only by a card left open, which is
    // the machines landing and therefore fast again.
  }, [hidden, woken, landing]);

  // **Whether this machine's hooks are installed** — polled rather than
  // read once on mount, because `khor hooks install` in a terminal is the
  // other way this changes and a button that only learns on mount would
  // sit there offering to do something already done. A minute late is
  // still early enough for that (`HOOKS_MS`).
  useEffect(() => {
    // **Not on a phone** (docs/IOS.md 第二步): there is no claude there to
    // hook, so there is no answer worth a minute's beat — and the card
    // that would show it is not drawn (`MachineCard`'s `Hooks`). **Asked
    // once the table has said which this is**, or a phone's first beat
    // would go out on the default before its own row arrived; the card
    // that shows the answer needs that table anyway.
    if (!devicesRead || !hosts) return;
    let live = true;
    const tick = () => {
      fetchHooks()
        .then((state) => live && setHooks(state))
        .catch(() => {});
    };
    tick();
    // ⚠️ **Not `hidden ? HIDDEN_MS : HOOKS_MS`**, which is what the two
    // effects above say and what this one said first: `HIDDEN_MS` is
    // 10 s and this beat is 60 s, so that spelling asks *more* often
    // while nobody is looking. `hidden` stays a dependency, which is the
    // half that was wanted — coming back to the window rebuilds the
    // effect and re-asks at once, the one moment a stale hooks answer
    // could be on the glass — while the beat itself stays one number.
    const t = setInterval(tick, HOOKS_MS);
    return () => {
      live = false;
      clearInterval(t);
    };
  }, [hidden, hosts, devicesRead]);

  const onSelect = useCallback((row: SessionRow) => {
    sessionTabs.open(row.id);
    if (row.unread > 0) {
      // Looking at it is the seen semantics; the watermark replicates,
      // so this clears the badge on every device.
      markSeen(row.id).catch(() => {});
    }
    setScreen("detail");
    // `open` is a stable callback; the object around it is not.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionTabs.open]);

  const onSelectDevice = useCallback((row: DeviceRow) => {
    setSelectedDevice(row.id);
    setScreen("detail");
  }, []);

  const setQuery = useCallback(
    (q: string) => setQueries((was) => ({ ...was, [landing]: q })),
    [landing],
  );

  // Pinning: ask the node, then refresh from it. **The rows are not
  // reordered here** — the order is the node's answer, and guessing it
  // locally would mean the screen briefly shows an order the library
  // never produced (and permanently shows it if the call fails).
  const refresh = useCallback(async () => {
    const [s, d] = await Promise.all([fetchSessions(arrangeBy), fetchDevices()]);
    setRows(s);
    setDevices(d);
    setRowsRead(true);
    setDevicesRead(true);
  }, [arrangeBy]);

  // Two-argument `then` for the same reason the pins use it: the
  // rejection handler must see *this* call failing and nothing else. And
  // the answer carries the state afterwards, so the button never spends
  // a poll interval showing the job it just finished.
  const onHooks = useCallback((run: () => Promise<HooksState>) => {
    run().then(
      (state) => {
        setHooks(state);
        setHooksFailed(false);
      },
      () => setHooksFailed(true),
    );
  }, []);

  const onInstallHooks = useCallback(() => onHooks(installHooks), [onHooks]);
  const onUninstallHooks = useCallback(() => onHooks(uninstallHooks), [onHooks]);

  /**
   * Rows whose last pin attempt did not take, by row id.
   *
   * **A failed pin used to look exactly like a missed click** — the row
   * simply did not move — and docs/UX.md forbids 做了但没变化 and 失败
   * wearing one face. The race that made this common is fixed at the
   * source (a shared temp filename in the block store, measured at 17 of
   * 40 pins failing before and 0 of 40 after), so what is left is the
   * rare disk error; rare is a reason for the face to be small, not a
   * reason for it to be absent.
   *
   * **It is said on the button that was pressed**, and no notification
   * surface is introduced to carry it. There is nowhere in this app that
   * collects messages, and building one for the least frequent thing
   * that happens here would be a whole mechanism whose first and only
   * customer is a disk error.
   *
   * Session ids and device ids cannot collide (`kind/leaf` against hex),
   * so one set serves both lists.
   */
  const [pinFailed, setPinFailed] = useState<Set<string>>(() => new Set());

  const markPin = useCallback((id: string, failed: boolean) => {
    setPinFailed((was) => {
      if (was.has(id) === failed) return was;
      const next = new Set(was);
      if (failed) next.add(id);
      else next.delete(id);
      return next;
    });
  }, []);

  // Two-argument `then`, not `.then().catch()`: the rejection handler
  // must see the pin call failing and nothing else. Chained, a refresh
  // that threw after a pin that worked would paint the failure face on a
  // pin that actually took.
  //
  // **The returned promise says whether the pin took**, which is what
  // the row's menu reads to decide between closing and staying open
  // (`RowMenu`). Two things about the shape are load-bearing and neither
  // is style: the rejection handler still **rethrows** rather than
  // swallowing, so the failure that painted the face is the same event
  // that keeps the menu open; and the refresh is *not* returned into the
  // chain — awaiting it would put a whole poll between a pin that
  // already worked and the menu closing, and a refresh that failed
  // afterwards would then read as a pin that failed, which is the very
  // confusion the two-argument form exists to prevent.
  const onPinSession = useCallback(
    (row: SessionRow) =>
      pinSession(row.id, !row.pinned).then(
        () => {
          markPin(row.id, false);
          refresh().catch(() => {});
        },
        (e) => {
          markPin(row.id, true);
          throw e;
        },
      ),
    [refresh, markPin],
  );

  /**
   * Ending a session from its row's menu.
   *
   * **The refusal is the point.** Whether a close is possible is
   * `Node::close`'s judgment — a discovered session is not khor's to
   * end — and it comes back as a whole sentence written for a person
   * ("不是 khor 起的,关不了它;去它自己的窗口里退"). Re-deriving that
   * rule here would be this face guessing at a backend judgment and
   * going stale the day one of those paths changes, which is the same
   * reasoning `DetailPane`'s own close carries.
   *
   * Kept by row id, the way `pinFailed` is, and cleared the moment a
   * later attempt on that row gets anywhere: a sentence about a refusal
   * that has since been answered is worse than no sentence.
   */
  const [closeRefused, setCloseRefused] = useState<Map<string, string>>(() => new Map());
  const onCloseSession = useCallback(
    (row: SessionRow) => {
      setCloseRefused((was) => {
        if (!was.has(row.id)) return was;
        const next = new Map(was);
        next.delete(row.id);
        return next;
      });
      return closeSession(row.id).then(
        () => {
          // Nothing to paint: the row leaves the list on the next poll,
          // and the selection guard below takes the pane with it.
          refresh().catch(() => {});
        },
        (e) => {
          setCloseRefused((was) =>
            new Map(was).set(row.id, String(e instanceof Error ? e.message : e)),
          );
          throw e;
        },
      );
    },
    [refresh],
  );

  // The session pin's shape, now that a machine row's pin is a menu
  // item too (2026-09-06): the promise says whether it took, the
  // rejection rethrows so the menu stays open on the row where the
  // press was made, and the refresh is not in the chain.
  const onPinDevice = useCallback(
    (row: DeviceRow) =>
      pinDevice(row.name, !row.pinned).then(
        () => {
          markPin(row.id, false);
          refresh().catch(() => {});
        },
        (e) => {
          markPin(row.id, true);
          throw e;
        },
      ),
    [refresh, markPin],
  );

  /**
   * 暂停 / 恢复 from a machine row's menu (docs/SHELL.md 第五轮 第 10 条):
   * `khor pause <机器>` for another machine, `khor pause` for this one —
   * the node takes this machine's own name as the same act. The refusal
   * (already in that state) is the node's sentence, kept by row the way
   * `forgetRefused` is; success refreshes the list, which is where the
   * row's grey comes from.
   */
  const [pauseRefused, setPauseRefused] = useState<Map<string, string>>(() => new Map());
  const onPauseDevice = useCallback(
    (row: DeviceRow) =>
      setMachinePaused(row.name, !row.paused).then(
        () => {
          setPauseRefused((was) => {
            if (!was.has(row.id)) return was;
            const next = new Map(was);
            next.delete(row.id);
            return next;
          });
          refresh().catch(() => {});
        },
        (e) => {
          setPauseRefused((was) => new Map(was).set(row.id, String(e instanceof Error ? e.message : e)));
          throw e;
        },
      ),
    [refresh],
  );

  /**
   * Showing a machine the door from its row's menu — `khor forget`.
   *
   * The refusal is `Node::forget_device`'s, in its own sentence (this
   * machine cannot forget itself; a name the table does not hold), and
   * it is kept by row the way `closeRefused` is: the menu prints it
   * under the item, and it is cleared the moment a later attempt on
   * that row gets anywhere. Nothing to paint on success — the row
   * leaves the list on the next poll, and if its card was open,
   * `openDevice` falls to null with it and the landing shows the map.
   */
  const [forgetRefused, setForgetRefused] = useState<Map<string, string>>(() => new Map());
  const onForgetDevice = useCallback(
    (row: DeviceRow) => {
      setForgetRefused((was) => {
        if (!was.has(row.id)) return was;
        const next = new Map(was);
        next.delete(row.id);
        return next;
      });
      return forgetDevice(row.name).then(
        () => {
          refresh().catch(() => {});
        },
        (e) => {
          setForgetRefused((was) =>
            new Map(was).set(row.id, String(e instanceof Error ? e.message : e)),
          );
          throw e;
        },
      );
    },
    [refresh],
  );

  // What is happening that nobody is watching — see the hook for the
  // two rules about what may appear here.
  const status = useStatusBar(rows);
  const selectedRow = rows.find((r) => r.id === selected) ?? null;

  // 会话落地页上的路径也可点 (2026-09-18 ruling 54). Ruling 35 did this
  // inside a workspace only, because that is where the two things it
  // needs were: a root for a relative path to be relative to, and a
  // middle column to open a file in. **A session row carries the
  // first** — `place` is its own working directory — and the second is
  // one landing away, which is the whole of this ruling.
  //
  // **The machine is the session's own**, so an answer about a file is
  // an answer from the disk the conversation is happening on.
  const sessionMachine = selectedRow?.home ?? "";
  const sessionRoot = selectedRow?.place ?? "";
  /** That machine's id. **The id and not the name**, because what it
      is compared against is a workspace row's `device` — and a name is
      a thing the user retypes (2026-09-17 改名也进命令行). */
  const sessionDevice = devices.find((d) => d.name === sessionMachine)?.id ?? "";
  const { known: sessionKnown, see: seeInSession } = useKnownFiles(sessionMachine, sessionRoot);
  /** A file a press in a conversation asked this shell to show, handed
      to whichever workspace pane opens next (`WorkspacePane`'s
      `reveal`). */
  const [reveal, setReveal] = useState<{
    path: string;
    line: number | null;
    at: number;
  } | null>(null);
  const sessionHome = useMemo<LinkHome | null>(() => {
    // **No place, no links** — a conversation nobody recorded a
    // directory for has no root, and a relative path in it is relative
    // to nothing. The context is null and the prose stays prose, which
    // is `linkHome`'s own default outside a workspace.
    if (sessionMachine === "" || sessionRoot === "") return null;
    /** That path on the machine the session runs on — the address's
        road, and what `⌘`+press and a workspace-less path both take. */
    const toHost = (abs: string) => {
      void launchOn(sessionMachine, abs).catch(() => {
        // The machine's own refusal (no opener, no screen) — it is
        // said where it is asked for, and a sentence about khor in the
        // middle of a sentence about code is not an improvement.
      });
    };
    return {
      machine: sessionMachine,
      root: sessionRoot,
      known: sessionKnown,
      see: seeInSession,
      launch: toHost,
      open: (abs, line) => {
        // **Asked at the press, from the local table** (ruling 54:
        // 「问本地那张工作区表(`ws ls`), 不走网」). One read of a
        // replicated document costs nothing and is never stale, which
        // is worth more here than a poll would be: the answer decides
        // where somebody lands.
        void fetchWorkspaces().then(
          (list) => {
            const ws = holding(list, sessionDevice, abs);
            // Nothing on this machine declares a project around it, so
            // there is no tree to open it in and no root to show it
            // under — the host's own opener is the honest answer, and
            // it is the same one an address in the same sentence gets.
            if (ws === null) {
              toHost(abs);
              return;
            }
            setMorePage(null);
            setLanding("workspaces");
            // The id stands in where the device table has no name for
            // it — `WorkspacesList`'s own line, for the same reason.
            setOpenWs({ device: ws.device, machine: ws.machine || ws.device, path: ws.path });
            setReveal({ path: abs, line, at: Date.now() });
            setScreen("detail");
          },
          // A table this face could not read is a face that knows of no
          // workspace, which is the branch above.
          () => toHost(abs),
        );
      },
    };
  }, [sessionMachine, sessionRoot, sessionDevice, sessionKnown, seeInSession]);
  // ⚠️ **The open row's workspace word left with the detail header**
  // (第十四轮 第 1 条): 「工作区 · 类型」 was the tab's own content said
  // a second time one row lower, and the user struck the whole row out.
  // The list's own chips and headings still name a workspace
  // (`SessionsList`, `placeLabels`); that is where it is said once.
  // A session that is gone stops being the selected one, and on the
  // narrow face the screen goes back to the list.
  //
  // **Without this the close leaves a lie standing**: `selectedRow`
  // falls to null on its own, so the wide face already empties — but the
  // *selection* would still name a dead id, and the narrow face would sit
  // on a detail screen with nothing in it and no list to see, which is a
  // dead end reachable by pressing 关闭.
  //
  // **Only a selection that was actually on the list gets cleared**, and
  // that is not belt-and-braces: the wizard selects the session it just
  // opened, and the row for it does not exist until the next poll — a
  // rule that read "selected but not in the list" as "gone" threw that
  // selection away a second after making it, and the fresh conversation
  // came up empty. Measured, on this very run.
  //
  // An empty list is likewise not evidence: a poll that answered with
  // nothing is a backend blink, not everything closing at once.
  //
  // The same rule for every open tab now, not only the front one: a tab
  // whose row was listed and is listed no more closes; one never yet
  // listed (a session the wizard just opened) stays until it appears.
  useEffect(() => {
    if (rows.length === 0 || openTabs.length === 0) return;
    const listed = new Set(rows.map((r) => r.id));
    for (const id of openTabs) if (listed.has(id)) wasListed.current.add(id);
    const gone = openTabs.filter((id) => !listed.has(id) && wasListed.current.has(id));
    if (gone.length === 0) return;
    for (const id of gone) wasListed.current.delete(id);
    dropTabs(gone);
    if (selected !== null && gone.includes(selected)) setScreen("list");
  }, [rows, openTabs, dropTabs, selected]);
  // **Rows, not events.** This added the two conditions, so a session
  // that was both waiting on an answer and unread counted twice and the
  // badge could read 4 over two rows — a number that matches nothing the
  // person can go and look at. What the glyph is for is "how many things
  // want you", and a thing wants you once however many reasons it has.
  const blockedOrUnread = rows.reduce(
    (n, r) => n + (r.word === "blocked" || r.unread > 0 ? 1 : 0),
    0,
  );

  // The filter offers the state keys the node actually sent, in the order
  // it sent them — which is its ranking, so the choices come out in the
  // order that matters. Enumerating the six words here instead would be
  // this layer deciding what states exist, and it does not get to
  // (docs/UX.md 状态呈现). Keys already ticked stay on the list even after
  // their last row leaves, or a tick becomes impossible to undo — which
  // now also covers a tick restored from storage before any row has
  // arrived, the one case where the ticked key is guaranteed to be
  // missing from `rows`.
  //
  // **Three axes now, and they are the node's own** (`khor_node::list`
  // groups by exactly these). A ticked key is a group key, so a filter
  // and a heading are the same string and cannot drift apart; the axis
  // heading is what `PaneBar` starts a section on.
  //
  // The machine axis reads `home` rather than `source`: `source` is the
  // offline axis and is absent on rows that live here, so an axis built
  // on it would be missing this machine — the one people reach for
  // first (`SessionRow::home` says the rest).
  // The four axes and their words, built by the helper both session
  // panes call (`lib/session-filter`). This landing offers all of them;
  // a workspace's column offers the state words alone, because the
  // other three are what that column already is.
  const wordOptions = useMemo(
    () => sessionOptions(rows, words.sessions),
    [rows, words],
  );
  const sessionAxes = useMemo<OmniAxis[]>(
    () => axesOf(wordOptions, devices),
    [wordOptions, devices],
  );
  // **The machines and the workspaces narrow the same way** (2026-09-29,
  // the owner: 「所有的session list应该都可以filter吧？」): the same menu,
  // chips and rule (`lib/list-filter`), on each list's own axes — which
  // each list names beside its rows (`DevicesList`, `WorkspacesList`).
  const deviceFilter = useDeviceFilter(devices, words.devices);
  const workspaceFilter = useWorkspaceFilter(devices, words.workspaces);
  /** What the open landing's pane bar offers to narrow by. */
  const paneFilter =
    landing === "sessions"
      ? { options: wordOptions, axes: sessionAxes }
      : landing === "devices"
        ? deviceFilter
        : landing === "workspaces"
          ? workspaceFilter
          : null;

  // (The files pane's box used to be assembled here — a machine chip
  // and a path axis, for a pane whose subject was one machine. It moved
  // into `AddressBar` with the landing's shape: the box now belongs to
  // a tab rather than to a pane, and a landing with no list pane has no
  // `PaneBar` to hang one on.)

  // (The browser pane's box was assembled here too, for the same kind of
  // pane, and left for the same reason. Both browsing landings put their
  // box in `AddressBar` now, where it belongs to a tab.)

  const chooseArrange = useCallback((key: string) => {
    setArrangeBy(key);
    rememberArrange(key);
  }, []);

  const toggleWord = useCallback(
    (key: string) =>
      setWords((was) => {
        const had = was[landing];
        const next = had.includes(key) ? had.filter((w) => w !== key) : [...had, key];
        rememberWords(landing, next);
        return { ...was, [landing]: next };
      }),
    [landing],
  );

  const sessionActions: PaneAction[] = [
    // First, because opening a session is the pane's first act
    // (会话身份批B: 建 session 是第一动作).
    // Every one of these opens a dialog, so each wears the ellipsis
    // (components/Button:「还会再问一次的动作，文案带省略号」) — the
    // `-ask` keys; the dialog's own title keeps the bare word.
    { key: "new", label: gui.new_session_ask, onSelect: () => openNewSession() },
    // What khor found and has not put on the list (docs/SHELL.md 第八轮).
    { key: "import", label: gui.import_sessions_ask, onSelect: () => setDialog("import") },
  ];
  const deviceActions: PaneAction[] = [
    { key: "invite", label: gui.make_a_ticket_ask, onSelect: () => setDialog("invite") },
    { key: "join", label: gui.join_with_a_ticket_ask, onSelect: () => setDialog("join") },
  ];

  // **The floating navigation** (the redesign, 2026-10-02). Its seats
  // are the user's first three landings; 更多 holds the rest, 设置 and
  // this machine, and is lit while any of them is on screen.
  const navOrder = useNavOrder();
  const seatOf = (key: Landing): NavSeat => {
    const l = landings().find((x) => x.key === key)!;
    return {
      key,
      name: l.name,
      glyph: l.glyph,
      // Only the sessions seat counts anything: the badge rule is that it
      // must be able to reach zero (docs/UX.md 状态呈现), and machines do
      // not go away when you have looked at them.
      badge: key === "sessions" ? blockedOrUnread : undefined,
    };
  };
  const goLanding = (key: Landing) => {
    // A 「…」 page is open over this landing, so choosing a landing closes
    // it — the one press that always answers 「带我回去」.
    setMorePage(null);
    setLanding(key);
    setScreen("list");
    setSwitchingTabs(false);
    // Choosing a landing brings a list put away back, at the width it had.
    listPane.expand();
  };
  const me = devices.find((d) => d.me) ?? null;
  /** The wide face's list put away by pressing the open landing again. */
  const folded = !narrow && listPane.collapsed;
  /** The wide face's top row (方案 B): the landings' glyphs beside the
      window's buttons, then 更多. The open landing pressed again folds its
      list, and once more brings it back; another landing switches and
      brings it back (the owner: no toggle button). */
  const strip = (
    <LandingStrip
      seats={navOrder.map(seatOf)}
      active={morePage === null ? landing : null}
      // Tinted on every page reached through it, 这台机器 too (F3.6; W23).
      moreOn={morePage !== null}
      onGo={(key) => {
        if (morePage === null && landing === key) listPane.toggle();
        else goLanding(key as Landing);
      }}
      // 更多's pages, the same place narrow's 设置 opens: the sections
      // in the list column, the first one beside them.
      onMore={() => {
        // Open already: folds the list, like any open landing's glyph.
        if (morePage !== null) {
          listPane.toggle();
          return;
        }
        setMorePage("settings");
        setSwitchingTabs(false);
        setScreen("detail");
        listPane.expand();
      }}
    />
  );
  /** **Folded, the strip does not move** (the audit fix plan F3.1; W01):
      the content's first row starts with a lead as wide as the list's
      default column, the strip right-aligned in it exactly as it stands
      over that column — so the glyph pressed to fold is still under the
      pointer to unfold, and the tabs begin after it. Only a list dragged
      wider than the default moves it, by at most the cap's 80. */
  const foldedLead = folded ? (
    <div
      data-folded-lead
      {...WINDOW_DRAG}
      className="flex w-list flex-none items-center justify-end self-stretch ps-wide-lead-start pe-row"
    >
      {strip}
    </div>
  ) : null;
  const nav = (
    <FloatNav
      seats={navOrder.slice(0, NAV_SHOWN).map(seatOf)}
      more={navOrder.slice(NAV_SHOWN).map(seatOf)}
      active={morePage === null ? landing : null}
      moreOn={morePage !== null || navOrder.slice(NAV_SHOWN).includes(landing)}
      onGo={(key) => goLanding(key as Landing)}
      onSettings={() => {
        setMorePage("settings");
        setSwitchingTabs(false);
        // Narrow: the sections' list, one screen; wide: the first page
        // beside the sections in the list column.
        setScreen(narrow ? "list" : "detail");
      }}
      me={me ? { face: me.face, name: me.shown } : null}
      onMe={() => {
        setMorePage("identity");
        setScreen("detail");
        setSwitchingTabs(false);
      }}
      // Floating over the foot of whatever holds it — the screen, or the
      // list column — centred, clear of a phone's home indicator.
      className="absolute bottom-[calc(var(--nav-gap)+var(--safe-bottom))] left-1/2 z-30 -translate-x-1/2"
    />
  );

  // **The keyboard and the menu bar** (`lib/shortcuts`, 2026-09-28): the
  // map and the rules are there; what is here is what only the shell can
  // do — walk the rail, open a 「…」 page, and lend the session tabs to
  // ⌘W and ⌃⇥ while they are the strip on screen.
  useTabHost(
    landing === "sessions" && morePage === null && openTabs.length > 0
      ? {
          count: openTabs.length,
          front: selected !== null,
          close: () => {
            if (selected !== null) sessionTabs.close(selected);
          },
          step: (by) => {
            const at = openTabs.indexOf(selected ?? "");
            const next = openTabs[(at + by + openTabs.length) % openTabs.length];
            if (next === undefined) return;
            sessionTabs.focus(next);
            setSwitchingTabs(false);
            setScreen("detail");
          },
        }
      : null,
  );
  useShortcuts({
    locked: !lock.settled || lock.locked,
    // What the rail's glyph does on a first press (the second press folds
    // the list, which a key repeated by accident should not).
    // **⌘-digits follow the strip** (the audit fix plan F3.5; W03): the
    // Nth glyph the strip shows, in the order 外观 → 导航 set. The Mac
    // menu bar's View items still name the old fixed order (`menu.rs`,
    // native — not changed here).
    go: (i) => {
      const l = landings().find((x) => x.key === navOrder[i]);
      if (!l) return;
      setMorePage(null);
      setLanding(l.key);
      setScreen("list");
      setSwitchingTabs(false);
      listPane.expand();
    },
    settings: () => {
      setMorePage("settings");
      setScreen("detail");
      setSwitchingTabs(false);
    },
    closePage: morePage !== null ? () => setMorePage(null) : null,
    lock: lock.facts?.on ? lock.now : null,
    find: () => window.dispatchEvent(new Event(FIND_ALL)),
    import: () => setDialog("import"),
  });

  const sessions = landing === "sessions";
  /** Whether this landing browses (docs/UX.md 浏览型) — the two that do
      share one shell and have no list pane at all. */
  const browses = landing === "browser";
  // The 额度 page is one region like the browsing landings, for the
  // reason its rail entry gives.
  const quotas = landing === "quota";
  // Every row's last day, on one beat, for the band under the open
  // session's header (`DetailPane`). Sized to the pane the band is drawn
  // in — measured, never assumed (`useStripWidth`) — and asked for every
  // row rather than the front one, so switching tabs costs no fetch.
  // A callback ref's state, not a ref object: the narrow face keys this
  // box by its screen, and a replaced node must re-aim the measurement
  // (`useStripWidth` says what the band looked like when it did not).
  const [detailBox, setDetailBox] = useState<HTMLDivElement | null>(null);
  const days = useDayStrips(useStripWidth(detailBox, sessions));
  /** One session's day as the pane holds it (`DayStrip` `Day`): `null`
      until the first answer, then its runs. */
  const dayOf = (id: string): Day => (days === null ? null : (days[id] ?? []));
  // Whether the day is asked for under the tabs (`use-day-shown`): the
  // front tab is the switch, and says so on hover.
  const dayShown = useDayShown();
  // The open sessions as the strip paints them: each tab wears its
  // row's title and the row's own identity assembly (`SessionFace` —
  // face, state halo, kind mark), the same facts in the same clothes. A
  // tab whose row has not been listed yet (just opened) wears its id.
  const tabFaces: TabFace[] = useMemo(
    () =>
      openTabs.map((id) => {
        const r = rows.find((row) => row.id === id);
        // **Pressed again, the tab in front shows or hides its day**
        // (2026-09-29:「点击标签后显示吧。悬停的时候加一个提示」). Read
        // by the strip only on the tab in front; the words and whether
        // there is anything to open are the day's (`daySwitch`).
        const sw = daySwitch(days === null ? null : (days[id] ?? []), dayShown, () => setDayShown(!dayShown));
        const again = {
          hint: sw.hint,
          expanded: sw.expanded,
          controls: sw.expanded ? DAY_ROW_ID : undefined,
          toggle: sw.toggle,
        };
        return r
          ? {
              id,
              label: r.title,
              face: r.face,
              path: r.place ?? "",
              session: { word: r.word, kind: r.kind, category: r.category },
              // A session tab's name is the row's title, which is a
              // name somebody gave it — so double-click renames it
              // here too. A browsing tab below wears a page's own
              // name and hands none of this in.
              rename: { kind: "session" as const, id },
              again,
            }
          : { id, label: id, face: null, path: "", again };
      }),
    [openTabs, rows, days, dayShown],
  );
  // The open tabs as rows to pick from — one switcher, written once,
  // whichever face asked for it (`BrowseLanding`'s rule). No `onNew`:
  // a session tab *is* its session, so there is no blank tab to mint —
  // sessions are made by the wizard, and the switcher's foot stays off.
  const tabSwitcher = (
    <TabSwitcher
      faces={tabFaces}
      active={selected ?? ""}
      onFocus={(id) => {
        sessionTabs.focus(id);
        setSwitchingTabs(false);
        setScreen("detail");
      }}
      onClose={sessionTabs.close}
      // Only the narrow face draws this screen; its box is L0's there,
      // beside the way back to the tab that was in front.
      seat="side"
      onBack={() => setSwitchingTabs(false)}
    />
  );
  const paneName = landings().find((l) => l.key === landing)!.name;

  // No pane wears its name where you can see it (docs/UX.md). The name
  // rides the region's aria-label — read aloud, taking no room — and the
  // strip where a title would have sat holds the work instead.
  //
  // **The ground, the head's height, the scroller and the seam are
  // `ListPane`'s** (2026-09-20 第 6 条). Three panes drew those four
  // things for themselves and had already drifted on the first; what is
  // left here is what only this landing knows — what the bar holds and
  // which list is under it.
  /** Open one workspace, from its row or from the overview. **The id
      stands in where there is no name.** A row whose machine has left
      the device table carries an empty `machine`, and an empty machine
      means「this one」to every verb the node has — so the pane would
      browse *this* disk under another machine's path. The hex resolves
      to no machine at all, so every question the pane asks refuses by
      name and it paints refusals, which is the true answer
      (`views/WorkspacesList` holds the same line for the row's two
      corners). */
  const openWorkspace = (r: WorkspaceRow) => {
    setOpenWs({ device: r.device, machine: r.machine || r.device, path: r.path });
    setScreen("detail");
  };

  const list = (
    <ListPane
      mark="data-list"
      name={paneName}
      pane={listPane}
      narrow={narrow}
      seam={{ "data-list-seam": "" }}
      seat="side"
      // Pull to refresh (the redesign): the rows and the machines asked
      // again. See `use-pull-refresh` for what it is not yet.
      onRefresh={refresh}
      head={
      <PaneBar
        seat="side"
        title={paneName}
        // Files and browser search machines, because machines are what
        // they list — so they borrow the machine pane's label rather
        // than growing one that names a thing the box cannot find.
        // **Three labels, because three panes list three things**
        // (2026-09-17 ruling 50). The workspace pane's box searches
        // names, machines and paths, so it cannot borrow the machine
        // pane's word any more than it could borrow the sessions'.
        searchLabel={
          sessions
            ? gui.search_sessions
            : landing === "workspaces"
              ? gui.search_workspaces
              : gui.search_devices
        }
        query={queries[landing]}
        onQuery={setQuery}
        filterLabel={paneFilter ? gui.filter : undefined}
        // Every list pane narrows now — sessions, machines, workspaces
        // (`paneFilter`), each by its own axes and all by one rule. The
        // two browsing landings have no list pane at all, and their box
        // belongs to a tab (`AddressBar`).
        filter={
          paneFilter
            ? { options: paneFilter.options, chosen: words[landing], onToggle: toggleWord }
            : undefined
        }
        axes={paneFilter?.axes}
        arrange={
          sessions
            ? {
                label: gui.arrange,
                options: arrange().map((a) => ({ key: a.key, label: a.label })),
                chosen: arrangeBy,
                onChoose: chooseArrange,
              }
            : undefined
        }
        // Nothing gets created from the files or browser panes yet, and
        // an empty "+" is worse than no "+": it is a menu that teaches
        // people not to open menus.
        actions={sessions ? sessionActions : landing === "devices" ? deviceActions : undefined}
        // The workspace pane's one creation road: the cross-machine
        // directory picker (工作区批②). It was drawn and unpressable
        // for one batch, which is what the empty state used to point
        // away from and now points at.
        create={
          landing === "workspaces"
            ? {
                label: gui.ws_add_ask,
                mark: "data-ws-add",
                onSelect: () => setDialog("ws-add"),
              }
            : undefined
        }
      />
      }
    >
      <>
        {/* ⚠️ 墓碑 (2026-09-27): 「够不着本机的 khor」 stood here as a
            banner over the list. Stale / offline is one `caption` line
            at R0's right end (components/RegionState, 20-states.md),
            never a banner — every landing's R0 carries it now: `Region
            caption` on the detail pages and the overviews, the tab
            strip's `tail` on 会话 with tabs open, and 额度's own Region. */}
        {sessions ? (
          <SessionsList
            rows={rows}
            devices={devices}
            loading={!rowsRead}
            query={queries.sessions}
            words={words.sessions}
            selected={selected}
            onSelect={onSelect}
            onPin={onPinSession}
            onClose={onCloseSession}
            pinFailed={pinFailed}
            closeRefused={closeRefused}
          />
        ) : landing === "workspaces" ? (
          <WorkspacesList
            devices={devices}
            query={queries.workspaces}
            words={words.workspaces}
            reloadAt={wsAdded}
            selected={openWs}
            // The chevron on the open row. **Not on the narrow face**:
            // there the list is a screen and the tree the next one, so
            // there is no column to fold.
            treeOpen={!treePane.collapsed}
            onTree={narrow ? undefined : () => treePane.toggle()}
            onOpen={openWorkspace}
          />
        ) : (
          <DevicesList
            rows={devices}
            loading={!devicesRead}
            query={queries[landing]}
            words={words.devices}
            onPin={onPinDevice}
            onForget={onForgetDevice}
            forgetRefused={forgetRefused}
            onPause={onPauseDevice}
            pauseRefused={pauseRefused}
            // 头像… on a row writes that machine's face for everyone; the
            // table is re-read, the way a rename's is.
            onRestyled={() => {
              refresh().catch(() => {});
            }}
            // One question left: the devices pane opens a machine's
            // card. Files and browser were the other two, and each
            // stopped being one when it stopped having a machine list
            // (`DevicesList` says why onOpen is a prop).
            onOpen={onSelectDevice}
            selected={selectedDevice}
            pinFailed={pinFailed}
          />
        )}
      </>
    </ListPane>
  );

  // Every pane has something to open now, and each gets the component
  // that can say something true about what it opened.
  //
  // **The devices pane with nothing picked is the exception, and it is
  // not a counter-example.** What used to sit there was an empty card,
  // for exactly the reason above: there was nothing true to say. There is
  // now — the whole mesh, which is what that pane is about — so the space
  // holds the map rather than a sentence about being empty.
  const openDevice = devices.find((d) => d.id === selectedDevice) ?? null;
  /** R0's name on a 详情 page (docs/SHELL.md 第十五轮 第 5 条): the picked
      thing where something is picked, the landing where nothing is.
      **Said once** — the board under it does not say it again. */
  const pageTitle =
    landing === "devices" && openDevice
      ? openDevice.shown
      : landing === "workspaces" && openWs
        ? openWs.path.split("/").filter(Boolean).pop() || openWs.machine
        : // Nothing picked: the head is empty (the redesign). The
          // landing's name and count are the left block's head now, and
          // saying them again over the content is the name said twice.
          "";
  // (Both browsing landings' starting points moved in with the shell,
  // which reads the device table itself — `BrowseLanding`.)
  /** The open machine's VS Code, with no folder — the fourth verb
      (2026-09-30, 「机器页也加一个 VS Code 图标，打开这台机器的 VS Code，
      不指定目录」). **Held here, by the shell**, for the workspace pane's
      reason: the verbs are drawn in R0 on the wide face and in the
      machine card's narrow head on the other, and the window's handle
      and its server must outlive that move. `null` while no machine is
      open, which gives the far server back (`components/CodeWindow`). */
  //
  // **Only where both ends are computers** (docs/IOS.md 第二步): the
  // editor runs on that machine, so one that cannot host has none; and
  // its window opens on this one, so a phone — one window, the
  // screen's — has nowhere to put it. Asked of neither, the switch is
  // not drawn and the far machine is never asked whether it has one.
  const machineCode = useCodeWindow(
    landing === "devices" && openDevice && openDevice.hosts && hosts ? openDevice.name : null,
    "",
  );
  /** The machine card's four verbs, **at R0's right end**
      (docs/SHELL.md 第十七轮 §3, D9): page buttons live in the toolbar,
      never as a row in the board. Only while a machine is open. */
  const machineVerbs =
    landing === "devices" && openDevice ? (
      <MachineVerbs
        row={openDevice}
        code={machineCode}
        sessionsOn={machineBody === "sessions"}
        onSessions={() => setMachineBody((b) => (b === "sessions" ? "dashboard" : "sessions"))}
        filesOn={machineBody === "files"}
        onFiles={() => setMachineBody((b) => (b === "files" ? "dashboard" : "files"))}
        // 浏览 in place (round 14): the third switch, exclusive with the
        // other two. ⚠️ 墓碑: it opened a new tab on the 浏览 landing.
        webOn={machineBody === "web"}
        onWeb={() => setMachineBody((b) => (b === "web" ? "dashboard" : "web"))}
        // The sessions pane's own wizard, opened with this machine picked
        // (2026-09-06:「会话是对应会话页的新建 session」).
        onNewSession={(row) => openNewSession(row.name)}
        // ⚠️ 墓碑 (2026-10-02): 文件 opened the 工作区 landing on this
        // machine's home (`openWs` with an empty path). It is a switch of
        // this page now; the files are drawn in place (`MachineFiles`).
      />
    ) : undefined;

  const detail = sessions ? (
    // The conversation in this pane is painted inside the workspace it
    // is running in, as far as its paths are concerned (2026-09-18
    // ruling 54). **Null where the row has no directory**, which is
    // `linkHome`'s own default and leaves every path as the prose it
    // already was.
    <LinkHomeProvider value={sessionHome}>
    <DetailPane
      row={selectedRow}
      narrow={narrow}
      day={selectedRow ? dayOf(selectedRow.id) : undefined}
      onBack={() => setScreen("list")}
      onNew={() => openNewSession()}
      // The way to the other open tabs on the face that has no strip:
      // the count is the button (`TabCount`), and pressing it swaps
      // this pane for the switcher. Wide has the strip above the pane,
      // so it hands nothing in.
      tabs={
        narrow && openTabs.length > 0 ? (
          <TabCount n={openTabs.length} onPress={() => setSwitchingTabs((v) => !v)} open={switchingTabs} />
        ) : undefined
      }
      // The network's now, painted from the shell's own polled state —
      // rows and devices, both already on their beats. It asked for a
      // third thing until 2026-08-29 (the spending tally, for the 今天
      // line) and asks for nothing of its own now.
      empty={
        <MeshNow
          rows={rows}
          devices={devices}
          onPick={onSelect}
          onDevice={(d) => {
            setLanding("devices");
            onSelectDevice(d);
          }}
        />
      }
    />
    </LinkHomeProvider>
  ) : landing === "workspaces" ? (
    // One workspace's files. **`FilesPane` unchanged from when it was a
    // browsing tab's body** (2026-09-11 工作区替代文件): what it does —
    // list one machine's directory, step into it, take a file — is the
    // same act whether the tree above it is a project or somebody's
    // home. What left with the old landing is the strip and the address
    // bar, not this.
    //
    // `openWs` null means the landing is on its list with nothing
    // chosen, which on the wide face is the list beside an empty
    // detail and on the narrow one is the list alone.
    openWs ? (
      <WorkspacePane
        // Remounted per workspace: the tree's position, the open file
        // and both switches are this workspace's, and carrying them
        // into the next one would show somebody a path that is not
        // under the project they just opened.
        key={`${openWs.device} ${openWs.path}`}
        treePane={treePane}
        machine={openWs.machine}
        // **Compared by channel, never by the name on the glass**: the
        // shown name is the user's to retype, and a machine renamed to
        // match another's label must not start answering for it
        // (`DeviceRow::name` against `DeviceRow::shown`).
        //
        // No `me` row yet is not「this is another machine」, it is「not
        // known yet」— and the direction that errs safely is drawing a
        // 拉过来 that is merely pointless for one poll, not hiding the
        // one action a remote tree has.
        here={devices.some((d) => d.me && d.name === openWs.machine)}
        root={openWs.path}
        rows={rows}
        // **Resolved here, from the id**, because the device table is
        // the shell's and a second copy of the id→channel lookup is a
        // second copy that can disagree (`SessionRow::home` is derived
        // in the node for exactly that reason). `null` where the table
        // has no row for this machine — a workspace outlives its
        // machine leaving the table (工作区批①), and its column then
        // says nothing rather than somebody else's work.
        channel={devices.find((d) => d.id === openWs.device)?.name ?? null}
        // The same four the sessions landing gets: one project's column
        // is the same rows with the same verbs (工作区批③).
        onPin={onPinSession}
        onClose={onCloseSession}
        pinFailed={pinFailed}
        closeRefused={closeRefused}
        // **The same arrangement the landing uses, because there is
        // one `rows` poll and the node is what orders it** (工作区批⑩).
        // A column with a sort of its own would be this layer
        // re-deriving a judgment the backend already made.
        arrange={{
          label: gui.arrange,
          options: arrange(),
          chosen: arrangeBy,
          onChoose: chooseArrange,
        }}
        // 「+」 in that column: the wizard, on this workspace's machine
        // and in its directory (工作区批⑩) — and what it opens stays
        // here (2026-09-16 ruling 34).
        onNewSession={(m, dir) => openNewSession(m, dir, true)}
        opened={openedInWs}
        onOpenedSeen={() => setOpenedInWs(null)}
        // A file a path in a conversation on the **sessions** landing
        // asked for, once this is the workspace that holds it (2026-09-18
        // ruling 54). Only where this pane is the one it named: a
        // request for another project must not open a file here.
        reveal={
          reveal !== null &&
          openWs !== null &&
          // **A machine's home is not a workspace and takes no
          // request.** Empty means「browse that disk」 (`openWs`'s own
          // head), and `${""}/` is a prefix every absolute path has —
          // so the test has to say so rather than rely on the string.
          openWs.path !== "" &&
          reveal.path.startsWith(`${openWs.path}/`)
            ? reveal
            : null
        }
        onRevealSeen={() => setReveal(null)}
        narrow={narrow}
        listOpen={!narrow && !listPane.collapsed}
        onBack={() => setScreen("list")}
      />
    ) : narrow ? null : (
      // **The landing's overview** (10-layout 「每个落地页有一个总览」,
      // 2026-09-28): the workspaces this face opened last, each with its
      // machine, path, branch and last change — the board was a blank
      // slab here. Wide only: on the narrow face the list is the screen.
      <WorkspacesNow devices={devices} onOpen={openWorkspace} />
    )
  ) : landing === "devices" && !openDevice && selectedDevice !== null && !devicesRead ? (
    // A machine asked for (⌘K) before the device table has answered
    // once: its card has never been read, so the board shows the
    // skeleton (components/RegionState) — not the map, which is the
    // answer to 「nothing picked」.
    <RegionState state="loading-never" />
  ) : landing === "devices" && !openDevice ? (
    // The map's one home (2026-09-28): 待批 rides each seat, counted off
    // the rows the shell already holds.
    <MandalaMap rows={devices} onOpen={onSelectDevice} selected={selectedDevice} waiting={waitingOn(devices, rows)} />
  ) : landing === "devices" ? (
    <MachineCard
      row={openDevice}
      narrow={narrow}
      // An open file is left first (`MachineFiles`, one back button).
      onBack={() => (fileBack ? fileBack.go() : setScreen("list"))}
      hooks={hooks}
      hooksFailed={hooksFailed}
      onInstallHooks={onInstallHooks}
      onUninstallHooks={onUninstallHooks}
      actions={narrow ? machineVerbs : undefined}
      body={
        !openDevice ? undefined : machineBody === "web" ? (
          <MachineWeb key={openDevice.id} machine={openDevice} devices={devices} />
        ) : machineBody === "files" ? (
          <MachineFiles key={openDevice.id} machine={openDevice.name} here={openDevice.me} onOpenFile={setFileBack} />
        ) : machineBody === "sessions" ? (
          <div data-machine-sessions data-nav-clear className="min-h-0 flex-1 overflow-y-auto px-rows-inset">
            <h2 className="flex min-h-touch items-end px-[calc(var(--page-inset)-var(--rows-inset))] pt-in pb-xxs text-group text-muted-foreground">
              {gui.machine_sessions}
            </h2>
            {/* The same rows and verbs the sessions landing draws, only
                this machine's. Opening one goes to the sessions landing,
                where its tab lives. */}
            <SessionsList
              rows={rows.filter((r) => r.home === openDevice?.name)}
              devices={devices}
              loading={!rowsRead}
              query=""
              words={[]}
              selected={null}
              onSelect={(r) => {
                setLanding("sessions");
                onSelect(r);
              }}
              onPin={onPinSession}
              onClose={onCloseSession}
              pinFailed={pinFailed}
              closeRefused={closeRefused}
            />
          </div>
        ) : undefined
      }
    />
  ) : null;


  const dialogs = (
    <>
      {/* 退出时还有草稿 (ruling 39). The app's half — a browser is
          asked by `beforeunload` in its own words and never sees this.
          No 存 button: `CloseDraftDialog`'s head says why. */}
      <CloseDraftDialog
        name={null}
        quitting={quitting ?? undefined}
        onSave={() => undefined}
        onDrop={() => {
          setQuitting(null);
          quitNow();
        }}
        onKeep={() => setQuitting(null)}
      />
      <ImportDialog
        open={dialog === "import"}
        onOpenChange={(o) => setDialog(o ? "import" : null)}
        // The first one imported is what the person came to see, the
        // way a freshly opened session is (`NewSessionDialog` below).
        onImported={(ids) => {
          if (ids.length === 0) return;
          setLanding("sessions");
          sessionTabs.open(ids[0]);
          setScreen("detail");
        }}
      />
      <NewSessionDialog
        open={dialog === "new"}
        onOpenChange={(o) => setDialog(o ? "new" : null)}
        machine={newSessionOn}
        dir={newSessionAt}
        // The freshly-opened session is what the person came to see —
        // on the sessions landing, on its row, whichever pane the
        // wizard was opened from (the machine card opens it too, and
        // its person wants the same thing). The selection outlives the
        // poll that has not listed the row yet, by the guard above.
        onOpened={(id) => {
          // **Made from a workspace: stay in the workspace** (ruling
          // 34). The session is that project's, the column beside its
          // files is where it belongs, and a landing change here was
          // khor walking somebody out of the screen they were working
          // on (「应该是在工作区就地打开」).
          if (newSessionInPlace) {
            setOpenedInWs(id);
            return;
          }
          setMorePage(null);
          setLanding("sessions");
          sessionTabs.open(id);
          setScreen("detail");
        }}
      />
      {/* 「+」 on the workspace pane. **The picker validates nothing and
          this is where the validating happens**: `wsAdd` probes the
          path on that machine and refuses in that machine's own words,
          which the picker then shows without closing — the person is
          still standing where they pressed, and the path they typed is
          still in the box to fix (`components/DirPicker`). */}
      <DirPicker
        open={dialog === "ws-add"}
        onOpenChange={(o) => setDialog(o ? "ws-add" : null)}
        onPick={async (pick) => {
          await wsAdd(pick.machine, pick.path);
          setWsAdded((n) => n + 1);
        }}
      />
      <InviteDialog open={dialog === "invite"} onOpenChange={(o) => setDialog(o ? "invite" : null)} />
      {/* The machine page's VS Code: its consent and its refusal. */}
      {machineCode.dialog}
      <JoinDialog open={dialog === "join"} onOpenChange={(o) => setDialog(o ? "join" : null)} />
      {/* (The root's words were asked for here, once, on the machine that
          made the root — 0e4fcaa. Since 2026-10-01 they wait on 「…」 →
          安全 until somebody presses 「显示…」: `SecurityPage`.) */}
      {/* (设置 was the fourth thing here — a dialog — until 2026-09-19.
          It is a page now, drawn beside the rail with its three
          siblings; see `morePage`.) */}
    </>
  );

  /**
   * The open page behind 「…」 or the avatar, or `null`.
   *
   * **A restyle repaints this machine everywhere at once**, because
   * `refresh` fetches rows and devices together and every face on screen
   * — the rail's, the device row's, each session row's — is painted from
   * that one answer. There is no path by which two of them could be
   * showing different styles.
   */
  /** Out of a 「…」 page. **Wide**: back to the landing it was opened
      over (R0's 「×」). **Narrow**: one screen back, to 更多's list — the
      page's head there is 「‹」 like every detail screen's (10-layout
      窄档, 2026-09-28), and 「‹」 that skipped the list would not be
      「返回」. */
  /**
   * **A session on the narrow face: content on top, the way back and the
   * tabs at the foot** (`components/SessionBar`, 2026-10-02, the owner:
   * 「上方完全交给内容，下方是操作与标签页」). Whenever an open session is
   * the screen — never on the list, the switcher, a 「…」 page — and on
   * the width alone: a Mac window dragged narrow gets it too. Inside it
   * the window's first row (L0) and the landings' bar are not drawn; the
   * bar's 「‹」 is the way back to the list, which keeps both.
   */
  const inSession =
    narrow && sessions && morePage === null && !switchingTabs && screen === "detail" && selected !== null && openTabs.length > 0;
  /** …and a browsing page is the same screen (the redesign, 2026-10-02:
      the browser takes the session page's layout): no head, no landings'
      bar; the session bar at its foot. */
  const fullScreen = inSession || (narrow && browses && morePage === null && screen === "detail");
  const sessionScreen = (
    <SessionScreen
      faces={tabFaces}
      active={selected ?? ""}
      onFocus={(id) => {
        sessionTabs.focus(id);
        setScreen("detail");
      }}
      // The last tab's × leaves the page with it: there is no session
      // left for this screen to be, and the list is where it came from
      // (the gone-row rule's answer, above, for a row that ended).
      onClose={(id) => {
        if (openTabs.length === 1) setScreen("list");
        sessionTabs.close(id);
      }}
      // Once the strip scrolls, its count opens the switcher — the
      // narrow shell's own screen, with its 「‹」 back to this one.
      onSwitch={() => setSwitchingTabs(true)}
      switching={switchingTabs}
      onBack={() => setScreen("list")}
      caption={stale ? gui.backend_unreachable : undefined}
    >
      {detail}
    </SessionScreen>
  );

  const leaveMorePage = () => (narrow ? setScreen("list") : setMorePage(null));
  const morePageEl =
    morePage === null ? null : morePage === "network" ? (
      <NetworkPage
        devices={devices}
        onClose={leaveMorePage}
        // ⚠️ 墓碑 (2026-09-30): 网络 carried 邀请设备… / 加入网络… at R0's
        // right end, the machines landing's two acts a second time. The
        // owner: 「机器页点 ＋ 就是邀请设备和加入网络，更多里的网络页就不要
        // 有右上方两个邀请、加入了」 — they are ＋'s on 机器, and only there.
        onChanged={() => {
          // The device table is the thing that changed, and every
          // surface on this screen paints off it — the rail's foot, the
          // machines landing and its map. One refresh, the same one a
          // forget has always got.
          refresh().catch(() => {});
        }}
      />
    ) : morePage === "settings" ? (
      <SettingsPage onClose={leaveMorePage} onSettingChanged={settings.reread} />
    ) : morePage === "appearance" ? (
      <AppearancePage onClose={leaveMorePage} onSettingChanged={settings.reread} me={me} />
    ) : morePage === "security" ? (
      <SecurityPage
        onClose={leaveMorePage}
        lock={lock.facts}
        onLockChanged={lock.reread}
        onLockNow={lock.now}
        onSettingChanged={settings.reread}
        devices={devices}
        onReset={() => {
          refresh().catch(() => {});
          lock.reread();
        }}
      />
    ) : morePage === "doctor" ? (
      <DoctorPage onClose={leaveMorePage} />
    ) : (
      <IdentityPage devices={devices} onClose={leaveMorePage} />
    );

  /**
   * **更多's L2** (docs/SHELL.md 第十七轮 §1): its six sections, in the
   * spec's order, as a list beside the page — System Settings' shape.
   * 「…」 used to open a menu of four pages; it opens this landing now,
   * and every door that menu had is a row here (网络 / 通用 / 安全 / 诊断)
   * or, on the narrow bar, one of the folded landings at the top.
   *
   * `data-more-open` keeps the key the menu items wore, so every walk
   * that pressed 「…」 then a page still presses the same two things.
   */
  const moreList = (
    <nav
      data-more-list
      aria-label={gui.rail_more}
      // ↑ ↓ Home End between the sections, System Settings' sidebar.
      onKeyDown={(e) => walkList(e, "[data-more-landing], [data-more-open]")}
      className="flex min-h-0 flex-1 flex-col overflow-y-auto bg-side py-1"
    >
      {/* The list page's title, 设置 — in the head row on the narrow face,
          a row of its own under the window's corner on the wide one (the
          same two places `PaneBar` puts a landing's). */}
      {narrow ? (
        <NarrowHead>
          {() => <h1 data-list-title className="min-w-0 flex-1 truncate text-title-lg">{gui.nav_settings}</h1>}
        </NarrowHead>
      ) : (
        <SideSeat>
          <h1 data-list-title className="min-w-0 flex-1 truncate text-title-lg">
            {gui.nav_settings}
          </h1>
        </SideSeat>
      )}
      {/* **这台机器 first, on the wide face** (the owner, 2026-10-02: its
          face left the top row and lives in 更多): this machine's face and
          name, over the word for the page. Narrow keeps it in its place
          among the sections; its 更多 menu already leads to it. */}
      {!narrow && (
        <button
          type="button"
          data-more-open="identity"
          data-more-me
          data-on={morePage === "identity"}
          onClick={() => {
            setMorePage("identity");
            setScreen("detail");
          }}
          className={cn(
            "mx-in mb-xxs flex min-h-row-2 items-center gap-row rounded-md px-3 text-start text-headline",
            morePage === "identity" ? "bg-fill-strong" : "hover:bg-fill",
          )}
        >
          <span className="flex w-list-face flex-none items-center justify-center">
            <MachineAvatar face={me?.face ?? null} className="size-avatar flex-none" />
          </span>
          <span className="flex min-w-0 flex-col">
            <span className="truncate">{me?.shown ?? moreName("identity")}</span>
            <span className="truncate text-callout text-muted-foreground">{moreName("identity")}</span>
          </span>
        </button>
      )}
      {morePageOrder.filter((p) => narrow || p !== "identity").map((p) => (
        <button
          key={p}
          type="button"
          data-more-open={p}
          data-on={morePage === p}
          onClick={() => {
            setMorePage(p);
            setScreen("detail");
          }}
          // **A list row like every other list's** (the audit fix plan
          // F2.1): the glyph in the face's slot so the words start on the
          // x the 这台机器 row's do, the title `headline` whether or not it
          // is the picked one, and the hairline under the words.
          className={cn(
            "mx-in flex min-h-touch items-stretch gap-row rounded-md px-3 text-start text-headline",
            // The selected row wears the block only while it is the one
            // on screen — on the narrow list the page is a screen away.
            morePage === p && !narrow ? "bg-fill-strong" : "hover:bg-fill",
          )}
        >
          {/* The page's glyph (2026-09-27): the four drawn for these
              doors, 外观's light-and-dark, and 这台机器's own face — the
              same face the rail's foot wears for it. `--nav-glyph` wide,
              the size every menu row gives its glyph. */}
          <span data-more-glyph={p} className="flex w-list-face flex-none items-center justify-center text-rail-ink [&>svg]:size-nav-glyph">
            {p === "identity" ? (
              // A face is a full disc and a glyph is not: at 16 it reads
              // the same size as the drawn marks beside it.
              <MachineAvatar face={devices.find((d) => d.me)?.face ?? null} className="size-4" />
            ) : (
              (() => {
                const Glyph = MORE_GLYPH[p];
                return <Glyph className="size-nav-glyph" />;
              })()
            )}
          </span>
          <span
            className={cn(
              "flex min-w-0 flex-1 items-center gap-row border-b py-1",
              morePage === p && !narrow && "border-transparent",
            )}
          >
            <span className="min-w-0 flex-1 truncate">{moreName(p)}</span>
            {narrow && <IconChevronRight className="size-affordance flex-none text-tertiary" />}
          </span>
        </button>
      ))}
    </nav>
  );

  /**
   * The two browsing landings, which are neither half of the
   * list-and-detail pair either — for a different reason from a 「…」
   * page's, and the two reasons are worth keeping apart.
   *
   * A 「…」 page is not a landing at all. These are: they are
   * simply **browsing** landings rather than **inbox** ones
   * (docs/UX.md 姿势 — 收件箱型 / 浏览型). 会话 and 机器 are lists a
   * person keeps an eye on, so the list is furniture and stays on
   * screen. A place is not watched, it is walked: which machine is a
   * step on the way, and a permanent column of machines is a column
   * spent on a question that was answered the moment the tab opened.
   * What replaces it is the browser's own answer — tabs, one per
   * place, and an address bar.
   *
   * Same on both faces, which is what the wide/narrow rule already
   * says: the strip is every tab at once where there is room, a count
   * and a switcher where there is not (`TabStrip`).
   *
   * **Tombstone (2026-09-11, 工作区替代文件).** This said "one
   * component, two kinds, and nothing else differs — now with a second
   * customer to test it against rather than one". The disk kind left
   * with its landing: browsing a directory happens inside a workspace
   * now, where the list beside it is worth watching. So the shell is
   * back to one customer, and the claim docs/DESIGN.md 浏览型外壳 makes
   * is back to being untested by a second one.
   *
   * **It is kept rather than folded back into the page landing**, and
   * the reason is dated rather than hopeful: the VS Code tab is the
   * next kind (a machine and a workspace, opened over a tunnel), and
   * inlining the shell now means writing it a third time then. If that
   * kind does not land, this abstraction has one customer and should
   * be inlined — that is the line for whoever reads this next.
   */
  const browsing = { kind: PAGES, tabs: pageTabs };
  const browsingLanding = (
    <BrowseLanding
      // **Keyed by the kind, and that is load-bearing.** A kind carries
      // a hook (`BrowseKind.useSecond`), so two kinds under one mounted
      // shell would change which hooks run — the one thing React cannot
      // survive. There really are two now, and switching landings is
      // exactly the swap that would do it; the key makes it a remount.
      key={browsing.kind.kind}
      kind={browsing.kind}
      devices={devices}
      tabs={browsing.tabs}
      narrow={narrow}
      // **The shell's own column width, since 批二** (docs/SHELL.md 第
      // 2 条): the tabs are a pane here now, so this landing has the
      // same seam and the same 「再按一次收起」 as every other one.
      pane={narrow ? null : listPane}
      caption={stale ? gui.backend_unreachable : undefined}
      // Narrow: the tabs are the list screen and a page the detail one,
      // the same pair every landing uses (the owner, 2026-10-02).
      screen={screen}
      onOpen={() => setScreen("detail")}
      onBack={() => setScreen("list")}
    />
  );

  /**
   * Locked, and therefore **the only thing in this document**
   * (docs/MORE.md 安全: 「页面里不留任何会话内容——不是盖一层,卸掉」).
   *
   * Two returns rather than one, and the first is not caution about a
   * flash: `useLock` starts locked (the safe direction — a face that
   * came up open and locked a frame later has already shown what it was
   * meant to hide), so a shell that drew the lock screen before the
   * store answered would flash one on every ordinary launch. A blank
   * ground for that one frame says nothing about anybody's session.
   */
  if (!lock.settled) return <div data-lock-wait className="h-dvh bg-background" />;
  if (lock.locked) {
    // Bound to a local first: TypeScript will not narrow an imported
    // binding inside a closure, and the `null` here is the seam itself
    // (`api.ts` systemUnlock) rather than a value that might arrive.
    const ask = systemUnlock;
    // **And only where the machine has one** (`LockFacts.system_door`,
    // docs/IOS.md 第二步): the command asks LocalAuthentication on a Mac
    // and answers no everywhere else, so elsewhere the button did nothing.
    const systemDoor =
      ask === null || !lock.facts?.system_door
        ? undefined
        : () => {
            void ask().then((took) => {
              if (took) lock.opened();
            });
          };
    return (
      <LockScreen
        me={devices.find((d) => d.me) ?? null}
        onOpen={lock.open}
        // The app's other key, and **only where there is one**: a
        // browser has no LocalAuthentication, and a button that refuses
        // is worse than no button (`api.ts` systemUnlock).
        systemUnlock={systemDoor}
      />
    );
  }

  /** A hit from ⌘K: go where it lives (`Node::find`'s kinds). */
  const jump = (hit: { kind: string; id: string; note: string | null }) => {
    setMorePage(null);
    if (hit.kind === "session") {
      setLanding("sessions");
      sessionTabs.open(hit.id);
      setScreen("detail");
    } else if (hit.kind === "machine") {
      setLanding("devices");
      setSelectedDevice(hit.id);
      setScreen("detail");
    } else if (hit.kind === "workspace") {
      const [device, ...rest] = hit.id.split(":");
      const path = rest.join(":");
      setLanding("workspaces");
      setOpenWs({
        device,
        machine: devices.find((d) => d.id === device)?.shown ?? "",
        path,
      });
      setScreen("detail");
    } else if (hit.kind === "tab") {
      setLanding("browser");
    } else {
      setMorePage(hit.id as MorePage);
      setScreen("detail");
    }
  };
  // **The left block's first row** (docs/SHELL.md 第十三轮): macOS's
  // buttons, the one search box, the landing's 「+」. `bare` where there
  // is no column beside the rail to search — a 「…」 page, or the list
  // put away — since a box in a rail's width is a slit; ⌘K still widens
  // over it. The narrow face always has the row: there the list is the
  // screen and its head has nowhere else to be.
  //
  // **L0 is one of the window's three handles**, marked here by the
  // shell once (`components/shell.ts`, 第十五轮 第 3 条) — on the row's
  // container, so whatever a landing seats in it (its search box, its
  // 「+」) keeps its press and the blank around them moves the window.
  const sideHead = (
    <div data-side-l0 {...WINDOW_DRAG} className="flex min-w-0 flex-none flex-col">
      <SideHead
        narrow={narrow}
        bare={false}
        seatRef={seats.setHead}
        claimed={seats.claimed}
        onJump={jump}
      />
    </div>
  );

  // One shell, two width classes: wide is rail|list|detail all at once,
  // narrow is one screen at a time with the rail as a bottom bar. A
  // 「…」 page is neither half of that pair, so it takes the whole of
  // what is left of the window beside the rail (see `morePage`).
  return (
    <SettingsProvider value={settings.rows}>
    <SettingsRereadProvider value={settings.reread}>
    <SideSeatsProvider value={seats.seats}>
    <HereHostsProvider value={hosts}>
    <TooltipProvider>
      {narrow ? (
        // **As tall as `#root`, not the screen** (2026-10-02): `#root`
        // follows what the soft keyboard leaves (`lib/viewport`), and a
        // `100dvh` would not — the keyboard covers, it does not resize.
        // **Clear of a phone's status bar** (`--safe-top`) on every
        // screen but a session's, which runs its paper up to the glass
        // and keeps the edge itself (`SessionScreen`).
        <div data-nav-host={!fullScreen || undefined} className={cn("relative flex h-full flex-col", !fullScreen && "pt-safe-top")}>
          {!fullScreen && sideHead}
          {/* ⚠️ 墓碑 (2026-09-23, docs/SHELL.md 第十轮, 第十三轮): a
              `[data-traffic-room]` spacer stood here, holding the head of
              the screen clear for macOS's three buttons; then the
              window's own row. The block's first row above does that
              now, on every face and every platform. */}
          {/* `key` is what makes the arrival visible: a CSS animation
              runs when an element is inserted, and without a changing
              key React would reuse this box and the new screen would
              simply appear inside it. The key is the screen, not the
              row — moving between two sessions is not a place change,
              and re-keying on the row would replay the slide (and
              remount a live terminal) every time the list is clicked.
              `data-screen` is what app.css keys the direction off. */}
          <div
            key={morePage ? `more:${screen === "detail" ? morePage : "list"}` : screen}
            data-screen={morePage ? "more" : screen}
            ref={setDetailBox}
            className="min-h-0 flex-1 overflow-hidden"
          >
            {/* A browsing landing is one screen either way — it has no
                list screen to be the other half of a pair, so `screen`
                says nothing about it and it is keyed on its own.

                A 「…」 page is one screen too, and its own `data-screen`
                word — **not "detail"**: the slide in `app.css` says
                「你往里走了一层」, and these do not sit inside anything.
                They are opened over the whole region, the way the
                mark's place used to be, and take the same silence. */}
            {morePage
              ? screen === "detail"
                ? morePageEl
                : moreList
              : quotas
                ? (
                    // 额度 is an inbox now, so it takes the inbox
                    // posture here: the same `screen` pair the sessions
                    // landing uses, keyed by the box above so its
                    // entrance plays like every other screen change.
                    // (It is not `browses` — those have no list screen
                    // to be the other half of a pair.)
                    <QuotaLanding
                      narrow
                      stale={stale}
                      // Narrow has one screen and nothing beside it, so
                      // there is no width to remember and no seam to
                      // drag — `ListPane` takes the same `null` every
                      // other pane hands it there.
                      pane={null}
                      query={queries.quota}
                      onQuery={setQuery}
                      words={words.quota}
                      onToggleWord={toggleWord}
                      screen={screen}
                      onOpen={() => setScreen("detail")}
                      onBack={() => setScreen("list")}
                    />
                  )
                : browses
                  ? browsingLanding
                  : switchingTabs && openTabs.length > 0
                    ? tabSwitcher
                    : screen === "detail" && detail
                      ? inSession
                        ? sessionScreen
                        : detail
                      : list}
          </div>
          {/* The navigation floats over every screen but a session's,
              whose foot is its own bar (the owner, 2026-10-02: 「会话里不
              显示」), and the tab switcher, which is a step of it. */}
          {!fullScreen && !switchingTabs && nav}
          {dialogs}
          <StatusBar items={status.items} onDismiss={status.dismiss} />
        </div>
      ) : (
        <div className="flex h-dvh flex-col">
          <div className="flex min-h-0 flex-1">
          {/* **The left column** (the redesign, 方案 B). Its first row is
              the window's: the buttons' room, then the landings' glyphs
              and 更多 in one glass capsule (这台机器 is inside 更多). Its
              second row is the phone's list head (`PaneBar`): the
              landing's name, then 筛选 / ＋ and the search circle. Then the
              rows, and a hairline on the one ground to the content.
              **Pressing the open landing's glyph folds the column away**;
              the strip stays in the window's top row, at the start of the
              content's first row (`foldedLead`). Its width has one owner (docs/SHELL.md 第十五轮
              第 4 条): `--list-w`, from the one seam's state. */}
          <aside
            data-side
            data-search-glass
            hidden={folded}
            className="flex flex-none flex-col border-r bg-side"
            style={{ "--list-w": `${listPane.width}px` } as CSSProperties}
          >
            <div
              data-wide-top
              {...WINDOW_DRAG}
              // **Right-aligned with the title row under it** (the owner,
              // 2026-10-02: 「这要右对齐」): the window buttons' room at the
              // left, then the strip pushed to the right edge — 更多 over the
              // search circle, the capsule's end over 筛选 / ＋'s — on the
              // same inset (`pe-row`, the title row's) and the same gap.
              className="flex h-pane-head flex-none items-center ps-wide-lead-start pe-row select-none"
            >
              <span className="flex-1" />
              {strip}
            </div>
            {sideHead}
            <div ref={seats.setList} data-side-list-seat className="flex min-h-0 min-w-0 flex-1">
              {morePage !== null && moreList}
            </div>
          </aside>
          {/* **The right side**: the app's ground, a first row at the
              same height as the block's, and a board floating under it
              (`Region`). A 「…」 page and a landing with nothing to head
              the row get a blank band; the sessions strip and the
              browsing address bar are rows of their own. */}
          <main data-main className="relative flex min-w-0 flex-1 flex-col bg-background">
          <RegionLead.Provider value={foldedLead}>
          {morePage ? (
            <Region
              kind="detail"
              title={moreName(morePage)}
              caption={stale ? gui.backend_unreachable : undefined}
              // The way out, at R0's right end with the page's other
              // buttons (components/Toolbar, D9) — not a second head in
              // the board, which would say the page's name twice. The
              // narrow face has no R0 and keeps the frame's own head.
              // A page's own buttons (网络's two, 安全's 立即锁定) are
              // seated before this by the page itself (`PageAction`).
              // A glass circle like every head's way out (the audit fix
              // plan F4.9; W21).
              actions={
                <GlassCircle aria-label={gui.close} data-more-close onClick={() => setMorePage(null)}>
                  <IconClose />
                </GlassCircle>
              }
            >
              {morePageEl}
            </Region>
          ) : quotas ? (
            <QuotaLanding
              narrow={false}
              stale={stale}
              // **The shell's own seam, not a `w-72` of its own**
              // (第 6 条 / 第 4 条). Sharing `listPane` is what makes
              // the rail's second press work here: the thing the rail
              // toggles and the thing this landing draws are now one
              // object. It also means a width dragged on the sessions
              // pane is the width here — which is the right answer
              // for the same reason the ground is: a person drags a
              // sidebar, not five sidebars.
              pane={listPane}
              query={queries.quota}
              onQuery={setQuery}
              words={words.quota}
              onToggleWord={toggleWord}
            />
          ) : browses ? (
            browsingLanding
          ) : (
            <>
              {/* Written here, drawn in the block (`ListPane` seat="side"):
                  a pane and the thing that sizes it are one component
                  (`ListPane`), which is what kept 额度 from having
                  either for four batches. */}
              {list}
              {/* The open sessions as tabs over the board (iTerm's
                  arrangement), only on the sessions landing and only
                  when something is open — otherwise the row is a blank
                  band over the board. The strip closes a *view*; the
                  header's 关闭 ends the session, and the gone-row rule
                  then closes the tab. `ref` is the band's width source.
                  A workspace draws its own three columns and their
                  heads, so it takes the region raw. */}
              {/* `min-h-0`: this is a column inside `main`'s column now
                  (it was a row's item), and a flex item's default
                  min-height is its content — the file scroller would
                  grow to its whole file and never scroll. */}
              <div ref={setDetailBox} className="flex min-h-0 min-w-0 flex-1 flex-col">
                {sessions && openTabs.length > 0 ? (
                  <Tabs
                    glass
                    faces={tabFaces}
                    active={selected ?? ""}
                    onFocus={(id) => {
                      sessionTabs.focus(id);
                      setScreen("detail");
                    }}
                    onClose={sessionTabs.close}
                    stripData={{ "data-session-tabs": "", "aria-label": gui.session_tabs }}
                    // 过期 / 离线 on B's R0: after the strip, which then
                    // compresses its tabs into what is left.
                    caption={stale ? gui.backend_unreachable : undefined}
                  >
                    {detail}
                  </Tabs>
                ) : (
                  // **Every other page is 详情 (A)** — pick one, see it:
                  // R0 is `PageHead`, the picked thing's name or, with
                  // nothing picked, the landing's. The sessions landing
                  // with no tab open is B's overview and wears the same
                  // head (D2). An open workspace comes through here too
                  // now: its three columns are the board's content, and
                  // R0 over them is the workspace's name (第十七轮 lays
                  // the columns out as the board's own panes).
                  <Region
                    kind={sessions ? "tabs" : "detail"}
                    title={pageTitle}
                    // The open machine's verbs, and nothing else: with
                    // nothing picked R0's right end is empty. ⚠️ 墓碑
                    // (2026-09-30): the overviews stood 新会话… 导入… /
                    // 邀请设备… 加入网络… here, a second copy of what ＋
                    // offers; an act that makes something lives on ＋
                    // alone (10-layout 「一个动作只有一个家」).
                    actions={machineVerbs}
                    flush={landing === "workspaces" && openWs !== null}
                    // 过期 / 离线 (第十七轮 §6): what is on the board is the
                    // last answer, and R0 says so — never a banner inside.
                    // (An open machine's model is R0's second line and its
                    // system's mark R0's avatar, both written by
                    // `MachineCard` — `PageSubtitle`, `PageAvatar`; its
                    // version and reading age are the card's, 2026-10-01.)
                    caption={stale ? gui.backend_unreachable : undefined}
                  >
                    {detail}
                  </Region>
                )}
              </div>
            </>
          )}
          </RegionLead.Provider>
          </main>
          </div>
          {dialogs}
          <StatusBar items={status.items} onDismiss={status.dismiss} />
        </div>
      )}
    </TooltipProvider>
    </HereHostsProvider>
    </SideSeatsProvider>
    </SettingsRereadProvider>
    </SettingsProvider>
  );
}
