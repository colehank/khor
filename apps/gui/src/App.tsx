import { sessionCwdOnScreen, workspaceCwd, followsSessionCwd } from "@/lib/session-cwd";
import { startTransition, useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type CSSProperties, type ReactNode } from "react";

import { hereActs, ProgressLine, termName } from "@/components/DoingLine";
import { tabAct, useTabActs } from "@/lib/tab-acts";
import { noticesOf, useNotices } from "@/lib/pill-notices";
import { rungOffScreen } from "@/lib/bell-marks";
import { FinishedWatch, type Finished } from "@/lib/finished";
import { NEW_SESSION_EVENT, OPEN_DIR_EVENT, OPEN_PATH_EVENT, OPEN_URL_EVENT, STEP_TAB_EVENT } from "@/lib/composer-handoff";
import {
  faceHere,
  fetchDevices,
  fetchLs,
  fetchSessions,
  fetchTabs,
  notifyFinished,
  openTab,
  recentVisit,
  syncNow,
  fetchWorkspaces,
  widenWindow,
  giveBackWindow,
  inApp,
  launchOn,
  closeSession,
  forgetDevice,
  markSeen,
  nameSettings,
  onFullscreen,
  openSettings,
  onQuitRequested,
  pinDevice,
  quitNow,
  pinSession,
  setMachinePaused,
  settingsFace,
  settingsPane,
  settingsMachine,
  systemUnlock,
  trafficLights,
  trafficRoom,
  wsAdd,
  type DeviceRow,
  type SessionRow,
  type WorkspaceRow,
} from "@/api";
import { MachineAvatar } from "@/components/Avatar";
import {
  IconAppearance,
  IconBack,
  IconBrowser,
  IconChevronRight,
  IconClose,
  IconPaneSize,
  IconDevices,
  IconFiles,
  IconGear,
  IconNetwork,
  IconPulse,
  IconQuota,
  IconSessions,
  IconShield,
  IconStore,
  IconTerminal,
} from "@/components/icons";
import { CloseDraftDialog } from "@/components/CloseDraftDialog";
import { DirPicker } from "@/components/DirPicker";
import { FloatNav, MoreMenuItems, type NavSeat } from "@/components/FloatNav";
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
import { useSessionContent, type ContentView } from "@/hooks/use-session-content";
import { ContentTabs, SessionContentPane } from "@/views/SessionContent";
import { useContentNavigation } from "@/hooks/use-content-navigation";
import { MobileContentSheet } from "@/views/MobileSessionContent";
import { useBrowseTabs } from "@/browse/use-browse-tabs";
import { gui, msg } from "@/gen/catalog";
import { lasting } from "@/words";
import {
  forgetSidePlace,
  keepSidePlace,
  keepSidesOf,
  placeFile,
  setSessionSide,
  setSideWidth,
  sidePlace,
  sidePlaces,
  useSessionSide,
  useSideWidth,
  type SidePlace,
  type StoreSpot,
  type SideSeat as PlaceSeat,
  type WsSpot,
} from "@/hooks/use-session-side";
import { useDayStrips, useStripWidth } from "@/hooks/use-day-strips";
import { HIDDEN_MS, useHidden } from "@/hooks/use-hidden";
import { useWoken } from "@/hooks/use-woken";
import { LIST_PANE, roomBeside, useListWidth } from "@/hooks/use-list-width";
import { isNewTab, newTabId, useSessionTabs } from "@/hooks/use-session-tabs";
import { tabsLimitOf } from "@/browse/tab-limit";
import { useLock } from "@/hooks/use-lock";
import { useStoredSettings } from "@/hooks/use-settings";
import { SettingsProvider, SettingsRereadProvider } from "@/settings";
import { speakLang, type LangPref } from "@/lang";
import { wearTheme, type ThemePref } from "@/theme";
import { wearWindowChoice, windowChoice } from "@/lib/window-surface";
import { NARROW_QUERY, useNarrow } from "@/hooks/use-narrow";
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
import { discardAll, flushAll } from "@/lib/save-road";
import { NAV_SHOWN, useNavOrder } from "@/lib/nav-order";
import { band } from "@/lib/scroll-edge";
import { levelOf, onTreeChanged, sameMachine } from "@/lib/tree-changed";
import { cn } from "@/lib/utils";
import { type Day } from "@/views/DayStrip";
import { DetailPane } from "@/views/DetailPane";
import { SessionPlaceSheet } from "@/views/SessionPlaceSheet";
import { SIDE_ID, SessionSide } from "@/views/SessionSide";
import { DayToday } from "@/views/DayToday";
import { SideRecent } from "@/views/SideRecent";
import { useSideRecent, type RecentItem } from "@/hooks/use-side-recent";
import { useDayTotals } from "@/hooks/use-day-totals";
import { holding, LinkHomeProvider, ShowFileProvider, useKnownFiles, type LinkHome, type ShowFile } from "@/views/linkHome";
import { PAGES } from "@/views/PagesKind";
import { DevicesList, useDeviceFilter } from "@/views/DevicesList";
import { WorkspacePane } from "@/views/WorkspacePane";
import { SideWorkspace } from "@/views/SideWorkspace";
import { WorkspacesList, useWorkspaceFilter } from "@/views/WorkspacesList";
import { WorkspacesNow } from "@/views/WorkspacesNow";
import { LockScreen } from "@/views/LockScreen";
import { inSettings, moreName, morePageOrder, type MorePage } from "@/views/more/MorePage";
import { StorePage, StoreScreen } from "@/views/StorePage";
import { rememberPlace, storedPlace } from "@/lib/place";
import { AppearancePage } from "@/views/more/AppearancePage";
import { DoctorPage } from "@/views/more/DoctorPage";
import { ConnectionPage } from "@/views/more/ConnectionPage";
import { SessionPage, TerminalPage, WorkspacePage } from "@/views/more/GroupPages";
import { MyKhorPage } from "@/views/more/MyKhorPage";
import { SecurityPage } from "@/views/more/SecurityPage";
import { AboutPage } from "@/views/more/AboutPage";
import { MachineCard, MachineVerbs } from "@/views/MachineCard";
import { MachineFiles } from "@/views/MachineFiles";
import { FullScreenProvider } from "@/lib/full-screen";
import { MachineWeb } from "@/views/MachineWeb";
import { RegionState, Waiting } from "@/components/RegionState";
import { Fault } from "@/components/Fault";
import { MandalaMap, waitingOn } from "@/views/MandalaMap";
import { MeshNow } from "@/views/MeshNow";
import { AddMachineDialog } from "@/views/AddMachine";
import { AloneNote } from "@/views/PairDialogs";
import { QuotaLanding } from "@/views/QuotaLanding";
import { SessionsList } from "@/views/SessionsList";
import { Region, RegionLead } from "@/components/Region";
import { WINDOW_DRAG } from "@/components/shell";
import { useKeptScroll } from "@/hooks/use-kept-scroll";
import { useEdgeSwipe } from "@/hooks/use-edge-swipe";
import { FIND_ALL, NarrowHead, SideHead, SideSeat, SideSeatsProvider, useSideSeats } from "@/components/SideHead";
import { SessionScreen, VerbSeat } from "@/components/SessionBar";
import { useShortcuts, useTabHost, walkList } from "@/lib/shortcuts";
import { ImportDialog } from "@/views/ImportDialog";
import { NewSessionPage } from "@/views/NewSessionPage";
import { MachineChat } from "@/views/MachineChat";
import { newTabKind } from "@/components/SessionFace";

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

/** A tab on the broadcast (`wide/all`), which may stand before its row
    does: put away with nothing newer, or before the khor has two
    machines. ⚠️ 墓碑 (STORE-BRIEF phase 5): a machine's `chat/<name>`
    conversation; that session kind is gone. */
const isChatTab = (id: string | null): boolean => id === "wide/all";

/** What a road into 新建会话 knew (UX-45): the machine, the directory,
    and whether it was a workspace column's own ＋. */
type NewTabAt = { machine?: string; dir?: string; inPlace?: boolean };

/** Which of the "+" dialogs is up, if any. `tell` left on 2026-09-06:
    leaving a line is the wizard's first kind (`NewSessionPage`), one door
    for every kind of session; `new` left on 2026-10-07 (UX-45): 新建会话
    is a new tab's page. */
type PlusDialog = "import" | "add" | "add-ticket" | "ws-add" | null;

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
/** How long a pull waits for its sync round before it reads the rows
    anyway (`refresh`). */
const SYNC_WAIT_MS = 6_000;
/** How often a face on screen says so again: a third of the resident's
    patience (`ipc::FACE_BEAT` × 3), so one lost beat never drops the
    pump to its quiet pace. */
const FACE_BEAT_MS = 10_000;
/** How long the quit waits, after the last unsaved file's write was
    answered, for the panes to take their drafts off (save-a). A draft
    goes when its view re-renders on the landing — a beat, not a wait. */
const QUIT_SETTLE_MS = 600;

/** Resolves as soon as no file is unsaved, or after `ms`. */
function settled(ms: number): Promise<void> {
  return new Promise((done) => {
    if (draftCount() === 0) return done();
    const stop = watchDrafts((n) => {
      if (n !== 0) return;
      stop();
      clearTimeout(id);
      done();
    });
    const id = setTimeout(() => {
      stop();
      done();
    }, ms);
  });
}

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

/** The glyph beside each 更多 page in its list (`me` wears this
    machine's face instead, drawn where the list is). 关于 has no glyph of
    its own in the set yet and wears the gear, 设置's. */
const MORE_GLYPH: Record<Exclude<MorePage, "me">, (p: { className?: string }) => ReactNode> = {
  appearance: IconAppearance,
  terminal: IconTerminal,
  session: IconSessions,
  workspace: IconFiles,
  connection: IconNetwork,
  security: IconShield,
  about: IconGear,
  // 存储 wears its own tray (store-c): received files, not a page.
  store: IconStore,
  doctor: IconPulse,
};

/** **A dropping file's name on the pill: no extension, no count** (the
    owner 2026-10-07 13:15): the line is the whole drop's, files after
    the first are seen going by, and the long form (正在传入, done of
    all) is the hover title's and the tabs sheet's. A dot-file keeps its
    name. */
