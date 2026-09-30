// A machine's VS Code, in a window of its own — **one flow, two doors**
// (2026-09-30, the owner: 「机器页也加一个 VS Code 图标，打开这台机器的
// VS Code，不指定目录」). The open workspace's switch opens that machine's
// editor on the workspace's directory; the machine page's opens it with no
// folder (`khor_gui_core::code::open` with an empty `dir`). Everything
// else is the same flow, and it lives here once so the two cannot drift:
// drawn only where that machine has an editor and never on the web face,
// the first press on a machine asks (the consent dialog), a second press
// brings the window forward rather than making another, the switch goes
// dark when the window does, and the far server is given back when the
// door stops pointing at that machine.
//
// Moved whole out of `views/WorkspacePane` (工作区批5, rulings 45–57); the
// reasoning in the comments below is that file's, kept where the code is.
//
// **The state is the caller's to hold, the button is anybody's to draw.**
// `useCodeWindow` is called where the door lives for as long as it points
// at a machine — the workspace pane, the shell for the machine page — and
// not inside the button: the button moves between R0 and the narrow head
// when the window crosses the breakpoint, and a remount there would drop
// the window's handle and stop the server under it.
import { useEffect, useRef, useState, type ReactNode } from "react";

import { codeSeat, fetchCodeHere, hear, onWebFace, openCode, stopCode, type CodeSeat } from "@/api";
import { IconCode } from "@/components/icons";
import { Button } from "@/components/ui/button";
import { Dialog, DialogContent, DialogFooter, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { Tooltip, TooltipContent, TooltipTrigger } from "@/components/ui/tooltip";
import type { Editor } from "@/gen/bindings/Editor";
import { gui } from "@/gen/catalog";
import { useHidden } from "@/hooks/use-hidden";
import { basename } from "@/words";

/** How often the web face asks its editor tab whether it is still there
    (2026-09-18 ruling 57). **A second, because the answer is drawn
    beside a pointer**: nothing a person can see improves by asking
    faster, and anything slower leaves a lit switch over a window that
    is gone — which is the whole of what this ruling is about. The cost
    is one property read of a handle this page already holds. */
const CODE_BEAT_MS = 1_000;

/** What the editor's window is called: the project, then the product.
    **The workspace first**, because a person with two of them open is
    reading window titles to tell them apart, and 「VS Code」 is the
    half both of them share. With no directory (the machine page's
    door) it is the machine. */
function windowTitle(dir: string, machine: string): string {
  return `${dir === "" ? machine : basename(dir)} — ${gui.code_tab}`;
}

export type CodeWindow = {
  /** Whether to draw the switch at all: that machine has an editor
      (`code --version` answered) and this is not the web face. */
  offered: boolean;
  /** Whether the window is open, as far as this face knows. */
  up: boolean;
  /** The press. */
  want: () => void;
  /** The consent and the refusal, as a dialog — mount it once. */
  dialog: ReactNode;
};

/**
 * `machine` is a node name (`null` where the door points at nothing);
 * `dir` is the folder to open, empty for none.
 */
export function useCodeWindow(machine: string | null, dir: string): CodeWindow {
  const hidden = useHidden();
  /** What that machine says about its editor. `version` empty means it
      has none **or could not be asked** — the two lead to the same next
      move, which is to draw no switch (`khor_gui_core::code::here`). */
  const [code, setCode] = useState<Editor>({ version: "", ready: false });
  /** The URL the window was pointed at, once one has been asked for. */
  const [codeAt, setCodeAt] = useState<string | null>(null);
  /** The consent card is up. Asked once per machine and remembered in
      this face's own storage — **the thing consented to happens on the
      far machine, so it is keyed by that machine**, and a second face
      asking again is right rather than a bug: the person at that face
      has not agreed to anything yet. */
  const [asking, setAsking] = useState(false);
  const [codeError, setCodeError] = useState<string | null>(null);
  /** The window this machine's editor is in, once one has been asked
      for (2026-09-17 ruling 46). **A ref, because nothing on the glass
      is drawn from it** — what the switch paints is `up` — and a
      window handle in state would be a render per press for a thing
      only the next press reads. */
  const seat = useRef<CodeSeat | null>(null);
  /** Whether that window is open, as far as this face knows.
   *
   * **`codeAt` is not this question.** That one is 「the node answered
   * a URL」, which stays true after somebody closes the window; this is
   * 「there is a window」, which is what the switch is lit for. */
  const [up, setUp] = useState(false);

  // **Given back when this door stops pointing at that machine.** A web
  // face is a `node` process with an extension host on somebody else's
  // box; leaving used to leave it running with nothing on any screen
  // pointed at it, which is the shape `held.ts` was written for on the
  // borrow side. Here the ledger is one entry deep — one door, one
  // machine — so the cleanup is the effect's own.
  useEffect(() => {
    if (codeAt === null || machine === null) return;
    return () => {
      stopCode(machine).catch(() => {
        // The pipe on this side is already dropped by the node; a far
        // machine that could not be told is a server that outlives this
        // by one `khor code --stop`, not a hang.
      });
    };
  }, [machine, codeAt]);

  // **The switch goes dark when the window does** (2026-09-17 ruling
  // 46:「窗口关掉 = 图标灭」). The app says so out loud — it is the one
  // that owns the window — and the web face's named tab is asked
  // instead, on the beat somebody comes back to this page, because a
  // tab showing another origin tells this page nothing and `closed` is
  // the one thing it will still answer.
  useEffect(() => hear<string>("code:closed", (who) => who === machine && setUp(false)), [machine]);
  useEffect(() => {
    if (hidden) return;
    if (seat.current?.gone() === true) setUp(false);
  }, [hidden]);
  // …and a beat while the tab is held (2026-09-18 ruling 57:「网页脸上
  // VS Code 标签页关了, 图标当场灭」). **Only while there is something to
  // ask, and only on the road that answers**: the app announces (`asks`
  // is false there), the question stops the moment it comes back true,
  // and a door with no seat sets no timer at all.
  useEffect(() => {
    if (!up) return;
    const held = seat.current;
    if (held === null || !held.asks) return;
    const beat = window.setInterval(() => {
      if (held.gone()) setUp(false);
    }, CODE_BEAT_MS);
    return () => window.clearInterval(beat);
  }, [up]);

  useEffect(() => {
    let live = true;
    setCode({ version: "", ready: false });
    setCodeAt(null);
    setAsking(false);
    setCodeError(null);
    // A door's editor window belongs to the machine it was opened on;
    // walking to another machine leaves that window where it is and this
    // door stops holding it (the far server is stopped by the effect
    // above, which is where it always was).
    seat.current?.drop();
    seat.current = null;
    setUp(false);
    if (machine === null) return;
    fetchCodeHere(machine)
      .then((e) => live && setCode(e))
      // Already folded to the default on the way here; this arm is for
      // the call itself failing, and it means the same thing.
      .catch(() => live && setCode({ version: "", ready: false }));
    return () => {
      live = false;
    };
  }, [machine]);

  /** Remembered per machine, in this face's own storage. */
  const agreedKey = `khor.code.agreed.${machine ?? ""}`;
  const agreed = (): boolean => {
    try {
      return window.localStorage.getItem(agreedKey) === "1";
    } catch {
      // Storage refused (a private window, a locked profile). **Then
      // nobody has agreed** — the safe answer is to ask again, not to
      // assume the yes we cannot read.
      return false;
    }
  };
  const go = () => {
    if (machine === null) return;
    setAsking(false);
    setCodeError(null);
    try {
      window.localStorage.setItem(agreedKey, "1");
    } catch {
      // The launch still happens; only the memory of the yes is lost,
      // and the cost of that is being asked again.
    }
    // **Claimed inside the press, before anything is awaited** — a
    // browser only lets a page open a window from the task a person's
    // own gesture started, and the editor's address is two round trips
    // away (`api.ts` `codeSeat`).
    const seated = codeSeat(machine);
    seat.current = seated;
    openCode(machine, dir)
      .then((a) => {
        setCodeAt(a.url);
        setUp(true);
        return seated.raise(a.url, windowTitle(dir, machine));
      })
      .catch((e: unknown) => {
        // A window with nothing in it is worse than none: it is khor
        // having opened something and said nothing.
        seated.drop();
        seat.current = null;
        setUp(false);
        setCodeError(String(e instanceof Error ? e.message : e));
      });
  };
  /** Pressing the switch: ask first if this machine has never been
      agreed to, otherwise go straight there.
      **The server already being over there is the other way past the
      card**, and it is a stronger reason than the remembered yes: the
      card's whole sentence is「第一次要下约 640 MB」, and on a machine
      that has the server that sentence is false (2026-09-13). */
  const want = () => {
    if (machine === null) return;
    setCodeError(null);
    // **Already open is brought forward** (2026-09-17 ruling 46:「再按
    // 图标 = 把那个窗口拉到前面(已开就不再开第二个)」). The switch is
    // one press with two meanings, and which one it has is a fact about
    // the window rather than a mode: there is one, or there is not.
    const held = seat.current;
    if (held !== null && up && codeAt !== null) {
      void held.raise(codeAt, windowTitle(dir, machine));
      return;
    }
    if (code.ready || agreed()) {
      go();
    } else {
      setAsking(true);
    }
  };

  // ── the editor's two sentences, as a dialog ─────────────────────
  // (2026-09-17 ruling 46:「同意卡…和『那台机器没有 code』的拒绝: 在工作区
  // 页里弹一张小 sheet」— the user; a centred dialog since 2026-09-30,
  // when the R0 sheet was retired.) **Both belong to one press and
  // neither belongs in a column** — they stood in the editor's own column
  // for one batch (ruling 45), and a person's file gave up room to hold a
  // question about a window that did not exist yet. A dialog asks and
  // goes. The words are the ones already written: two sentences about
  // what happens to somebody else's machine, and the far machine's own
  // refusal.
  const dialog = (
    <Dialog
      open={asking || codeError !== null}
      onOpenChange={(o) => {
        if (o) return;
        setAsking(false);
        setCodeError(null);
      }}
    >
      <DialogContent data-code-dialog>
        <DialogHeader>
          <DialogTitle>{gui.code_tab}</DialogTitle>
        </DialogHeader>
        {asking ? (
          <>
            <div data-code-consent className="max-w-prose text-body whitespace-pre-line text-foreground">
              {gui.code_consent}
            </div>
            <DialogFooter>
              <Button variant="ghost" size="sm" data-code-no onClick={() => setAsking(false)} className="min-h-touch">
                {gui.code_consent_no}
              </Button>
              <Button variant="primary" size="sm" data-code-go onClick={go} className="min-h-touch">
                {gui.code_consent_go}
              </Button>
            </DialogFooter>
          </>
        ) : (
          <div data-code-refused className="text-body text-muted-foreground">
            {codeError}
          </div>
        )}
      </DialogContent>
    </Dialog>
  );

  return { offered: code.version !== "" && !onWebFace && machine !== null, up, want, dialog };
}

/**
 * The switch itself, as a toolbar button (components/Toolbar): VS Code's
 * mark in a `ctl-md` box, `--hit` around it, a `Tooltip` naming it, the
 * grey block while its window is open. **Not a toggle** — pressing it
 * again brings the window forward rather than closing it — so it says no
 * `aria-pressed`. Draws nothing where the door is not `offered`.
 *
 * **Equal ink, not equal boxes** (2026-09-16:「右上角的icon, 明显大小
 * 不一」, then 「注意四个icon的大小一致性」). The mark fills its canvas and
 * the rail's glyphs beside it keep air inside their own (about five
 * sevenths of it), so their 24 paints about 17 and this box is 16: the
 * *painted* marks match within the gate's pixel and a half, which
 * `check-face` measures — never the class.
 */
export function CodeButton({ code, className, mark }: { code: CodeWindow; className?: string; mark?: Record<string, string> }) {
  if (!code.offered) return null;
  return (
    <Tooltip>
      <TooltipTrigger asChild>
        <Button
          variant="ghost"
          size="icon"
          {...mark}
          data-code={code.up ? "on" : "off"}
          data-on={code.up}
          aria-label={gui.code_tab}
          onClick={code.want}
          className={className}
        >
          <span className="flex size-4 flex-none [&_svg]:size-full">
            <IconCode />
          </span>
        </Button>
      </TooltipTrigger>
      <TooltipContent side="bottom">{gui.code_tab}</TooltipContent>
    </Tooltip>
  );
}