const stemOf = (name: string) => {
  const dot = name.lastIndexOf(".");
  return dot > 0 ? name.slice(0, dot) : name;
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
  // The page a back swipe uncovers has its own first row (F7.3, V01):
  // the list's head is seated there, not in the row the page on top
  // holds.
  const underSeats = useSideSeats();
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
  const windowPref = windowChoice(settings.rows);
  useEffect(() => {
    if (windowPref) wearWindowChoice(windowPref);
  }, [windowPref?.material, windowPref?.opacity, windowPref?.outline]);
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
  /** Panes holding the narrow face full screen (`lib/full-screen`): a file
      open in a workspace or on a machine page (round UX-3). */
  const [fullClaims, setFullClaims] = useState(0);
  const claimFull = useCallback(() => {
    setFullClaims((n) => n + 1);
    return () => setFullClaims((n) => n - 1);
  }, []);
  useEffect(() => onFullscreen(setFullscreen), []);
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
    // The settings window holds no drafts and closes as any window does.
    if (settingsFace) return;
    // **Unsaved edits are written first** (save-a): every open file's
    // pending text goes, and the window closes when nothing is left
    // unsaved. Only what could not be saved asks — that is the warning.
    let flushing = false;
    const stop = onQuitRequested(() => {
      if (draftCount() === 0) return false;
      if (flushing) return true;
      flushing = true;
      void flushAll()
        .then(() => settled(QUIT_SETTLE_MS))
        .then(() => {
          flushing = false;
          const n = draftCount();
          if (n === 0) quitNow();
          else setQuitting(n);
        });
      return true;
    });
    const leaving = (e: BeforeUnloadEvent) => {
      // A browser will not wait; the writes go, and it asks if any
      // file still has something unwritten.
      void flushAll();
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
  // ⚠️ 墓碑 (UX-23): `treePane`, the workspace's file tree as a seam of
  // its own beside the list, folded from the open row (2026-09-16 ruling
  // 42). The tree is the list column's now (`wsTree` below).
  /** Where the face was when it last ran (P3, `lib/place`), read once.
      **On the narrow face only the seat** (UX-14 §7, the owner
      2026-10-05: 「iOS 保持」): a cold start opens the seat's list where it
      was scrolled (`useKeptScroll`), never a pushed page — a machine, a
      workspace, a session, a 「…」 page. ⚠️ 墓碑: P3 (10-03) restored the
      pushed page too, and a relaunch went straight into the last
      workspace (「进入工作区直奔最近内容」). The wide face is unchanged. */
  const [place] = useState(() => {
    const p = storedPlace();
    return window.matchMedia(NARROW_QUERY).matches ? { landing: p.landing } : p;
  });
  const [landing, setLanding] = useState<Landing>(() =>
    landings().some((l) => l.key === place.landing) ? (place.landing as Landing) : "sessions",
  );
  const [rows, setRows] = useState<SessionRow[]>([]);
  // **Long commands that finished out of sight** (roadmap 2.4,
  // `lib/finished`), said by the system. On screen = this window shown,
  // the sessions page up, and that session's tab the one in front.
  const finished = useRef(new FinishedWatch());
  const onScreen = useRef<(id: string) => boolean>(() => false);
  const announceFinished = useCallback((now: SessionRow[]) => {
    const done = finished.current.see(now, (id) => onScreen.current(id));
    // For the gates, as `__xterm` is: what was announced, in order.
    const said = window as unknown as { __finished?: Finished[] };
    if (done.length) said.__finished = [...(said.__finished ?? []), ...done];
    for (const f of done) {
      const took = lasting(f.ms);
      const body =
        f.notice ??
        (f.exit === null ? gui.finished_on_plain(f.command, f.machine, took) : gui.finished_on(f.command, f.machine, f.exit, took));
      notifyFinished(f.title, body).catch(() => {});
      // The system's notice brings the app forward when it is pressed,
      // and says nothing more to it; so the window that comes forward
      // within a minute of a notice posted while it was in the back
      // opens that session.
      if (!document.hasFocus()) noticed.current = { id: f.id, at: Date.now() };
    }
  }, []);
  const noticed = useRef<{ id: string; at: number } | null>(null);
  const openNoticed = useRef<(id: string) => void>(() => {});
  useEffect(() => {
    const back = () => {
      const n = noticed.current;
      noticed.current = null;
      if (n && Date.now() - n.at < 60_000) openNoticed.current(n.id);
    };
    window.addEventListener("focus", back);
    return () => window.removeEventListener("focus", back);
  }, []);
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
  onScreen.current = (id) =>
    document.visibilityState === "visible" && landing === "sessions" && selected === id && (!narrow || screen === "detail");
  // Whether the open tabs are up as a list to pick from (`TabSwitcher`)
  // instead of the front tab's content. The narrow face's only way to
  // another tab — a phone has no strip. ⚠️ 墓碑 (UX-45): the wide face's
  // too, once its strip scrolled; the wide strip draws every tab now and
  // has no count to open it. Reset wherever the tabs
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
  const [selectedDevice, setSelectedDevice] = useState<string | null>(() => place.device ?? null);
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
  /** What the machine page opens on when the next machine opens, where
      something asked for more than its dashboard (a conversation's
      「在文件视图中打开」, round CHAT): read once by the reset below. */
  const bodyNext = useRef<"files" | "sessions" | null>(null);
  useEffect(() => {
    setMachineBody(bodyNext.current ?? "dashboard");
    bodyNext.current = null;
  }, [selectedDevice]);
  /** A file the machine page's files should open, stamped so asking for
      the same file twice opens it twice (`MachineFiles`' `start`). */
  const [fileStart, setFileStart] = useState<{ path: string; at: number } | null>(null);
  // Spent once the page shows something else, so the files opened by
  // hand later start from the home as they always did.
  useEffect(() => {
    if (machineBody !== "files") setFileStart(null);
  }, [machineBody]);
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
  } | null>(() => place.ws ?? null);
  /** **The workspace list drills down** (UX-23 §2, the owner 2026-10-05,
      Files and Notes' folders): opening a workspace turns the list column
      into its file tree, under a head 「‹ 工作区名」 that returns to the
      list; the content holds only the file. Kept per seat — another
      landing and back finds the tree — and not across a cold start, which
      lands on the list (UX-14 §7). */
  const [wsTree, setWsTree] = useState(false);
  // …and the browser landing's own tabs. **A second strip, not a
  // second mechanism** (浏览器接壳批): both browsing landings are the
  // same shell with a different kind, and the two stores are kept apart
  // by that kind alone (`useBrowseTabs` keys on it), so a page can
  // never turn up where a directory belongs.
  // **Watched while its landing is open** (`useBrowseTabs`'s `WATCH_MS`):
  // `.khor/tabs.json` has two writers now, and `khor tabs open` in a
  // terminal has to show up in the column without anybody reloading.
  // …and while the 附栏 is on its 浏览器 seat (UX-39 walk: the seat drew
  // the tabs as they were when the landing was last open, and a page a
  // terminal opened into it was not among them). Set below, where the
  // 附栏's state is.
  const [sideBrowsing, setSideBrowsing] = useState(false);
  const pageTabs = useBrowseTabs(PAGES.kind, landing === "browser" || sideBrowsing);
  const content = useSessionContent(landing === "sessions");
  const contentNavigation = useContentNavigation(content);
  // One portal container keeps editors mounted when the slot moves between
  // sessions/landings. React owns its children; the shell only seats the box.
  const [contentHost] = useState(() => { const el = document.createElement("div"); el.className = "session-content-host"; return el; });
  const contentParking = useRef<HTMLDivElement | null>(null);
  const seatContent = useCallback((el: HTMLDivElement | null) => {
    (el ?? contentParking.current)?.appendChild(contentHost);
  }, [contentHost]);

  const browsing = { kind: PAGES, tabs: pageTabs };
  // Narrow face only: which single screen is up. Wide ignores it.
  const [screen, setScreen] = useState<"list" | "detail">(() => place.screen ?? "list");
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
  /** 安装 khor on this SSH 主机 (the machine page's act): the add dialog
      runs it (round join-d). */
  const [promoting, setPromoting] = useState<string | null>(null);
  /* ⚠️ 墓碑 (UX-45): `newSessionOn` / `newSessionAt`, the dialog's
     machine and directory; a new tab carries its own (`newTabs`). */
  /** Bumped when this shell has changed the workspace table from
      outside the list — the pane bar's 「+」 lives up here, and the list
      owns every read it makes. One number rather than a callback,
      because what the list needs to know is 「something changed」 and
      not what: it re-reads a local table, which costs nothing. */
  const [wsAdded, setWsAdded] = useState(0);
  /** **Every road into 新建会话 is a new tab** (UX-45, the owner
      2026-10-07), with what the road knew filled in: the machine card's
      and the machines row's 开会话 (the machine), the workspace column's
      ＋ (machine and directory, `inPlace`), a terminal's 发给智能体 and
      在这里新开会话 (machine, directory, the handed block). `inPlace` is
      the workspace column's own (2026-09-16 ruling 34): the session it
      makes belongs to the project somebody is standing in, so once made
      the shell goes back there and hands the id down. Through a ref:
      the new tab is opened by state declared further down. */
  const openNewTabRef = useRef<(at?: NewTabAt) => void>(() => {});
  const openNewSession = useCallback((machine = "", dir = "", inPlace = false) => {
    openNewTabRef.current({ machine, dir, inPlace });
  }, []);
  // A terminal block's "Send to Agent" (roadmap 2.3, `lib/composer-handoff`):
  // the wizard on that machine, in that directory.
  useEffect(() => {
    const asked = (e: Event) => {
      const { machine, dir } = (e as CustomEvent<{ machine: string; dir: string }>).detail;
      openNewSession(machine, dir);
    };
    window.addEventListener(NEW_SESSION_EVENT, asked);
    return () => window.removeEventListener(NEW_SESSION_EVENT, asked);
  }, [openNewSession]);
  /** The id a new tab opened from a workspace's own column left there,
      which the column reads. ⚠️ 墓碑 (UX-45): `newSessionInPlace`, while
      the dialog was up; the new tab carries it (`NewTabAt.inPlace`). */
  /** Back from a workspace column's new tab (UX-45, `WorkspacePane`
      `back`). */
  const [wsBack, setWsBack] = useState(false);
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
  const [morePage, setMorePage] = useState<MorePage | null>(() =>
    // **The settings window is 设置's seven and nothing else** (UX-10;
    // UX-16): it opens on the pane it was asked for, or 外观, and never on
    // a landing, 我的 khor or 诊断.
    settingsFace
      ? morePageOrder.some((p) => p === settingsPane && inSettings(p))
        ? (settingsPane as MorePage)
        : "appearance"
      : (morePageOrder as readonly string[]).includes(place.more ?? "")
        ? (place.more as MorePage)
        : null,
  );
  // …and written back whenever it changes, for the next start — by the
  // main window only: the settings window's pane is not where the app was.
  useEffect(() => {
    if (!settingsFace) rememberPlace({ landing, more: morePage, screen, device: selectedDevice, ws: openWs });
  }, [landing, morePage, screen, selectedDevice, openWs]);
  // The settings window: never off its pages, its title the pane's name,
  // and a later ⌘, from the menu brings it to the pane it names.
  useEffect(() => {
    if (!settingsFace) return;
    if (morePage === null || !inSettings(morePage)) setMorePage("appearance");
    else nameSettings(moreName(morePage)).catch(() => {});
  }, [morePage]);
  useEffect(() => {
    if (!settingsFace || !inApp) return;
    let stop: (() => void) | undefined;
    let gone = false;
    void import("@tauri-apps/api/event").then(({ listen }) =>
      listen<string>("settings:pane", (e) => {
        if (morePageOrder.some((p) => p === e.payload && inSettings(p))) setMorePage(e.payload as MorePage);
      }).then((un) => (gone ? un() : (stop = un))),
    );
    return () => {
      gone = true;
      stop?.();
    };
  }, []);
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

  const [recognitionMachine, setRecognitionMachine] = useState<string | null>(settingsFace ? settingsMachine : null);
  useEffect(() => {
    if (!settingsFace || !inApp) return;
    let stop: (() => void) | undefined;
    let gone = false;
    void import("@tauri-apps/api/event").then(({ listen }) =>
      listen<string>("settings:machine", (e) => setRecognitionMachine(e.payload))
        .then((un) => (gone ? un() : (stop = un))),
    );
    return () => { gone = true; stop?.(); };
  }, []);

  const hidden = useHidden();
  // Back from the background with the node caught up (docs/IOS.md 第三步):
  // the rows and the machines ask again at once, the way `hidden` makes
  // them (`use-woken`).
  const woken = useWoken();

  // **Telling the resident a face is on screen** (round NET-6): its pump
  // syncs every 5 s while one is and every 30 s while none is, and a face
  // arriving gets a round at once — so a person coming back never reads
  // rows half a minute old. Said on arrival, again every FACE_BEAT_MS
  // while on screen, and once more on the way out.
  useEffect(() => {
    if (hidden) {
      faceHere(false).catch(() => {});
      return;
    }
    faceHere(true).catch(() => {});
    const t = window.setInterval(() => faceHere(true).catch(() => {}), FACE_BEAT_MS);
    return () => window.clearInterval(t);
  }, [hidden]);

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
          announceFinished(rows);
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
    // **One question at a time.** The beat is 2 s and the node may take
    // up to its 5 s deadline, so asking on every beat stacked three
    // questions whose answers landed in any order — the road word on a
    // row flipped between two readings that were both seconds old
    // (audit/net.md F4). A beat that finds a question still out skips;
    // the answer that comes is the newest there is.
    let asking = false;
    const tick = () => {
      if (asking) return;
      asking = true;
      fetchDevices()
        .then((rows) => {
          if (!live) return;
          setDevices(rows);
          setDevicesRead(true);
        })
        // Silent on purpose: 「后端连不上」 belongs to the rows above,
        // which asks more often and would already be saying it. The rows
        // already on screen stay — the last reading, not a blank.
        .catch(() => {})
        .finally(() => {
          asking = false;
        });
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

  const onSelect = useCallback((row: SessionRow) => {
    sessionTabs.open(row.id);
    if (row.unread > 0 && row.id !== "wide/all") {
      // Looking at it is the seen semantics; the watermark replicates,
      // so this clears the badge on every device.
      markSeen(row.id).catch(() => {});
    }
    setScreen("detail");
    // `open` is a stable callback; the object around it is not.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sessionTabs.open]);

  openNoticed.current = (id) => {
    const row = rows.find((r) => r.id === id);
    if (!row) return;
    setLanding("sessions");
    onSelect(row);
  };

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
  // **Pulling a list down is a sync round** (the redesign): every machine
  // is asked now (`syncNow`, the node's `sync_now`), then the rows and the
  // machines are read again — so a machine that answered shows what it
  // has, and one that did not is marked quiet by its road and its rows
  // dim with 无法获取 and the time they were last heard (`SessionsList`).
  // **Bounded**: a machine that is away costs a dial's timeout, and the
  // spinner does not wait for it — the round goes on, the next poll
  // reads what it brought.
  const refresh = useCallback(async () => {
    await Promise.race([syncNow().catch(() => undefined), new Promise((r) => setTimeout(r, SYNC_WAIT_MS))]);
    const [s, d] = await Promise.all([fetchSessions(arrangeBy), fetchDevices()]);
    setRows(s);
    setDevices(d);
    setRowsRead(true);
    setDevicesRead(true);
  }, [arrangeBy]);

  const openRecognition = (machine: string) => {
    if (trafficLights && !settingsFace) {
      openSettings("terminal", machine).catch(() => {});
      return;
    }
    setRecognitionMachine(machine);
    setSwitchingTabs(false);
    listPane.expand();
    setMorePage("terminal");
    setScreen("detail");
  };

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
  /** A session's machine, by name (the browser's machine chips, UX-42). */
  const homeOf = (id: string) => rows.find((r) => r.id === id)?.home ?? null;
  // **A session the wizard just opened, not listed yet** (UX-8, the first
  // run): its row arrives with the next sync, and a machine that went away
  // in between left the pane showing the network's overview under a tab
  // named by a bare id — a state with no word. Only sessions opened here
  // in this run: a tab restored from an earlier one is not waited for.
  const justOpened = useRef(new Map<string, number>());
  const waitingFor = selected !== null && !selectedRow ? (justOpened.current.get(selected) ?? null) : null;

  // 会话落地页上的路径也可点 (2026-09-18 ruling 54). Ruling 35 did this
  // inside a workspace only, because that is where the two things it
  // needs were: a root for a relative path to be relative to, and a
  // middle column to open a file in. **A session row carries the
  // first** — its live cwd, with birthplace as fallback — and the second is
  // one landing away, which is the whole of this ruling.
  //
  // **The machine is the session's own**, so an answer about a file is
  // an answer from the disk the conversation is happening on.
  const sessionMachine = selectedRow?.home ?? "";
  const sessionRoot = sessionCwdOnScreen(selectedRow) ?? "";
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
    // **No directory, no links** — a conversation nobody recorded a
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
    // ＋'s 新会话 is a new tab (UX-45, the owner 2026-10-07): in front at
    // once, its page the choices — the strip's ＋, and ⌘N / ⌘T with it.
    // ⚠️ 墓碑: the 新建会话 dialog from here.
    { key: "new", label: gui.new_session_ask, onSelect: () => openNewTab() },
    // What khor found and has not put on the list (docs/SHELL.md 第八轮).
    { key: "import", label: gui.import_sessions_ask, onSelect: () => setDialog("import") },
  ];
  const deviceActions: PaneAction[] = [
    // 添加机器 (round join-d): the one entry, both kinds of machine.
    { key: "add", label: gui.add_machine_ask, onSelect: () => setDialog("add") },
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
  // **Each seat keeps its place** (the audit fix plan F7.1; N11): the
  // narrow bar's seats each remember the screen they were on and how far
  // down it was, the wide landings how far down their list and board
  // were (the chosen row is each landing's own state already). Pressing
  // the seat that is showing goes back to its root, and at the root, to
  // the top — iOS's tab bar.
  // …keyed by the row that is open too (V06): two machines' pages are
  // two places, and one's scroll is not the other's.
  const openRow =
    landing === "devices" ? (selectedDevice ?? "") : landing === "sessions" ? (selected ?? "") : landing === "workspaces" ? (openWs?.path ?? "") : "";
  const placeKey = narrow
    ? `${morePage ? `more:${morePage}` : landing}:${switchingTabs ? "tabs" : screen}:${screen === "detail" ? openRow : ""}`
    : `${morePage ?? landing}:${openRow}`;
  const forgetScroll = useKeptScroll(placeKey);
  const seatScreen = useRef<Partial<Record<Landing, "list" | "detail">>>({});
  useEffect(() => {
    if (morePage === null && !switchingTabs) seatScreen.current[landing] = screen;
  }, [landing, screen, morePage, switchingTabs]);
  const goLanding = (key: Landing) => {
    // The seat that is showing, pressed again: its root, then its top.
    if (narrow && morePage === null && key === landing) {
      setSwitchingTabs(false);
      if (screen === "detail") {
        forgetScroll(`${key}:detail:${openRow}`);
        setScreen("list");
      } else {
        forgetScroll(`${key}:list:`);
        document.querySelectorAll<HTMLElement>("[data-pane-body], [data-nav-clear]").forEach((e) => e.scrollTo({ top: 0, behavior: "smooth" }));
      }
      return;
    }
    // **A transition** (V08): the new landing is rendered in slices, so
    // the chosen piece's slide, started on the press (`lib/travel`), is
    // drawn while the page is being built instead of after it.
    startTransition(() => {
      // A 「…」 page is open over this landing, so choosing a landing
      // closes it — the one press that always answers 「带我回去」.
      setMorePage(null);
      setLanding(key);
      // Narrow: the screen this seat was left on; wide has one.
      setScreen(narrow ? (seatScreen.current[key] ?? "list") : "list");
      setSwitchingTabs(false);
      // Choosing a landing brings a list put away back, at the width it had.
      listPane.expand();
    });
  };
  const me = devices.find((d) => d.me) ?? null;
  /** The wide face's list put away by pressing the open landing again. */
  const folded = !narrow && listPane.collapsed;
  /** **我的 khor is one page, the dial** (UX-26): on the wide face the
      list column folds away for the page's lifetime (nothing beside it)
      and ‹ at its top-left closes it back to the landing it was opened
      over. The column's own fold state is untouched — it comes back as
      it was. The settings window never shows it (UX-16). */
  const dial = !narrow && morePage === "me" && !settingsFace;
  /** The wide face's top row (方案 B): the landings' glyphs beside the
      window's buttons, then 更多. The open landing pressed again folds its
      list, and once more brings it back; another landing switches and
      brings it back (the owner: no toggle button). */
  /** **更多's four, on both bars** (UX-16; UX-19 gave the wide strip the
      phone's menu): 我的 khor, the landings past the fourth, 设置, 诊断. */
  const moreSettings = () => {
    // A Mac opens the settings window, from either bar.
    if (trafficLights && !settingsFace) {
      openSettings().catch(() => {});
      return;
    }
    setMorePage("appearance");
    setSwitchingTabs(false);
    // Narrow: 设置's list of seven, one screen; wide: its first page
    // beside the list in the list column.
    setScreen(narrow ? "list" : "detail");
    listPane.expand();
  };
  const moreDoctor = () => {
    setMorePage("doctor");
    setScreen("detail");
    setSwitchingTabs(false);
    listPane.expand();
  };
  const moreStore = () => {
    setMorePage("store");
    setScreen("detail");
    setSwitchingTabs(false);
    listPane.expand();
  };
  const moreMe = () => {
    setMorePage("me");
    setScreen("detail");
    setSwitchingTabs(false);
    listPane.expand();
  };
  const moreMenu = (
    <MoreMenuItems
      more={navOrder.slice(NAV_SHOWN).map(seatOf)}
      // The open landing chosen again folds its list, the strip's one
      // rule, from 更多's menu too (wide; narrow `goLanding` pops it).
      onGo={(key) => {
        if (!narrow && morePage === null && landing === key) listPane.toggle();
        else goLanding(key as Landing);
      }}
      onSettings={moreSettings}
      onDoctor={moreDoctor}
      onStore={moreStore}
      me={me ? { face: me.face } : null}
      onMe={moreMe}
    />
  );
  const strip = (
    <LandingStrip
      seats={navOrder.slice(0, NAV_SHOWN).map(seatOf)}
      active={morePage === null ? landing : null}
      // Tinted on every page reached through it, 我的 khor too (F3.6; W23),
      // and while the open landing is one of the folded ones.
      moreOn={morePage !== null || navOrder.slice(NAV_SHOWN).includes(landing)}
      onGo={(key) => {
        if (morePage === null && landing === key) listPane.toggle();
        else goLanding(key as Landing);
      }}
      // ⚠️ 墓碑 (UX-19): `onMore`, a circle of its own that opened 我的 khor
      // beside 更多's list, and none on a Mac (UX-10).
      moreMenu={moreMenu}
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
        // A Mac window dragged narrow opens the settings window too.
        if (trafficLights && !settingsFace) {
          openSettings().catch(() => {});
          return;
        }
        setMorePage("appearance");
        setSwitchingTabs(false);
        // Narrow: 设置's list of seven, one screen; wide: its first page
        // beside the list in the list column.
        setScreen(narrow ? "list" : "detail");
      }}
      onDoctor={() => {
        setMorePage("doctor");
        setScreen("detail");
        setSwitchingTabs(false);
      }}
      onStore={() => {
        setMorePage("store");
        setScreen("detail");
        setSwitchingTabs(false);
      }}
      me={me ? { face: me.face } : null}
      onMe={() => {
        setMorePage("me");
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
  // A sideways swipe on a terminal (roadmap 4.4): the next or previous
  // session tab, the same step ⌃⇥ takes.
  const stepTab = useRef<(by: number) => void>(() => {});
  stepTab.current = (by) => {
    if (openTabs.length < 2) return;
    const at = openTabs.indexOf(selected ?? "");
    // Stops at the ends, as the deck pill's own swipe does (Safari's tab
    // bar): a swipe past the last tab is not a jump back to the first.
    const next = at < 0 ? undefined : openTabs[at + by];
    if (next === undefined) return;
    sessionTabs.focus(next);
    setSwitchingTabs(false);
    setScreen("detail");
  };
  useEffect(() => {
    const asked = (e: Event) => stepTab.current((e as CustomEvent<{ by: number }>).detail.by);
    window.addEventListener(STEP_TAB_EVENT, asked);
    return () => window.removeEventListener(STEP_TAB_EVENT, asked);
  }, []);
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
      // ⌘, on a Mac: the settings window (UX-10; ux-9 M1).
      if (trafficLights && !settingsFace) {
        openSettings().catch(() => {});
        return;
      }
      setMorePage("appearance");
      setScreen("detail");
      setSwitchingTabs(false);
    },
    me: () => {
      setMorePage("me");
      setScreen("detail");
      setSwitchingTabs(false);
    },
    doctor: () => {
      setMorePage("doctor");
      setScreen("detail");
      setSwitchingTabs(false);
    },
    closePage: morePage !== null ? () => setMorePage(null) : null,
    lock: lock.facts?.on ? lock.now : null,
    find: () => window.dispatchEvent(new Event(FIND_ALL)),
    import: () => setDialog("import"),
    // ⌃⌘S: the same fold the open landing's glyph does (F6.7).
    toggleList: narrow ? null : () => listPane.toggle(),
    listFolded: listPane.collapsed,
    order: navOrder,
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
  /** Where the open session puts its verbs on the wide face (`VerbSeat`). */
  const [verbSeat, setVerbSeat] = useState<HTMLElement | null>(null);
  const days = useDayStrips(useStripWidth(detailBox, sessions));
  /** One session's day as the pane holds it (`DayStrip` `Day`): `null`
      until the first answer, then its runs. */
  const dayOf = (id: string): Day => (days === null ? null : (days[id] ?? []));
  /** **The front tab's press opens the session's 附栏** (UX-30: the
      phone's pill gesture, docked). Per session (`use-session-side`):
      open, width, seat, and where it was left (UX-47). */
  const side = useSessionSide(selected);
  /** The 附栏 put back where it was left (UX-47); through a ref, because
      the tab faces are memoised and the place reads state declared
      further down. */
  const resumeSideRef = useRef<(session: string) => boolean>(() => false);
  // The open sessions as the strip paints them: each tab wears its
  // row's title and the row's own identity assembly (`SessionFace` —
  // face, state halo, kind mark), the same facts in the same clothes. A
  // tab whose row has not been listed yet (just opened) wears its id.
  const actsVersion = useTabActs();
  const noticesVersion = useNotices();
  // **A bell rung off screen** (the terminal audit, item 67,
  // `lib/bell-marks`): the sessions whose bell rang while they were not
  // in front wear the mark on their tab and row until they are.
  const bellSeen = useRef(new Map<string, number>());
  const rung = useMemo(
    () => rungOffScreen(rows, bellSeen.current, onScreen.current),
    // `onScreen` reads these.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [rows, selected, landing, screen, narrow, hidden],
  );
  /** **＋ makes a new tab** (UX-45): in front at once, its page the
      new-session choices (`NewSessionPage`); made, it becomes
      that session (`sessionTabs.become`), closed first, it is gone. */
  const openNewTab = (at: NewTabAt = {}) => {
    const id = newTabId();
    newTabs.current.set(id, {
      at,
      from: { landing, screen, morePage, front: isNewTab(selected) ? null : selected },
    });
    setMorePage(null);
    setLanding("sessions");
    sessionTabs.open(id);
    setScreen("detail");
  };
  openNewTabRef.current = openNewTab;
  /** Each new tab's road: what it filled in, and **where the person was
      when they asked** — the landing, its screen, a 「…」 page, the tab
      in front — which 取消 (and a workspace's 创建) goes back to. */
  const newTabs = useRef(
    new Map<string, { at: NewTabAt; from: { landing: Landing; screen: typeof screen; morePage: typeof morePage; front: string | null } }>(),
  );
  /** **Back to where they asked from** (UX-45, the owner 2026-10-07): the
      machine page, the terminal, the workspace, the list, or the tab
      that was in front — one rule on the phone and wide. */
  const leaveNewTab = (id: string) => {
    const road = newTabs.current.get(id);
    newTabs.current.delete(id);
    sessionTabs.close(id);
    if (!road) return;
    const { from } = road;
    if (road.at.inPlace) setWsBack(true);
    if (from.front && openTabs.includes(from.front)) sessionTabs.focus(from.front);
    setLanding(from.landing);
    setMorePage(from.morePage);
    setScreen(from.front || from.landing !== "sessions" ? from.screen : "list");
  };
  // Group presentation and pane visibility are separate; entering the group reveals its content.
  const [sessionTabsFocused, setSessionTabsFocused] = useState(false);
  const tabFaces: TabFace[] = useMemo(
    () =>
      openTabs.map((id) => {
        if (isNewTab(id))
          return {
            id,
            label: gui.new_tab,
            face: null,
            mark: newTabKind(),
            path: "",
            menu: [{ key: "close", label: gui.close_tab, run: () => sessionTabs.close(id) }],
          };
        const r = rows.find((row) => row.id === id);
        // **The broadcast the list does not show** (UX-45): put away with
        // nothing newer (docs/SESSION.md 动作) it is still there, and 留言
        // opens it; its first message brings the row back.
        if (!r && isChatTab(id)) {
          return {
            id,
            label: msg.broadcast_title,
            face: null,
            path: "",
            session: { word: "idle", kind: "broadcast", category: null },
            menu: [{ key: "close", label: gui.close_tab, run: () => sessionTabs.close(id) }],
          };
        }
        // **Pressed again, the tab in front opens or closes its 附栏**
        // (UX-30; ⚠️ 墓碑 2026-09-29 → 10-06: it showed or hid the day —
        // 「点击标签后显示吧」 — which is 最近's first row now, UX-39 §4). Read by
        // the strip only on the tab in front, so the front tab's memory
        // is the one consulted.
        const again = {
          hint: side.open ? gui.side_hide : gui.side_show,
          expanded: side.open,
          controls: side.open ? SIDE_ID : undefined,
          toggle: () => {
            if(side.open){content.setMaximized(null);return setSessionSide(id,{open:false});}
            contentNavigation.resume(id);
            setSessionSide(id, { open: true, seat: "workspace" });
          },
        };
        // ⚠️ 墓碑 (UX-36 → UX-39 §4, the owner 2026-10-06 22:10: 「药丸下面就
        // 不绘制 session 的日志了，日志在最近里面，放在最上面」): the day's
        // line in the pill's slot, the day in full hanging from it, and the
        // menu's 「今天」. The day is 最近's first row now, on both faces.
        // **Files going in** (the owner 2026-10-07 12:50): the pill wears
        // the file's kind for a face and says 「正在传入」, and its third
        // seat cancels them; back to the session once they are in.
        const dropping = tabAct(id)?.drop;
        return r
          ? {
              id,
              // An unnamed shell's tab follows what runs in it (roadmap
              // 2.5, `termName`); a name somebody gave stays.
              label: dropping ? stemOf(dropping.name) : termName(r),
              // The long form: the hover title, the tabs sheet's row.
              title: dropping
                ? dropping.total > 1
                  ? msg.drop_into_of(dropping.name, dropping.done, dropping.total)
                  : msg.drop_into(dropping.name)
                : undefined,
              face: r.face,
              path: sessionCwdOnScreen(r) ?? "",
              // **One line, the title** (UX-20, the owner 2026-10-05): where
              // the shell stands is the row's left-bottom (UX-15) and the
              // pill's 「目录」 (UX-17), never a tab. ⚠️ 墓碑: `sub`, the
              // directory under the name (roadmap 2.5).
              // What the program says of its progress (roadmap 4.1).
              progress: r.doing?.running && r.doing.progress !== null ? <ProgressLine doing={r.doing} /> : undefined,
              // Where the shell stands, and its two acts (roadmap 2.5) —
              // and 关闭 (UX-33: the pill has no ×; the menu, ⌘W and the
              // tabs sheet close).
              menu: [
                ...(sessionCwdOnScreen(r) ? hereActs(r.home, sessionCwdOnScreen(r)!) : []),
                { key: "close", label: gui.close_tab, run: () => sessionTabs.close(id) },
              ],
              session: { word: r.word, kind: r.kind, category: r.category },
              // A session tab's name is the row's title, which is a
              // name somebody gave it — so double-click renames it
              // here too. A browsing tab below wears a page's own
              // name and hands none of this in.
              rename: { kind: "session" as const, id },
              again,
              // **The third seat, re-attach** (UX-33): the pane's own act,
              // offered by id while it is mounted (`lib/tab-acts`); a tab
              // whose pane has not offered one yet is brought to the front,
              // where it mounts and offers it.
              act: dropping
                ? { kind: "stop" as const, label: gui.term_drop_stop, onPress: () => tabAct(id)?.run() }
                : {
                    kind: "reload" as const,
                    label: gui.term_reload,
                    onPress: () => {
                      sessionTabs.focus(id);
                      tabAct(id)?.run();
                    },
                  },
              loading: tabAct(id)?.busy ?? false,
              share: tabAct(id)?.share,
              rang: rung.has(id),
              // **What stops this session** (NOTICE-BRIEF): its pane
              // offers them by id, as it offers its act.
              interaction: {
                drop: dropping ? { name: stemOf(dropping.name) } : undefined,
                notices: noticesOf(id),
              },
            }
          : { id, label: id, face: null, path: "", again };
      }),
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [openTabs, rows, devices, side, actsVersion, noticesVersion, days, selected, rung, narrow, content.tabs],
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
    setWsTree(true);
    setScreen("detail");
  };
  /** **Where the column's 工作区 seat stands** (UX-30 §2, §4): the
      session's live directory, else its birthplace, when the column opens
      on it; a directory or a file a session printed, opened in place. */
  const [sideFiles, setSideFiles] = useState<{
    session: string;
    machine: string;
    dir: string | null;
    start: { path: string; at: number } | null;
    follow?: boolean;
  } | null>(null);
  /** **Where the 工作区 tree stands inside** (UX-47): said by the mounted
      tree as it moves (`SideWorkspace`'s `onSpot`), kept with the place. */
  const [sideSpot, setSideSpot] = useState<{ session: string; spot: WsSpot } | null>(null);
  /** The spot an opening came back to, handed to the tree's next mount
      only; any seat entered or road taken since starts it afresh. */
  const [sideResume, setSideResume] = useState<{ session: string; spot: WsSpot | null } | null>(null);
  const sideFilesRef = useRef(sideFiles);
  sideFilesRef.current = sideFiles;
  // A terminal's 「open enclosing folder」 (roadmap 2.5): that machine's
  // files at that path.
  const openDirRef = useRef<(machine: string, path: string) => void>(() => {});
  /** The main area's road for a directory. */
  const openDirMain = (machine: string, path: string) => {
    const d = devices.find((x) => x.name === machine);
    if (!d) return;
    setLanding("workspaces");
    openWorkspace({ device: d.id, machine, path } as WorkspaceRow);
  };
  /** **Whether a thing a session printed opens in its 附栏** (UX-30 §4):
      with a session in front, paths, URLs and directories open inside
      the 附栏 — the main area is the session, always. On the wide face
      that is the column; **on the phone the half sheet** (UX-37 addendum
      A1, the owner 18:40), over the session's own screen, the terminal
      staying where it is. Elsewhere (another landing) the old roads
      stand. ⚠️ 墓碑 (UX-30 → A1): on the phone a tapped file opened in
      the main area, and its ‹ climbed the directory tree there. */
  const inColumn = () => landing === "sessions" && selectedRow !== null && (!narrow || inSession);
  /** The 附栏 turned to a seat: the column opened on it, or on the phone
      the sheet risen on it (a sheet already up keeps its detent). */
  /** **Opened from 最近** (UX-39, the owner 2026-10-06 22:25: 「Recent 里
      点开了一个文件或什么，返回应该是返回到 Recent」): the page a row of
      最近 opens wears ‹ back to 最近; a seat entered by the menu does not. */
  const [sideFrom, setSideFrom] = useState<"recent" | null>(null);
  /** Whose 附栏 the shell's pieces of it (`sideFrom`, `sideBrowse`,
      `sideQuota`, the resume spots) describe: set with them as a place is
      put back or a road opens it, so a place is never kept under the name
      of a session whose pieces have not arrived yet (fix-e). */
  const [piecesOf, setPiecesOf] = useState<string | null>(null);
  const backToRecent = () => {
    setSideFrom(null);
    setSideBrowse({ page: false });
    sideTo("recent");
  };
  const sideTo = (seat: PlaceSeat) => {
    if (!selectedRow) return;
    // A road from outside (a path or a URL the terminal printed) opening
    // the 附栏: what was opened from 最近 before it shut does not carry
    // over into the place it now keeps (UX-47).
    if (sideOpenFor.current !== selectedRow.id) setSideFrom(null);
    setPiecesOf(selectedRow.id);
    if (!narrow) return setSessionSide(selectedRow.id, { open: true, seat });
    setSessionSide(selectedRow.id, { seat });
    setPlaceSheet((was) => was ?? { dir: null, session: selectedRow.id });
  };
  /** The 附栏's 工作区 seat, pointed somewhere (and the 附栏 opened on
      it): a directory, or a file to open in place — the file page inside
      the 附栏, whose ‹ is the seat's own (the listing, then up). */
  const openContent = (view: ContentView, machine: string, path: string, line?: number | null) => {
    if (!selectedRow) return;
    const session = selectedRow.id;
    const device = devices.find(d => machine ? d.name === machine || d.id === machine : d.me);
    if (!device) return content.fail(gui.content_machine_gone);
    (document.activeElement as HTMLElement|null)?.blur();
    if(view==='workspace')contentNavigation.update(session,{scope:path?{device:device.id,root:path}:null,source:'workspace',scene:'picker'});
    else contentNavigation.document(content.open(session,device.id,path,view,line));
    setSessionSide(session,{open:true,seat:'workspace'});
  };
  const sideShow = (machine: string, dir: string | null, start: { path: string; at: number } | null) =>
    openContent(start ? "file" : "workspace", machine, start?.path ?? dir ?? "");
  /** **The session a thing was opened from** (UX-37 §1): the one in
      front of the sessions landing — its terminal printed the path or
      the URL, or its 附栏 opened it. Written into the record, which puts
      the thing in that session's slice. */
  const fromSession = () => (landing === "sessions" && selectedRow ? selectedRow.id : undefined);
  /** A place opened from a session, remembered as its (`recentVisit`). */
  const touched = (kind: "dir" | "file", machine: string, path: string) => {
    const session = fromSession();
    if (session && machine && path.startsWith("/")) recentVisit(kind, machine, path, session).catch(() => {});
  };
  openDirRef.current = (machine, path) => {
    const name = machine || (devices.find((x) => x.me)?.name ?? "");
    touched("dir", name, path);
    if (inColumn()) return sideShow(name, path, null);
    openDirMain(machine, path);
  };
  useEffect(() => {
    const asked = (e: Event) => {
      const { machine, path } = (e as CustomEvent<{ machine: string; path: string }>).detail;
      openDirRef.current(machine, path);
    };
    window.addEventListener(OPEN_DIR_EVENT, asked);
    return () => window.removeEventListener(OPEN_DIR_EVENT, asked);
  }, []);
  // **A path or a URL a terminal printed** (roadmap 3.2): a directory is
  // that machine's files there; a file opens in the workspace that holds
  // it, or in its own directory's, at its line (`reveal`, the
  // conversation's road); a URL is a browsing tab out through that
  // terminal's machine.
  const openPathRef = useRef<(machine: string, path: string, dir: boolean, line: number | null) => void>(() => {});
  /** The main area's road for a file: the workspace that holds it, at
      its line. */
  const openPathMain = (machine: string, path: string, dir: boolean, line: number | null) => {
    const d = machine === "" ? devices.find((x) => x.me) : devices.find((x) => x.name === machine);
    if (!d) return;
    const name = machine || d.name;
    if (dir) return openDirMain(name, path);
    void fetchWorkspaces()
      .catch(() => [] as WorkspaceRow[])
      .then((list) => {
        const ws = holding(list, d.id, path);
        setMorePage(null);
        setLanding("workspaces");
        setOpenWs(
          ws
            ? { device: ws.device, machine: ws.machine || ws.device, path: ws.path }
            : { device: d.id, machine: name, path: path.slice(0, path.lastIndexOf("/")) || "/" },
        );
        setReveal({ path, line, at: Date.now() });
        setScreen("detail");
      });
  };
  openPathRef.current = (machine, path, dir, line) => {
    touched(dir ? "dir" : "file", machine || (devices.find((x) => x.me)?.name ?? ""), path);
    if (inColumn()) {
      const name = machine || (devices.find((x) => x.me)?.name ?? "");
      return openContent(dir ? "workspace" : "file", name, path, line);
    }
    openPathMain(machine, path, dir, line);
  };
  const openUrlRef = useRef<(machine: string, url: string) => void>(() => {});
  /** A page, as a browsing tab out through that machine; in the column's
      browser with a session in front, else on the browser landing.
      **By the record's device id where there is one, this machine
      included** (sb-a, 2026-10-09, the owner: 「侧栏里最近打开的浏览器地址,
      点击直接打开, 现在点击后还等着输地址选机器」): the lookup was by name
      and skipped this machine — a leftover from when an empty device meant
      this one — so a page here opened with no machine and the pill asked
      for one and an address. */
  const openUrlMain = (machine: string, url: string, column: boolean, device?: string) => {
    if (column && selectedRow) return openContent("page", device || machine || selectedRow.home, url);
    const d = devices.find((x) => device !== undefined && x.id === device) ?? (machine === "" ? null : devices.find((x) => x.name === machine));
    void openTab("page", d?.id ?? device ?? "", url, fromSession())
      .then((id) => {
        if (column && selectedRow) {
          // In front at once, and in the list: the 附栏's page is the tab
          // just opened, not the one in front before (the landing's beat
          // would have caught up a beat late, the 附栏 had none).
          pageTabs.focus(id);
          setSideBrowse({ page: true });
          sideTo("browser");
          return;
        }
        setMorePage(null);
        setLanding("browser");
        setScreen("detail");
      })
      .catch(() => {});
  };
  openUrlRef.current = (machine, url) => openUrlMain(machine, url, inColumn());
  useEffect(() => {
    const path = (e: Event) => {
      const { machine, path: p, dir, line } = (e as CustomEvent<{ machine: string; path: string; dir: boolean; line: number | null }>).detail;
      openPathRef.current(machine, p, dir, line);
    };
    const url = (e: Event) => {
      const { machine, url: u } = (e as CustomEvent<{ machine: string; url: string }>).detail;
      openUrlRef.current(machine, u);
    };
    window.addEventListener(OPEN_PATH_EVENT, path);
    window.addEventListener(OPEN_URL_EVENT, url);
    return () => {
      window.removeEventListener(OPEN_PATH_EVENT, path);
      window.removeEventListener(OPEN_URL_EVENT, url);
    };
  }, []);

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
        // ⌘T: a new session is a new tab here (`lib/shortcuts`).
        newIsTab={sessions}
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
            rung={rung}
          />
        ) : landing === "workspaces" ? (
          <WorkspacesList
            devices={devices}
            query={queries.workspaces}
            words={words.workspaces}
            reloadAt={wsAdded}
            selected={openWs}
            // A row opens its workspace's tree in this column (UX-23):
            // there is no tree beside the list to fold any more.
            treeOpen={false}
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
        {devicesRead && devices.length === 1 && (sessions || landing === "devices") && !queries[landing] && (
          <AloneNote onAdd={() => setDialog("add")} />
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

  /** 「在文件视图中打开」 from a conversation's file (round CHAT): the
      workspace that holds it, asked of the local table at the press the
      way a path in prose is (ruling 54); else that machine's files, in
      place on its page, opened at the file. */
  const showFile: ShowFile = (machine, abs) => {
    const device = devices.find((d) => d.name === machine) ?? null;
    const onPage = () => {
      if (device === null) return;
      setMorePage(null);
      setLanding("devices");
      if (device.id === selectedDevice) setMachineBody("files");
      else bodyNext.current = "files";
      setSelectedDevice(device.id);
      setFileStart({ path: abs, at: Date.now() });
      setScreen("detail");
    };
    void fetchWorkspaces().then((list) => {
      const ws = device === null ? null : holding(list, device.id, abs);
      if (ws === null) return onPage();
      setMorePage(null);
      setLanding("workspaces");
      setOpenWs({ device: ws.device, machine: ws.machine || ws.device, path: ws.path });
      setReveal({ path: abs, line: null, at: Date.now() });
      setScreen("detail");
    }, onPage);
  };

  /** **Where the session is**, for the column's 工作区 seat (UX-30 §2:
      the current pane/shell cwd, else where the session started) — read off the
      terminal on screen, as the pill's tap reads it (`placeSheet`), when
      the column opens on it or the session changes. */
  useEffect(() => {
    if (narrow || !side.open || !selectedRow) return;
    // The default slice follows cd/pane changes. Explicitly opened files
    // and directories (`sideShow`) remain pinned until the seat is pressed.
    const prior = sideFilesRef.current;
    if (prior?.session === selectedRow.id && (!prior.follow || prior.dir === sessionRoot)) return;
    setSideFiles({ session: selectedRow.id, machine: selectedRow.home, dir: sessionCwdOnScreen(selectedRow), start: null, follow: true });
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [narrow, side.open, selectedRow?.id, sessionRoot]);
  // **One page per seat, on this session's slice, and nothing goes back
  // to a whole** (UX-39 §3, the owner 2026-10-06 21:05: 「都不可返回，全
  // 都和 额度 一样」): 最近 and 浏览器 on what this session opened, 工作区
  // on the tree of the workspace holding the shell's directory, 机器 on
  // its machine, 额度 on its account. The only ‹ inside a seat is a page's
  // back to the list it was opened from. ⚠️ 墓碑 (UX-37 §1 → UX-39 §3):
  // ‹ from each slice to the whole — 全部最近, 全部网页, the workspaces
  // list, the machines list, every account. ⚠️ 墓碑 (UX-30 §1): each
  // seat kept its own stack across visits.
  const [sideBrowse, setSideBrowse] = useState<{ page: boolean }>({ page: false });
  const [sideQuota, setSideQuota] = useState<"list" | "detail">("list");
  /** **Where 存储 stands in the 附栏** (UX-47): said by the page as it
      moves, kept with the place; and the spot an opening came back to,
      for the page's next mount only. */
  const [sideStore, setSideStore] = useState<{ session: string; spot: StoreSpot } | null>(null);
  const [sideStoreResume, setSideStoreResume] = useState<{ session: string; spot: StoreSpot | null } | null>(null);
  /** Where the 工作区 seat points: the directory a seat press, a tapped
      path or a row of 最近 asked for, else the session's own. */
  const sideWs = selectedRow
    ? sideFiles?.session === selectedRow.id
      ? { ...sideFiles, dir: workspaceCwd(selectedRow, sideFiles, sessionRoot) }
      : { session: selectedRow.id, machine: selectedRow.home, dir: sessionCwdOnScreen(selectedRow), start: null, follow: true }
    : null;
  /** **The workspace that holds that directory: the tree's ceiling**
      (UX-39 §5) — on a seat press the shell's directory, so the session's
      workspace; outside every workspace, the directory itself. Read from
      the local table once per directory; `root` is `null` when no
      workspace holds it. */
  const sideWsKey = sideWs ? `${sideWs.machine}\n${sideWs.dir ?? ""}` : null;
  const [sideRoot, setSideRoot] = useState<{ key: string; root: string | null } | null>(null);
  useEffect(() => {
    if (sideWsKey === null || !sideWs?.dir) return;
    const dir = sideWs.dir;
    const dev = devices.find((d) => d.name === sideWs.machine)?.id ?? "";
    let live = true;
    void fetchWorkspaces().then(
      (list) => live && setSideRoot({ key: sideWsKey, root: holding(list, dev, `${dir.replace(/\/$/, "")}/`)?.path ?? null }),
      () => live && setSideRoot({ key: sideWsKey, root: null }),
    );
    return () => {
      live = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sideWsKey, devices.length]);
  /** Bumped as a seat is entered: the seat's page mounts afresh, so a
      page that lands by itself (额度's account) lands again. */
  const [sideEntry, setSideEntry] = useState(0);
  /** Every seat back on its slice — the pill's tap, a seat pressed —
      and nothing opened from 最近 any more. */
  const toSlices = () => {
    setSideFrom(null);
    setSideBrowse({ page: false });
    setSideQuota("list");
    setSideResume(null);
    setSideStoreResume(null);
    setSideEntry((n) => n + 1);
  };
  /** A seat pressed: on its slice, the 工作区 at the shell's directory
      now (the pill's own rule). */
  const enterSeat = (seat: PlaceSeat) => {
    if (!selectedRow) return;
    toSlices();
    if (seat === "workspace") {
      setSideFiles({ session: selectedRow.id, machine: selectedRow.home, dir: sessionCwdOnScreen(selectedRow), start: null, follow: true });
    }
    setSessionSide(selectedRow.id, { seat });
  };
  /** **The terminal pill opens the session's 附栏 as a sheet** (UX-17 →
      UX-30 §6): the same seats, risen from the bottom. The
      sheet's lifetime is this state; the seat is the session's memory. */
  const [placeSheet, setPlaceSheet] = useState<{ dir: string | null; session: string } | null>(null);
  /** **Each pill remembers its 附栏** (fix-e, the owner 2026-10-08: 「要记
      住每一个药丸的侧边展开情况, 从一个没有展开的到展开的, 要自动展开」):
      a tab in front whose 附栏 was left open opens it again, where it was
      left (UX-47), and one left closed shows none; another landing draws
      none and forgets nothing, so coming back to the session is coming
      back to its pill. The window's room follows (`sideWanted` below):
      closed to open widens, open to closed gives back, open to open keeps
      the room it has. The phone's sheet is each opening's glance, and
      goes. ⚠️ 墓碑 (UX-46 §1 → fix-e): leaving the session — another
      landing, another tab — closed it, and coming back found it closed. */
  // A layout effect: the place is back before the column is painted, so
  // a tab switched to never shows the last one's page for a frame.
  useLayoutEffect(() => {
    setPlaceSheet(null);
    if (!narrow && sessions && selected !== null && side.open) resumeSide(selected);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [selected, sessions]);
  // A tab closed forgets its 附栏's open (`keepSidesOf`).
  useEffect(() => keepSidesOf(openTabs), [openTabs]);
  /**
   * **Opened again, the 附栏 is where it was left** (UX-47, the owner
   * 2026-10-07, replacing UX-40 §3's 「每次打开都是显示最近」): per
   * session, the seat and what was open in it — a file page, a web page,
   * the 工作区 tree, the browser's list, 机器, 额度's list or account,
   * 存储's machine and the file open there. Kept in memory while the 附栏 is open (`keepSidePlace`,
   * below), never written down: after a restart it opens on 最近.
   *
   * **What is gone lands on 最近**: never opened; the file deleted (khor's
   * own acts say so at once, `onTreeChanged` below; anything else is
   * asked as it opens); the page closed; the session's machine gone from
   * the table. The detent of the phone's sheet is not part of it: each
   * opening is the pill's glance, at half (UX-37 §3).
   *
   * Answers whether there was a place to go back to.
   */
  const resumeSide = (session: string): boolean => {
    if (!narrow) return content.tabs.some(t => t.session === session);
    const p = sidePlace(session);
    const known = (machine: string) => devices.some((d) => d.name === machine);
    const standing =
      p !== null &&
      selectedRow?.id === session &&
      (p.seat !== "machine" || known(selectedRow.home)) &&
      (p.seat !== "workspace" || (p.files !== null && known(p.files.machine))) &&
      (p.seat !== "browser" || p.page === null || pageTabs.tabs.some((t) => t.id === p.page && t.session === session)) &&
      (p.seat !== "store" || p.store === null || known(p.store.machine));
    setPiecesOf(session);
    if (!standing || p === null) {
      forgetSidePlace(session);
      toSlices();
      setSessionSide(session, { seat: "recent" });
      return false;
    }
    setSideFrom(p.fromRecent ? "recent" : null);
    setSideBrowse({ page: p.seat === "browser" && p.page !== null });
    setSideQuota(p.quota);
    const follow = p.files ? followsSessionCwd(selectedRow, p.files) : false;
    if (p.files) setSideFiles({ session, machine: p.files.machine, dir: follow ? sessionCwdOnScreen(selectedRow) : p.files.dir, start: p.files.start, follow });
    setSideResume(p.seat === "workspace" && p.files && !follow ? { session, spot: p.files.spot } : null);
    setSideStoreResume(p.seat === "store" ? { session, spot: p.store } : null);
    if (p.seat === "browser" && p.page !== null && p.page !== pageTabs.current.id) pageTabs.focus(p.page);
    setSessionSide(session, { seat: p.seat });
    // Asked as it opens: a file or a page gone by a road khor did not see
    // (another window, the shell, another machine). Only while the 附栏
    // still stands on that very thing.
    const still = () => sidePlace(session) !== null && sideOpenFor.current === session;
    const file = p.seat === "workspace" ? placeFile(p.files?.spot ?? null) : null;
    if (file !== null && p.files) {
      void fileThere(p.files.machine, file).then((there) => {
        if (there === false && still() && placeFile(sidePlace(session)?.files?.spot ?? null) === file) landOnRecent(session);
      });
    }
    const stored = p.seat === "store" ? (p.store?.file ?? null) : null;
    if (stored !== null && p.store) {
      void fileThere(p.store.machine, stored).then((there) => {
        if (there === false && still() && sidePlace(session)?.store?.file === stored) landOnRecent(session);
      });
    }
    if (p.seat === "browser" && p.page !== null) {
      const page = p.page;
      void fetchTabs(PAGES.kind).then(
        (open) => {
          if (!open.tabs.some((t) => t.id === page) && still() && sidePlace(session)?.page === page) landOnRecent(session);
        },
        () => {},
      );
    }
    return true;
  };
  resumeSideRef.current = resumeSide;
  /** The thing come back to has gone: 最近, and nothing to come back to. */
  const landOnRecent = (session: string) => {
    forgetSidePlace(session);
    toSlices();
    setSessionSide(session, { seat: "recent" });
  };
  /** The session whose 附栏 is up now — the column or the phone's sheet —
      for an answer that arrives after it may have gone. */
  const sideOpenFor = useRef<string | null>(null);
  sideOpenFor.current = narrow ? (placeSheet?.session ?? null) : sessions && side.open ? selected : null;
  /** Whether a file is still there: its directory, read. `null` when the
      machine did not answer — not gone, only not known. */
  const fileThere = (machine: string, path: string): Promise<boolean | null> =>
    fetchLs(machine, levelOf(path)).then(
      (l) => l.entries.some((e) => !e.dir && e.name === path.slice(path.lastIndexOf("/") + 1)),
      () => null,
    );
  // **A file khor itself deleted or moved** (`lib/tree-changed`): any
  // session that was left on it lands on 最近 when its 附栏 opens again.
  // The one open now is not touched — its own page answers the change.
  const fileThereRef = useRef(fileThere);
  fileThereRef.current = fileThere;
  useEffect(
    () =>
      onTreeChanged((c) => {
        for (const [session, p] of sidePlaces()) {
          const file = p.seat === "workspace" ? placeFile(p.files?.spot ?? null) : null;
          if (session === sideOpenFor.current || file === null || !p.files) continue;
          if (!sameMachine(c, { machine: p.files.machine, here: p.files.here }) || levelOf(file) !== c.dir) continue;
          void fileThereRef.current(p.files.machine, file).then((there) => {
            if (there === false && sidePlace(session) === p) forgetSidePlace(session);
          });
        }
      }),
    [],
  );
  /** **过去24小时, 最近's first row, on both faces, and it does not
      expand** (UX-39 §4, the owner 2026-10-06 20:30 and 22:10): the day's
      band and its two totals; hover or a press on a stretch says that
      stretch. ⚠️ 墓碑 (UX-37 §5 → UX-39 §4): 今天, the phone's only, a tap
      expanding the day in full. ⚠️ 墓碑 (UX-36 §1): a 「今天」 row under
      the sheet's capsule. ⚠️ 墓碑 (UX-30 §5): the ruler, under the
      column's capsule and the sheet's. */
  const dayTotals = useDayTotals(selectedRow?.id ?? null, (narrow ? placeSheet !== null : side.open) && side.seat === "recent");
  const sheetToday = (row: SessionRow) => <DayToday runs={dayOf(row.id)} totals={dayTotals} />;
  const sideOn = selectedRow !== null && sessions && (narrow ? placeSheet !== null : side.open);
  useEffect(() => setSideBrowsing(narrow && sideOn && side.seat === "browser"), [narrow, sideOn, side.seat]);
  // **Where the 附栏 stands, kept as it moves** (UX-47, `resumeSide`): only
  // while it is up beside the session in front — a render on the way to
  // another session still holds the last one's pieces, and must not
  // write them under the new one's name.
  const pageFront = pageTabs.current.id;
  useEffect(() => {
    if (!sideOn || !selectedRow || sideOpenFor.current !== selectedRow.id) return;
    // The pieces are another session's until its place has been put
    // back (`resumeSide`): a tab switched to an open 附栏 renders once
    // with the last one's 浏览器 page, 额度 screen and road from 最近.
    if (!narrow && piecesOf !== selectedRow.id) return;
    const files = sideFiles?.session === selectedRow.id ? sideFiles : null;
    keepSidePlace(selectedRow.id, {
      seat: side.seat,
      fromRecent: sideFrom === "recent",
      files: files && {
        machine: files.machine,
        follow: files.follow,
        here: devices.find((d) => d.name === files.machine)?.me ?? !selectedRow.source,
        dir: files.dir,
        start: files.start,
        spot: sideSpot?.session === selectedRow.id ? sideSpot.spot : null,
      },
      page: side.seat === "browser" && sideBrowse.page && pageFront !== "?" && pageFront !== "" ? pageFront : null,
      quota: sideQuota,
      store: sideStore?.session === selectedRow.id ? sideStore.spot : null,
    } satisfies SidePlace);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sideOn, selectedRow?.id, side.seat, sideFrom, sideFiles, sideSpot, sideBrowse.page, pageFront, sideQuota, sideStore, piecesOf]);
  const [recent, forgetRecentRow] = useSideRecent(narrow && sideOn && side.seat === "recent");
  /** 最近's rows open in place (§4): a page in the column's browser, a
      workspace as the column's listing, a file as its file page. */
  const openRecent = (it: RecentItem) => {
    setSideFrom("recent");
    if (it.kind === "page") return openUrlMain(it.machine, it.url, true, it.device);
    if (it.kind === "workspace") {
      touched("dir", it.machine, it.path);
      return sideShow(it.machine, it.path, null);
    }
    sideShow(it.machine, it.path.slice(0, it.path.lastIndexOf("/")) || "/", { path: it.path, at: Date.now() });
  };
  /** …and a double-click (or 「在主区打开」) takes the same thing to the
      main area, by the roads the landings use. */
  const openRecentMain = (it: RecentItem) => {
    if (it.kind === "page") return openUrlMain(it.machine, it.url, false, it.device);
    if (it.kind === "workspace") return openDirMain(it.machine, it.path);
    openPathMain(it.machine, it.path, false, null);
  };
  /** The column's seat, composed here because the narrow pages read the
      shell's own state (`SessionSide` is the box and the capsule). */
  const sideSeat = (): ReactNode => {
    if (!selectedRow) return null;
    const faceOf = (id: string) => devices.find((d) => d.id === id)?.face ?? null;
    switch (side.seat) {
      case "recent":
        return (
          <SideRecent
            items={recent}
            slice={selectedRow.id}
            today={sheetToday(selectedRow)}
            faceOf={faceOf}
            onOpen={openRecent}
            onOpenMain={openRecentMain}
            onForget={forgetRecentRow}
          />
        );
      case "workspace": {
        const f = sideWs!;
        const here = devices.find((d) => d.name === f.machine)?.me ?? !selectedRow.source;
        // **The workspace's own tree, and nothing above it** (UX-39 §5, the
        // owner 2026-10-06 22:00): the ceiling is the workspace holding the
        // directory, else that directory; nothing is drawn until the table
        // has said which, so the tree mounts once. ⚠️ 墓碑 (UX-37 §1):
        // `MachineFiles` climbing to the root and on to every workspace.
        if (f.dir && sideRoot?.key !== sideWsKey) return null;
        const root = (f.dir ? sideRoot?.root : null) ?? f.dir ?? "/";
        return (
          <div data-side-files className="flex min-h-0 flex-1 flex-col">
            <SideWorkspace
              key={`${sideEntry} ${f.machine} ${root}`}
              machine={f.machine}
              here={here}
              root={root}
              start={f.start}
              // Where it was left (UX-47), and where it is as it moves.
              spot={sideResume?.session === selectedRow.id ? sideResume.spot : null}
              onSpot={(spot) => {
                setSideSpot({ session: selectedRow.id, spot });
                if (spot.file) setSideFiles((f) => f?.session === selectedRow.id && f.follow ? { ...f, follow: false } : f);
              }}
              onLeave={sideFrom === "recent" ? backToRecent : undefined}
              onRead={(path) => touched("file", f.machine, path)}
              fileActs={(path) => [{ key: "open-main", label: gui.open_in_main, run: () => openPathMain(f.machine, path, false, null) }]}
            />
          </div>
        );
      }
      case "browser":
        return (
          <BrowseLanding
            key={`${sideEntry} ${browsing.kind.kind}`}
            kind={browsing.kind}
            devices={devices}
            tabs={browsing.tabs}
            narrow
            pane={null}
            caption={stale ? gui.backend_unreachable : undefined}
            screen={sideBrowse.page ? "detail" : "list"}
            onOpen={() => setSideBrowse({ page: true })}
            onBack={sideFrom === "recent" ? backToRecent : () => setSideBrowse({ page: false })}
            bare={{ here: selectedRow.id }}
            homeOf={homeOf}
          />
        );
      case "machine": {
        // The session's machine: its name, then its card (UX-39 §3).
        const row = devices.find((d) => d.name === selectedRow.home) ?? null;
        if (row === null) return null;
        return (
          <MachineCard
            row={row}
            narrow
            onBack={() => {}}
            onRecognition={openRecognition}
          />
        );
      }
      case "quota":
        return (
          <QuotaLanding
            key={sideEntry}
            narrow
            stale={stale}
            devices={devices}
            pane={null}
            // The landing's search and ticks are the landing's: the seat
            // shows every account, and lands on this session's.
            query=""
            onQuery={() => {}}
            words={[]}
            onToggleWord={() => {}}
            screen={sideQuota}
            onOpen={() => setSideQuota("detail")}
            onBack={() => setSideQuota("list")}
            landOn={selectedRow.category}
            bare
          />
        );
      case "store":
        // 存储 (STORE-BRIEF phase 1), this machine first; a file opens
        // in place, and 在文件视图中打开 is the 工作区 seat's tree.
        return (
          <StorePage
            key={sideEntry}
            devices={devices}
            spot={sideStoreResume?.session === selectedRow.id ? sideStoreResume.spot : null}
            onSpot={(spot) => setSideStore({ session: selectedRow.id, spot })}
            onChanged={() => {
              refresh().catch(() => {});
            }}
            onReveal={(machine, path) => sideShow(machine, levelOf(path), null)}
          />
        );
    }
  };
  /**
   * **The column extends the window to the right; it does not take the
   * session's width** (UX-39, the owner 2026-10-06 23:15: 「保留 app 宽度，
   * 拓展到右侧的，就像现在在工作区点 chat 一样」 — `WorkspacePane`'s
   * `showSessions`, rulings 2026-09-16 and 51). The window grows first and
   * the column arrives into the room it made; shut, the window gives back
   * exactly the frame it had (UX-46 §2, the owner 2026-10-07: 「打开多大，
   * 关闭了不影响原来的大小」) — the app keeps that account (`widenWindow`),
   * by any road the column goes: the tab pressed again, the seam, leaving
   * the session. Where the window cannot grow (the screen's edges,
   * fullscreen, the web face) the column takes its room off the session,
   * as before.
   *
   * **Nothing beside the column moves while the window does** (UX-46 §3,
   * the owner: 「收起附栏的时候，按钮从左边跑到右边去了」). The column went
   * first and the window followed a moment later, so for that moment the
   * strip ran to the still-wide window's edge and its end — the verbs
   * seat's ↻, the front tab's × — jumped right, then back as the window
   * narrowed; opening did the same the other way round. The session's
   * side is held at its width (`sideHold`) from before the ask until the
   * window has answered, and the column comes or goes in the same frame
   * the hold lets go.
   *
   * A narrow window draws no column and asks nothing: the room stays
   * on the account while the 附栏 stays open, and its close asks for it
   * back — which a window the person resized (into the narrow layout or
   * not) declines: their size stands (the lead's ruling, 2026-10-07).
   *
   * **The column takes what the right edge has, down to its floor**
   * (sb-a, 2026-10-09, the owner: 「侧栏展开时, 原来的页面位置不要动, 侧栏
   * 展开自己额外扩展出去」): it asks for its seat's width (640, or what
   * was dragged there, per seat) and the least it lives with (360); the
   * window answers what it grew by (`room.rs`), and the column is that
   * wide for this opening. **A seat of another width refits the window**:
   * the room is given back and asked for again at the new width, the
   * session held still between. **As the window narrows the column gives
   * way first** (「缩小窗口不能把整个顶部导航给略缩畸形了」): the session
   * stands at a flex basis of what it had beside the column (`sideBasis`)
   * and barely shrinks, the column's basis shrinks to its floor, and only
   * then does the session narrow, to `--session-main-min`.
   */
  const sideWanted = sessions && side.open && selectedRow !== null;
  const [sideShown, setSideShown] = useState(false);
  const [sideHold, setSideHold] = useState<number | null>(null);
  const sideMain = useRef<HTMLDivElement | null>(null);
  /** The row the session and the column share. */
  const sideRow = useRef<HTMLDivElement | null>(null);
  /** The width the seat asks for: dragged there, else the token's. */
  const sideKept = useSideWidth(side.seat);
  const sidePx = (name: string) => Number.parseFloat(getComputedStyle(document.documentElement).getPropertyValue(name)) || 0;
  /** What the window gave this opening, where it gave less than asked
      (the right edge); `null` = the asked width. */
  const [sideFit, setSideFit] = useState<number | null>(null);
  /** The session's flex basis beside the column (above). */
  const [sideBasis, setSideBasis] = useState<number | null>(null);
  /** Whether the window holds room for the column now. */
  const sideAsked = useRef(false);
  /** The width the window was last asked for, to know a seat's own. */
  const sideAskedW = useRef<number | null>(null);
  /** Bumped each turn: an answer from the window that a later turn
      overtook changes nothing. */
  const sideTurn = useRef(0);
  /** The session's basis from the row as it stands, less the column. */
  const baseBeside = (paneW: number) => {
    const row = sideRow.current?.getBoundingClientRect().width;
    setSideBasis(row === undefined ? null : Math.max(0, row - paneW));
  };
  // A layout effect: the hold is measured with the column still drawn and
  // set in the same frame it goes, before anything is painted.
  useLayoutEffect(() => {
    const turn = ++sideTurn.current;
    const want = sideKept ?? sidePx("--session-side-w");
    const least = sidePx("--session-side-min");
    const hold = () => setSideHold(sideMain.current ? sideMain.current.getBoundingClientRect().width : null);
    /** The window's answer, as this opening's width. */
    const fit = (grew: number) => {
      const w = grew <= 0 ? want : Math.max(grew, least);
      setSideFit(w === want ? null : w);
      return w;
    };
    const ask = () =>
      widenWindow("session-side", want, least).then((grew) => {
        if (turn !== sideTurn.current) return;
        sideAskedW.current = want;
        baseBeside(fit(grew));
        setSideShown(true);
        setSideHold(null);
      });
    if (sideWanted && !narrow && sideAsked.current && sideShown && sideAskedW.current !== null && sideAskedW.current !== want) {
      // Another seat, another width: the room given back and asked again.
      hold();
      void giveBackWindow("session-side").then((gave) => {
        if (turn !== sideTurn.current) return;
        if (gave > 0) return ask();
        // The person resized the window meanwhile: their size stands, and
        // the column takes its new width off the session.
        sideAskedW.current = want;
        setSideFit(null);
        baseBeside(want);
        setSideHold(null);
      });
      return;
    }
    if (sideWanted === sideAsked.current || (sideWanted && narrow)) {
      setSideShown(sideWanted && !narrow);
      setSideHold(null);
      return;
    }
    hold();
    if (sideWanted) {
      sideAsked.current = true;
      void ask();
      return;
    }
    sideAsked.current = false;
    sideAskedW.current = null;
    setSideShown(false);
    setSideFit(null);
    void giveBackWindow("session-side").then(() => turn === sideTurn.current && setSideHold(null));
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [sideWanted, narrow, sideKept, side.seat]);
  /** The seam moved by hand: the seat keeps the width, and the session
      gives or takes the same pixels (the window does not change). */
  const sideDragged = (w: number | null) => {
    const to = w ?? sidePx("--session-side-w");
    sideAskedW.current = w === null ? sidePx("--session-side-w") : w;
    setSideFit(null);
    baseBeside(to);
    setSideWidth(side.seat, w);
  };
  const sideLive = (w: number | null) => {
    const row = sideRow.current?.getBoundingClientRect().width;
    if (row !== undefined && sideMain.current) sideMain.current.style.flexBasis = `${Math.max(0, row - (w ?? sidePx("--session-side-w")))}px`;
  };
  // Drawn until the window effect lets it go (`sideShown`), not until the
  // store says closed: the hold is measured with it still there.
  const sessionSide =
    sideShown && selectedRow ? (
      <SessionSide
        embedded
        maximized={content.maximized === selectedRow.id}
        key={selectedRow.id}
        session={selectedRow.id}
        width={sideFit ?? sideKept}
        seat={side.seat}
        onSeat={enterSeat}
        onWidth={sideDragged}
        onLive={sideLive}
        onClose={() => { content.setMaximized(null); setSessionSide(selectedRow.id, { open: false }); }}
        boxRef={setDetailBox}
        who={{ face: selectedRow.face, title: termName(selectedRow), word: selectedRow.word, kind: selectedRow.kind, category: selectedRow.category }}
      >
        <div ref={seatContent} className="flex min-h-0 min-w-0 flex-1 flex-col" />
      </SessionSide>
    ) : undefined;

  const detail = sessions && isNewTab(selected) ? (
    // A new tab's page (UX-45): the wizard's questions laid out in the
    // tab, one per new tab.
    <NewSessionPage
      key={selected}
      machine={newTabs.current.get(selected ?? "")?.at.machine}
      dir={newTabs.current.get(selected ?? "")?.at.dir}
      onCancel={() => selected && leaveNewTab(selected)}
      onOpened={(id) => {
        if (!selected) return;
        // **Made from a workspace: back to the workspace** (ruling 34,
        // 「应该是在工作区就地打开」): the column beside its files is
        // where the session belongs.
        if (newTabs.current.get(selected)?.at.inPlace) {
          leaveNewTab(selected);
          setOpenedInWs(id);
          return;
        }
        newTabs.current.delete(selected);
        justOpened.current.set(id, Date.now());
        sessionTabs.become(selected, id);
      }}
    />
  ) : sessions && selectedRow === null && isChatTab(selected) ? (
    // That conversation's page, before its row is back (UX-45).
    <MachineChat key={selected} id={selected ?? ""} sessions={rows} active={!hidden && morePage === null && !switchingTabs && (!narrow || screen === "detail")} onSession={(id) => { setMorePage(null); sessionTabs.open(id); setScreen("detail"); }} />
  ) : sessions ? (
    // The conversation in this pane is painted inside the workspace it
    // is running in, as far as its paths are concerned (2026-09-18
    // ruling 54). **Null where the row has no directory**, which is
    // `linkHome`'s own default and leaves every path as the prose it
    // already was.
    <LinkHomeProvider value={sessionHome}>
    <ShowFileProvider value={showFile}>
    <DetailPane
      row={selectedRow}
      sessions={rows}
      active={!hidden && morePage === null && !switchingTabs && (!narrow || screen === "detail")}
      onSession={(id) => { setMorePage(null); sessionTabs.open(id); setScreen("detail"); }}
      narrow={narrow}
      openIds={openTabs}
      day={selectedRow ? dayOf(selectedRow.id) : undefined}
      onBack={() => setScreen("list")}
      // The empty pane's 新会话…: a new tab, as ＋'s (UX-45).
      onNew={() => openNewTab()}
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
        waitingFor !== null ? (
          <Waiting
            since={waitingFor}
            rows={false}
            onRetry={() => {
              refresh().catch(() => {});
            }}
          />
        ) : (
          <MeshNow
            rows={rows}
            devices={devices}
            onPick={onSelect}
            onDevice={(d) => {
              setLanding("devices");
              onSelectDevice(d);
            }}
          />
        )
      }
    />
    </ShowFileProvider>
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
        // The tree in the list column (wide), while the list is drilled
        // into this workspace (`wsTree`); the narrow face's tree is a
        // screen of the pane's own.
        side={narrow ? null : { on: wsTree, pane: listPane, set: setWsTree }}
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
        machineFace={devices.find((d) => d.id === openWs.device)?.face ?? null}
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
        back={wsBack}
        onBackSeen={() => setWsBack(false)}
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
      rows={devices}
      onPromote={(r) => setPromoting(r.name)}
      // An open file is left first (`MachineFiles`, one back button).
      onBack={() => (fileBack ? fileBack.go() : setScreen("list"))}
      onRecognition={openRecognition}
      actions={narrow ? machineVerbs : undefined}
      body={
        !openDevice ? undefined : machineBody === "web" ? (
          <MachineWeb key={openDevice.id} machine={openDevice} devices={devices} />
        ) : machineBody === "files" ? (
          <MachineFiles key={openDevice.id} machine={openDevice.name} here={openDevice.me} onOpenFile={setFileBack} start={fileStart} />
        ) : machineBody === "sessions" ? (
          // The rows' hairlines end on the page's right line (W-V5): the
          // list's own 8 and its row's 12 inside, the rest here.
          <div data-machine-sessions data-nav-clear className="min-h-0 flex-1 overflow-y-auto ps-rows-inset pe-[calc(var(--page-inset)-var(--gap-in))]">
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
          // Nothing more is written on the way out (save-a): the
          // window being hidden as it closes would flush otherwise.
          discardAll();
          quitNow();
        }}
        onKeep={() => setQuitting(null)}
      />
      <ImportDialog
        open={dialog === "import"}
        onOpenChange={(o) => setDialog(o ? "import" : null)}
        // The first one imported is what the person came to see, the
        // way a freshly opened session is (`NewSessionPage`).
        onImported={(ids) => {
          if (ids.length === 0) return;
          setLanding("sessions");
          sessionTabs.open(ids[0]);
          setScreen("detail");
        }}
      />
      {/* ⚠️ 墓碑 (UX-45): the 新建会话 dialog. Every road opens a new tab
          whose page asks the same (`NewSessionPage`). */}
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
      {/* The machine page's VS Code: its consent and its refusal. */}
      {machineCode.dialog}
      <AddMachineDialog
        open={dialog === "add" || dialog === "add-ticket" || promoting !== null}
        onOpenChange={(o) => {
          if (o) return;
          setDialog(null);
          setPromoting(null);
        }}
        machines={devices}
        start={dialog === "add-ticket" ? "ticket" : "pick"}
        promote={promoting}
        onAdded={() => {
          fetchDevices()
            .then(setDevices)
            .catch(() => {});
          refresh().catch(() => {});
        }}
        onOpenSession={(id) => {
          setLanding("sessions");
          sessionTabs.open(id);
          setScreen("detail");
        }}
      />
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
  const fullScreen =
    inSession || (narrow && browses && morePage === null && screen === "detail") || (narrow && fullClaims > 0);
  /** **The terminal pill opens the sheet** (UX-17): the 工作区 seat at
      the shell's directory now (the terminal's OSC 7 marks), else where
      it started — read at the tap, off the terminal on screen. */
  // Every session's pill opens its 附栏 (UX-30 §6 — the column stands
  // beside a terminal and a conversation alike); ⚠️ 墓碑 (UX-17): the
  // terminal's alone, for its directory.
  const terminalPill = selectedRow !== null;
  const sessionScreen = (
    <SessionScreen
      faces={tabFaces}
      active={selected ?? ""}
      contentMode={narrow && side.open && selectedRow !== null}
      onTabs={narrow&&selectedRow?()=>{(document.activeElement as HTMLElement|null)?.blur();contentNavigation.sessions(selectedRow.id);setSessionSide(selectedRow.id,{open:true,seat:"workspace"});}:undefined}
      onTitle={terminalPill && selectedRow ? () => {
        (document.activeElement as HTMLElement | null)?.blur();
        if (!side.open) contentNavigation.resume(selectedRow.id);
        setSessionSide(selectedRow.id, { open: !side.open, seat: "workspace" });
      } : undefined}
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
      // ＋ in the tabs sheet: a new tab, its page the choices (UX-45).
      onNew={() => openNewTab()}
      newLabel={gui.new_session}
      onBack={() => setScreen("list")}
      caption={stale ? gui.backend_unreachable : undefined}
      // ⚠️ 墓碑 (UX-31 → NOTICE-BRIEF, the owner 2026-10-08: 「别的会话
      // 不用提醒」): the window's foot hung under the pill here — what was
      // happening in other sessions, growing the pill a row. The list's
      // lights say it; the pill says only what stops this session.
      away={placeSheet !== null && selectedRow !== null}
    >
      {detail}
      {narrow && side.open && selectedRow && tabFaces.find(f => f.id === selectedRow.id) && <>
        <MobileContentSheet onClose={() => { (document.activeElement as HTMLElement | null)?.blur(); setSessionSide(selectedRow.id, { open: false }); }}><div ref={seatContent} className="flex min-h-0 min-w-0 flex-1 flex-col" /></MobileContentSheet>
      </>}

      {placeSheet && selectedRow && (
        <SessionPlaceSheet
          key={selectedRow.id}
          open
          onOpenChange={(o) => !o && setPlaceSheet(null)}
          seat={side.seat}
          onSeat={enterSeat}
          boxRef={setDetailBox}
          who={{ face: selectedRow.face, title: termName(selectedRow), word: selectedRow.word, kind: selectedRow.kind, category: selectedRow.category }}
        >
          {sideSeat()}
        </SessionPlaceSheet>
      )}
    </SessionScreen>
  );

  /** Out of a 「…」 page. Narrow, a settings page goes one screen back to
      设置's list; 我的 khor and 诊断 are 更多's own items (UX-16), pushed
      over the landing they were opened from, so they go back to it. */
  const leaveMorePage = () => {
    if (narrow && morePage !== null && inSettings(morePage)) {
      setScreen("list");
      return;
    }
    setMorePage(null);
    if (narrow) setScreen(seatScreen.current[landing] ?? "list");
  };
  /** Where a swipe from the left edge goes (F7.3, narrow): the same
      place each screen's ‹ goes, or nowhere on a screen with no back. */
  const swipeBack: (() => void) | null = !narrow
    ? null
    : switchingTabs && openTabs.length > 0
      ? () => setSwitchingTabs(false)
      : screen !== "detail"
        ? null
        : morePage
          ? leaveMorePage
          : landing === "devices" && fileBack
            ? fileBack.go
            : () => setScreen("list");
  const [swipeHost, setSwipeHost] = useState<HTMLDivElement | null>(null);
  useEdgeSwipe(swipeHost, swipeBack);
  // **Esc is the same way back** (UX-7, keyboard only): a narrow screen's
  // ‹, for someone without a pointer. Only where nothing nearer wants the
  // key — a field or the terminal keeps Esc (a shell needs it), and an
  // open sheet or menu closes first (its own Escape runs before this, in
  // the bubble phase, and it is still in the document when this asks).
  const backKey = useRef(swipeBack);
  backKey.current = swipeBack;
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.key !== "Escape" || e.defaultPrevented || e.isComposing) return;
      if (e.metaKey || e.ctrlKey || e.altKey || e.shiftKey) return;
      const go = backKey.current;
      if (!go) return;
      const at = document.activeElement;
      if (at instanceof HTMLInputElement || at instanceof HTMLTextAreaElement || (at instanceof HTMLElement && at.isContentEditable)) return;
      if (document.querySelector('[role="dialog"], [role="alertdialog"], [role="menu"], [role="listbox"]')) return;
      e.preventDefault();
      go();
    };
    window.addEventListener("keydown", key);
    return () => window.removeEventListener("keydown", key);
  }, []);
  // Which kind of swipe this screen has: a file moves alone, over its
  // listing (`app.css` `data-swipe-kind`).
  const swipesFile = landing === "devices" && fileBack !== null;
  useEffect(() => {
    document.documentElement.dataset.swipeKind = swipesFile ? "file" : "page";
  }, [swipesFile]);
  // The list a page was opened from stays a moment under it, fading,
  // while the page comes in (V19) — the same layer a swipe uncovers.
  const [leaving, setLeaving] = useState(false);
  const shownKey = `${morePage ?? landing}:${screen}`;
  const lastShown = useRef(shownKey);
  useEffect(() => {
    const was = lastShown.current;
    lastShown.current = shownKey;
    if (!narrow || was === shownKey || !was.endsWith(":list") || screen !== "detail") return;
    setLeaving(true);
    const t = window.setTimeout(() => setLeaving(false), 280);
    return () => window.clearTimeout(t);
  }, [narrow, shownKey, screen]);

  const morePageEl =
    morePage === null ? null : morePage === "connection" ? (
      <ConnectionPage
        devices={devices}
        onClose={leaveMorePage}
        // The device table is the thing 暂停 changes, and every surface on
        // this screen paints off it.
        onChanged={() => {
          refresh().catch(() => {});
        }}
      />
    ) : morePage === "appearance" ? (
      <AppearancePage onClose={leaveMorePage} onSettingChanged={settings.reread} me={me} />
    ) : morePage === "terminal" ? (
      <TerminalPage onClose={leaveMorePage} devices={devices} machine={recognitionMachine} onMachine={setRecognitionMachine} />
    ) : morePage === "session" ? (
      <SessionPage onClose={leaveMorePage} onSettingChanged={settings.reread} />
    ) : morePage === "workspace" ? (
      <WorkspacePage onClose={leaveMorePage} onSettingChanged={settings.reread} />
    ) : morePage === "security" ? (
      <SecurityPage
        onClose={leaveMorePage}
        lock={lock.facts}
        onLockChanged={lock.reread}
        onLockNow={lock.now}
        onSettingChanged={settings.reread}
      />
    ) : morePage === "about" ? (
      <AboutPage onClose={leaveMorePage} />
    ) : morePage === "doctor" ? (
      <DoctorPage devices={devices} onClose={leaveMorePage} />
    ) : morePage === "store" ? (
      <StoreScreen
        devices={devices}
        onClose={leaveMorePage}
        onChanged={() => {
          refresh().catch(() => {});
        }}
        onReveal={showFile}
      />
    ) : (
      <MyKhorPage
        devices={devices}
        rows={rows}
        onClose={leaveMorePage}
        // The card's acts (UX-26 rule 6): each one is a road the app
        // already has — the machine's page on its sessions, the wizard
        // with the machine picked, a landing narrowed to the machine
        // (the filter words `lib/list-filter` reads), the ＋ menu's
        // invite / join windows, 设置 → 连接 for the relays.
        onOpenMachine={(row, body) => {
          setMorePage(null);
          setLanding("devices");
          onSelectDevice(row);
          bodyNext.current = body === "sessions" ? "sessions" : null;
        }}
        onNewSession={(row) => openNewSession(row.me ? "" : row.name)}
        onOpenWorkspaces={(row) => {
          setMorePage(null);
          setWords((was) => ({ ...was, workspaces: [`dev:${row.id}`] }));
          rememberWords("workspaces", [`dev:${row.id}`]);
          setLanding("workspaces");
          setScreen("list");
          listPane.expand();
        }}
        onOpenBlocked={(row) => {
          setMorePage(null);
          setWords((was) => ({ ...was, sessions: [`dev:${row.name}`, "state:blocked"] }));
          rememberWords("sessions", [`dev:${row.name}`, "state:blocked"]);
          setLanding("sessions");
          setScreen("list");
          listPane.expand();
        }}
        onPin={onPinDevice}
        onForget={onForgetDevice}
        onAdd={(start) => setDialog(start === "ticket" ? "add-ticket" : "add")}
        onOpenRelays={() => {
          if (trafficLights && !settingsFace) {
            openSettings("connection").catch(() => {});
            return;
          }
          setMorePage("connection");
          setScreen("detail");
          listPane.expand();
        }}
        onChanged={() => {
          refresh().catch(() => {});
        }}
        onReset={() => {
          refresh().catch(() => {});
          lock.reread();
        }}
      />
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
          {/* 更多's list on a wide face (我的 khor · 设置's seven · 诊断);
              the settings window's own sidebar is 设置. */}
          <h1 data-list-title className="min-w-0 flex-1 truncate text-title-lg">
            {settingsFace ? gui.nav_settings : gui.rail_more}
          </h1>
        </SideSeat>
      )}
      {/* **这台机器 first, on the wide face** (the owner, 2026-10-02: its
          face left the top row and lives in 更多): this machine's face and
          name, over the word for the page. Narrow keeps it in its place
          among the sections; its 更多 menu already leads to it. */}
      {/* **The settings window holds settings and nothing else** (UX-16
          ruling, 2026-10-05): no 我的 khor at its head, no 诊断 at its
          foot — the app menu opens those two in the main window. */}
      {!narrow && !settingsFace && (
        <button
          type="button"
          data-more-open="me"
          data-more-me
          data-on={morePage === "me"}
          // **One Tab stop for the list** (F3.8): the open section's row,
          // or this first one; ↑ ↓ walk the rest.
          tabIndex={morePage === null || morePage === "me" ? 0 : -1}
          onClick={() => {
            setMorePage("me");
            setScreen("detail");
          }}
          className={cn(
            "mx-in mb-xxs flex min-h-row-2 items-center gap-row rounded-md px-3 text-start text-headline focus-visible:-outline-offset-2",
            morePage === "me" ? "bg-fill-strong" : "hover:bg-fill",
          )}
        >
          <span className="flex w-list-face flex-none items-center justify-center">
            <MachineAvatar face={me?.face ?? null} className="size-avatar flex-none" />
          </span>
          <span className="flex min-w-0 flex-col">
            <span className="truncate">{me?.shown ?? moreName("me")}</span>
            <span className="truncate text-callout text-muted-foreground">{moreName("me")}</span>
          </span>
        </button>
      )}
      {/* **Narrow: 设置's seven, as iOS Settings' first screen** (UX-16):
          one inset-grouped card on the ground, a row per page — its
          glyph, its name, the chevron that says it opens a page — with
          the card's hairlines between them. 我的 khor and 诊断 are 更多's
          own items and are not here. */}
      {narrow ? (
        <div data-settings-card data-more-card className="mx-page mt-in mb-pane flex min-w-0 flex-col">
          {morePageOrder.filter(inSettings).map((p) => (
            <button
              key={p}
              type="button"
              data-setting-row="nav"
              data-more-open={p}
              tabIndex={p === (morePage ?? "appearance") ? 0 : -1}
              onClick={() => {
                setMorePage(p);
                setScreen("detail");
              }}
              className="flex min-h-touch items-center gap-row px-row text-start text-body focus-visible:-outline-offset-2"
            >
              <span data-more-glyph={p} className="flex flex-none items-center justify-center text-rail-ink [&>svg]:size-nav-glyph">
                {(() => {
                  const Glyph = MORE_GLYPH[p as Exclude<MorePage, "me">];
                  return <Glyph className="size-nav-glyph" />;
                })()}
              </span>
              <span className="min-w-0 flex-1 truncate">{moreName(p)}</span>
              <IconChevronRight className="size-affordance flex-none text-tertiary" />
            </button>
          ))}
        </div>
      ) : (
        // **Wide** (and the Mac settings window's sidebar): 我的 khor at
        // the head (above), 设置's seven, then — a step apart — 诊断.
        morePageOrder
          .filter((p) => (settingsFace ? inSettings(p) : p !== "me"))
          .map((p) => (
            <button
              key={p}
              type="button"
              data-more-open={p}
              data-on={morePage === p}
              tabIndex={morePage === p ? 0 : -1}
              onClick={() => {
                setMorePage(p);
                setScreen("detail");
              }}
              // **A list row like every other list's** (the audit fix plan
              // F2.1): the glyph in the face's slot so the words start on
              // the x the 我的 khor row's do, the title `headline`, and the
              // hairline under the words.
              className={cn(
                // The ring inside the row, as every list row's (W-V6).
                "mx-in flex min-h-touch items-stretch gap-row rounded-md ps-3 text-start text-headline focus-visible:-outline-offset-2",
                p === "store" && "mt-group",
                morePage === p ? "bg-fill-strong" : "hover:bg-fill",
              )}
            >
              <span data-more-glyph={p} className="flex w-list-face flex-none items-center justify-center text-rail-ink [&>svg]:size-nav-glyph">
                {(() => {
                  const Glyph = MORE_GLYPH[p as Exclude<MorePage, "me">];
                  return <Glyph className="size-nav-glyph" />;
                })()}
              </span>
              <span
                className={cn(
                  "flex min-w-0 flex-1 items-center gap-row py-1 pe-3",
                  // Its hairline ends where a list's does (W-V10); none
                  // under the last of the seven or under 诊断.
                  p !== "about" && p !== "doctor" && "border-b",
                  morePage === p && "border-transparent",
                )}
              >
                <span className="min-w-0 flex-1 truncate">{moreName(p)}</span>
              </span>
            </button>
          ))
      )}
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
      homeOf={homeOf}
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
  //
  // **Narrow, it is the screen's band** (`lib/scroll-edge`, round HEADS):
  // the screen under it scrolls beneath it on glass, which reaches up
  // under the status bar. Wide, it is the second half of the left
  // column's band (`[data-wide-band]`).
  /** **Where the window is**, for the seams (`components/Fault`): a part
      that failed to draw is drawn again once a person goes somewhere
      else, the way leaving a page and coming back would. */
  const seamAt = [landing, morePage ?? "", selected ?? "", selectedDevice ?? "", openWs?.device ?? "", openWs?.path ?? ""].join("\u0000");
  const sideHead = (
    <div
      data-side-l0
      {...WINDOW_DRAG}
      ref={narrow ? band : undefined}
      data-band={narrow || undefined}
      data-band-reach={narrow || undefined}
      className="flex min-w-0 flex-none flex-col"
    >
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
  /**
   * **What a back swipe uncovers, complete** (F7.3; V01): the page the
   * swipe goes to, as it is drawn when it is the screen — its own first
   * row (`underSeats`) and the bottom bar, at the y they have then — so
   * nothing jumps or blinks when the finger lets go. Also what a list
   * leaves under the page it opens (V19), fading as that page comes in.
   *
   * - A landing's page uncovers the landing's list (额度's and the
   *   browser's are their own landings drawn on their list screen).
   * - A settings page uncovers 设置's list; 我的 khor and 诊断 uncover
   *   the landing they were opened over (UX-16).
   * - The tab switcher uncovers its page's own chrome — the session bar,
   *   without its terminal: a live terminal is never drawn twice.
   * - A file uncovers nothing here: its listing is the machine page's
   *   own, under the file (`MachineFiles`), and only the file moves.
   */
  const under: ReactNode =
    !narrow || swipeBack === null || swipesFile
      ? null
      : switchingTabs
        ? (
            <SessionScreen
              faces={tabFaces}
              active={selected ?? ""}
              onFocus={() => {}}
              onClose={() => {}}
              onSwitch={() => {}}
              switching={false}
              onBack={() => {}}
            >
              {null}
            </SessionScreen>
          )
        : morePage !== null && inSettings(morePage)
          ? moreList
          : quotas
            ? (
                <QuotaLanding
                  narrow
                  stale={stale}
                  devices={devices}
                  pane={null}
                  query={queries.quota}
                  onQuery={setQuery}
                  words={words.quota}
                  onToggleWord={toggleWord}
                  screen="list"
                  onOpen={() => {}}
                  onBack={() => {}}
                />
              )
            : browses
              ? (
                  <BrowseLanding
                    key={browsing.kind.kind}
                    kind={browsing.kind}
                    devices={devices}
                    tabs={browsing.tabs}
                    narrow
                    pane={null}
                    screen="list"
                    onOpen={() => {}}
                    onBack={() => {}}
                  />
                )
              : list;
  /** Whether the page under draws its own bottom bar: only where the page
      on top has none (a session, full screen) — elsewhere the one on
      screen is already the list's, and stays put. Never under the
      switcher, whose page is a session. */
  const underNav = !switchingTabs && !settingsFace;
  /** **The landings' bar belongs to the landings** (2026-10-03, the owner
      on a phone: the capsule sat over a file's text). It floats over a
      landing's own list — 机器, 会话, 额度, 更多's list and the landings
      inside 更多 — and over nothing reached from one: a machine, a
      workspace and the file open in it, an 额度 account, a 「…」 page, a
      session, a browsing page. Those are a step in, with 「‹」 (and the
      edge) as their way out, the way an iPhone app pushes a page over
      its tab bar. The bar comes back under the finger as the page is
      swiped away: the page under is the list, and it draws its own
      (`underNav`). */
  const onLanding =
    morePage !== null || quotas || browses
      ? screen !== "detail"
      : !(switchingTabs && openTabs.length > 0) && !(screen === "detail" && detail);
  // **The settings window is the settings page alone** (UX-19, the owner
  // 2026-10-05: the window carried the bar's capsule and 更多): no bar in
  // it at any width.
  const navShown = !switchingTabs && onLanding && !settingsFace;

  return (
    <SettingsProvider value={settings.rows}>
    <SettingsRereadProvider value={settings.reread}>
    <SideSeatsProvider value={seats.seats}>
    <HereHostsProvider value={hosts}>
    <FullScreenProvider value={claimFull}>
    <TooltipProvider>
      {narrow ? (
        // **As tall as `#root`, not the screen** (2026-10-02): `#root`
        // follows what the soft keyboard leaves (`lib/viewport`), and a
        // `100dvh` would not — the keyboard covers, it does not resize.
        // **Clear of a phone's status bar** (`--safe-top`) on every
        // screen but a session's, which runs its paper up to the glass
        // and keeps the edge itself (`SessionScreen`).
        //
        // **The band under the status bar wears the ground of the row
        // under it** (2026-10-02, the owner on a phone: the area where
        // the system shows its own things was grey over a dark app). The
        // band is this box's `pt-safe-top`, so the box paints L0's
        // ground (`bg-side`, the one ground since the redesign) and the
        // status bar sits on the first row's colour, both themes. Zero
        // high wherever there is no status bar (`--safe-top` is 0 off a
        // phone).
        <div data-nav-host={!fullScreen || undefined} data-nav-shown={navShown || undefined} className={cn("relative flex h-full flex-col", !fullScreen && "bg-side pt-safe-top")}>
          {under && (
            <div
              data-screen-under
              data-leaving={leaving || undefined}
              aria-hidden
              inert
              className="absolute inset-0 flex flex-col overflow-hidden bg-background pt-safe-top"
            >
              <SideSeatsProvider value={underSeats.seats}>
                {!switchingTabs && (
                  <div data-side-l0-under className="flex min-w-0 flex-none flex-col">
                    <SideHead narrow bare={false} seatRef={underSeats.setHead} claimed={underSeats.claimed} onJump={() => {}} />
                  </div>
                )}
                <div className="relative flex min-h-0 flex-1 flex-col overflow-hidden">{under}</div>
                {underNav && nav}
              </SideSeatsProvider>
            </div>
          )}
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
          <div ref={setSwipeHost} data-swipe-host className="relative flex min-h-0 flex-1 flex-col overflow-hidden">
          <div
            key={morePage ? `more:${screen === "detail" ? morePage : "list"}` : screen}
            data-screen={morePage ? "more" : screen}
            // ⚠️ 墓碑 (UX-30): `ref={setDetailBox}` — the day's axis was this
            // screen's; it is the pill sheet's now (`SessionPlaceSheet`).
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
            <Fault
              name="page"
              reset={seamAt}
              // The screen's 「‹」, which the page that failed would have
              // drawn: where the edge swipe goes (`swipeBack`).
              head={
                swipeBack && (
                  <NarrowHead>
                    {(seated) =>
                      seated && (
                        <header data-fault-head className="flex min-w-0 flex-1 items-center gap-in self-stretch">
                          <GlassCircle aria-label={gui.back} data-back onClick={swipeBack}>
                            <IconBack />
                          </GlassCircle>
                        </header>
                      )
                    }
                  </NarrowHead>
                )
              }
            >
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
                      devices={devices}
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
            </Fault>
          </div>
          </div>
          {/* The navigation floats over the landings and nothing else
              (`navShown`; the owner, 2026-10-02: 「会话里不显示」, and
              2026-10-03: not over a file either). */}
          {navShown && nav}
          {dialogs}
          {/* Inside a session the strip is the deck pill's (UX-31); the
              foot keeps it everywhere else on the narrow face. */}
          {!inSession && <StatusBar items={status.items} onDismiss={status.dismiss} />}
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
            hidden={folded || dial}
            className="flex flex-none flex-col border-r bg-side"
            style={{ "--list-w": `${listPane.width}px` } as CSSProperties}
          >
            {/* **The window's top row and the title row are one band**
                over the column's rows (`lib/scroll-edge`, round HEADS):
                the list passes under both on one glass. */}
            <div ref={band} data-band data-wide-band className="flex flex-none flex-col">
              {/* The settings window has the system's title bar and no
                  landings: no first row of khor's own (UX-10). */}
              {!settingsFace && (
                <div
                  data-wide-top
                  {...WINDOW_DRAG}
                  // **Across the column** (UX-19, the owner 2026-10-05): the
                  // window buttons' room at the left where there are window
                  // buttons (`app.css`: the title row's inset where there are
                  // not), then the five-seat capsule to the title row's right
                  // inset (`pe-row`) — 更多 over the search circle. ⚠️ 墓碑:
                  // right-aligned, its own width (「这要右对齐」, 10-02).
                  className="flex h-pane-head flex-none items-center ps-wide-lead-start pe-row select-none"
                >
                  {/* Folded, the strip stands in the content's lead
                      (`foldedLead`) and only there: a second copy in the
                      hidden column was a second set of the same buttons. */}
                  {!folded && strip}
                </div>
              )}
              {sideHead}
            </div>
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
          {/* One seam per landing (`components/Fault`); the board's own
              content has a seam of its own inside, so a broken page
              keeps its R0 and the list beside it. */}
          <Fault name="page" reset={seamAt}>
          {dial ? (
            morePageEl
          ) : morePage ? (
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
                // The settings window closes as a window does (⌘W, the
                // red button); a ✕ of its own would be a second way out.
                settingsFace ? undefined : (
                  <GlassCircle aria-label={gui.close} data-more-close onClick={() => setMorePage(null)}>
                    <IconClose />
                  </GlassCircle>
                )
              }
            >
              <Fault name="more" reset={seamAt}>
                {morePageEl}
              </Fault>
            </Region>
          ) : quotas ? (
            <QuotaLanding
              narrow={false}
              stale={stale}
              devices={devices}
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
              {/* Drilled into a workspace, the column is its tree, drawn
                  by the pane (`WorkspacePane` `side`). */}
              {!(landing === "workspaces" && openWs !== null && wsTree) && list}
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
              {/* ⚠️ 墓碑 (UX-30): `ref={setDetailBox}` stood here — the day's
                  axis was this pane's; it is the column's now
                  (`SessionSide` `boxRef`). */}
              {sessions && openTabs.length > 0 ? (
                <VerbSeat.Provider value={verbSeat}>
                <Tabs
                  grouped
                  bodyClass="flex min-h-0 flex-col"
                  faces={tabFaces.filter(f => !sessionTabsFocused || isNewTab(selected ?? "") || f.id === selected).map(f => ({ ...f,
                    again: !isNewTab(f.id) ? {
                      hint: sessionTabsFocused ? gui.session_tabs_all : gui.session_tabs_focus,
                      expanded: sessionTabsFocused,
                      toggle: () => {
                        if (!sessionTabsFocused) {
                          contentNavigation.resume(f.id);
                          setSessionSide(f.id, {open:true,seat:"workspace"});
                        }
                        setSessionTabsFocused(value => !value);
                      },
                    } : undefined,
                    group: !isNewTab(f.id) ? <ContentTabs session={f.id} open={f.id === selected && sessionTabsFocused} content={content} navigation={contentNavigation} onHide={()=>{content.setMaximized(null);setSessionSide(f.id,{open:false});}} onFocus={()=>{sessionTabs.focus(f.id);setSessionSide(f.id,{open:true,seat:"workspace"});}} onAdd={()=>{
                    contentNavigation.browse(f.id);setSessionSide(f.id,{open:true,seat:"workspace"});
                  }} /> : undefined }))}
                  active={selected ?? ""}
                  seek={sessionTabs.picks}
                  onFocus={id => { sessionTabs.focus(id); setScreen("detail"); }}
                  onClose={sessionTabs.close}
                  onNew={sessionTabsFocused && !isNewTab(selected) ? undefined : () => openNewTab()}
                  onReorder={sessionTabs.reorder}
                  stripData={{ "data-session-tabs": "", "aria-label": gui.session_tabs }}
                  caption={stale ? gui.backend_unreachable : !side.open ? content.error : undefined}
                  tail={<>
                    {side.open&&selectedRow&&<span data-session-content-actions className="flex flex-none items-center gap-in">
                      <GlassCircle data-content-maximize aria-label={content.maximized===selectedRow.id?gui.content_restore:gui.content_expand} onClick={()=>content.setMaximized(content.maximized===selectedRow.id?null:selectedRow.id)}><IconPaneSize expanded={content.maximized===selectedRow.id}/></GlassCircle>
                      <GlassCircle data-content-hide aria-label={gui.side_hide} onClick={()=>{content.setMaximized(null);setSessionSide(selectedRow.id,{open:false});}}><IconClose/></GlassCircle>
                    </span>}
                    <span ref={setVerbSeat} data-session-verbs className="flex flex-none items-center gap-in empty:hidden" />
                  </>}
                >
                  <div ref={sideRow} data-session-panes className={cn("flex min-h-0 min-w-0 flex-1 flex-row", sideHold !== null && "overflow-hidden")}>
                    <div ref={sideMain} data-side-held={sideHold !== null ? "" : undefined} className="flex min-h-0 min-w-0 flex-1 flex-col" style={
                      sideShown && content.maximized === selected ? { display: "none" } : sideHold !== null ? { flex: "none", width: sideHold } : sessionSide && sideBasis !== null ? { flex: `1 1 ${sideBasis}px`, minWidth: "min(var(--session-main-min), 50%)" } : undefined
                    }>
                      <Fault name="detail" reset={seamAt}>{detail}</Fault>
                    </div>
                    {sessionSide && <Fault name="side" reset={seamAt}>{sessionSide}</Fault>}
                  </div>
                </Tabs>
                </VerbSeat.Provider>
              ) : (
                <div ref={sideMain} className="flex min-h-0 min-w-0 flex-1 flex-col">
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
                    capped={landing === "devices" && selectedDevice !== null && machineBody === "dashboard"}
                    // 过期 / 离线 (第十七轮 §6): what is on the board is the
                    // last answer, and R0 says so — never a banner inside.
                    // (An open machine's model is R0's second line and its
                    // system's mark R0's avatar, both written by
                    // `MachineCard` — `PageSubtitle`, `PageAvatar`; its
                    // version and reading age are the card's, 2026-10-01.)
                    caption={stale ? gui.backend_unreachable : undefined}
                  >
                    <Fault name="detail" reset={seamAt}>
                      {detail}
                    </Fault>
                  </Region>
                </div>
              )}
            </>
          )}
          </Fault>
          </RegionLead.Provider>
          </main>
          </div>
          {dialogs}
          <StatusBar items={status.items} onDismiss={status.dismiss} />
        </div>
      )}
      <div hidden ref={el => { contentParking.current = el; if (el && !contentHost.parentElement) el.appendChild(contentHost); }} />
      <SessionContentPane container={contentHost} content={content} session={selectedRow?.id ?? null} devices={devices} rows={rows}
        mobile={narrow} navigation={contentNavigation} sessionFaces={tabFaces}
        onInspect={id=>{if(isNewTab(id)){sessionTabs.focus(id);return;}contentNavigation.inspect(id);setSessionSide(id,{open:true,seat:"workspace"});sessionTabs.focus(id);setScreen("detail");}}
        onCloseSession={id=>{if(openTabs.length===1)setScreen("list");sessionTabs.close(id);}}
        onNewSession={()=>openNewTab()}
        visible={sessions && side.open && (narrow ? inSession : sideShown) && !hidden && morePage === null && !switchingTabs}
        onClose={() => { if (selected) { content.setMaximized(null); setSessionSide(selected, { open: false }); } }}
        />
    </TooltipProvider>
    </FullScreenProvider>
    </HereHostsProvider>
    </SideSeatsProvider>
    </SettingsRereadProvider>
    </SettingsProvider>
  );
}
