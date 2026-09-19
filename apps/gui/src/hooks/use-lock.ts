// The app lock, as one piece of state the shell can ask a question of
// (docs/MORE.md 安全).
//
// # Locked is this face's, never the store's
//
// The store says **whether** these faces lock; whether *this* one is
// locked right now is the face's own, held here and nowhere else. That
// is the whole of 「网页脸同一规则，按 cookie 会话算」: a reload is a new
// face and comes up locked again, a second browser is locked on its
// own, and the app window is locked apart from both — because none of
// them is reading a shared "unlocked" bit.
//
// It also means unlocking cannot travel, which is the property that
// matters: a phone somebody handed to a friend does not become unlocked
// because the laptop is.
//
// # Three ways in, one way out
//
// Opening (the lock is on), going idle, and ⌘L. The way out is the PIN
// — or, in the app on macOS, the system's own check, which the shell
// hands in as `systemUnlock` because it is a window's door and this
// hook has no window.
import { useCallback, useEffect, useRef, useState } from "react";

import { fetchLock, unlock as askUnlock, type LockFacts } from "@/api";

/** Idle is measured off the things a person does, not off a clock that
    only a visible tab can hear: a window left on a desk fires this, and
    that is the case the whole setting exists for. */
const STIRS = ["pointerdown", "keydown", "wheel", "touchstart"] as const;

export type Lock = {
  /** `null` until the store has answered, and `null` for good on a
      store this face could not read. */
  facts: LockFacts | null;
  /** Whether the store has answered at all, either way.
   *
   * **The shell draws nothing until this is true**, and that is not
   * timidity about a flash: `locked` starts true (the safe direction —
   * a face that came up unlocked and locked a frame later has already
   * shown the thing it was meant to hide), so without this every
   * ordinary launch would flash a lock screen on a machine that has no
   * lock. One blank frame says nothing about anybody's session. */
  settled: boolean;
  /** Whether this face is locked **right now**. */
  locked: boolean;
  /** Tries a PIN. Answers whether it opened; a wrong one is `false`,
      never a throw. */
  open: (pin: string) => Promise<boolean>;
  /** The app's other key (Touch ID / the login password), already
      checked by the shell. */
  opened: () => void;
  /** 现在锁上 — ⌘L, and the button on 安全. */
  now: () => void;
  /** Re-read the store: 安全 changes these and the shell holds them. */
  reread: () => void;
};

export function useLock(): Lock {
  const [facts, setFacts] = useState<LockFacts | null>(null);
  const [settled, setSettled] = useState(false);
  // **Starts locked and is only ever let go by an answer.** The
  // alternative — start open, lock once the store says so — draws the
  // session list for one frame on a machine whose whole setting says
  // not to.
  const [locked, setLocked] = useState(true);
  /** Bumped by `reread`, so 安全 changing the setting reaches here
      without this hook polling a file that changes by hand once a year. */
  const [asked, setAsked] = useState(0);

  useEffect(() => {
    let alive = true;
    fetchLock().then(
      (f) => {
        if (!alive) return;
        setFacts(f);
        setSettled(true);
        // Only the **first** answer decides: a re-read while somebody is
        // working must not lock the face under them, and turning the
        // lock on from 安全 is a setting for next time, not an act.
        setLocked((was) => (facts === null ? f.on : was));
      },
      () => {
        // A store this face cannot read is a face with no lock to
        // enforce — and leaving it locked would be a window nothing can
        // open. `facts` stays null rather than being invented: there is
        // no ladder and no floor to offer, and 安全 says so where it
        // would have drawn them. The shell shows the ordinary backend
        // refusal on its own beat.
        if (!alive) return;
        setLocked(false);
        setSettled(true);
      },
    );
    return () => {
      alive = false;
    };
    // `facts` is read for its null-ness only, and adding it would
    // re-run this on every answer.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [asked]);

  const now = useCallback(() => {
    // **Only when there is a key to it.** ⌘L on a machine with no PIN
    // would be a window nobody can open (`khor_node::lock`'s own rule,
    // said again on the side that could otherwise bypass it).
    setFacts((f) => {
      if (f?.on) setLocked(true);
      return f;
    });
  }, []);

  // ⌘L / Ctrl+L. Captured at the window so a text box does not eat it.
  useEffect(() => {
    const key = (e: KeyboardEvent) => {
      if (e.key.toLowerCase() === "l" && (e.metaKey || e.ctrlKey) && !e.shiftKey && !e.altKey) {
        e.preventDefault();
        now();
      }
    };
    window.addEventListener("keydown", key, true);
    return () => window.removeEventListener("keydown", key, true);
  }, [now]);

  // Idle. **One timer, restarted by activity** rather than a tick that
  // compares clocks: a laptop that slept fires this on the first stir
  // after waking, which is the same moment either way, and there is no
  // beat running while nobody is there.
  const stirredAt = useRef(Date.now());
  useEffect(() => {
    const minutes = facts?.on ? facts.timeout_min : 0;
    if (!minutes) return;
    const span = minutes * 60_000;
    const stir = () => {
      stirredAt.current = Date.now();
    };
    for (const e of STIRS) window.addEventListener(e, stir, true);
    // Checked on a coarse beat rather than armed for exactly `span`:
    // a timer armed once is a timer that a sleeping machine silently
    // stretches, and what this has to answer is 「how long since a
    // person touched it」.
    const t = window.setInterval(() => {
      if (Date.now() - stirredAt.current >= span) setLocked(true);
    }, 15_000);
    return () => {
      for (const e of STIRS) window.removeEventListener(e, stir, true);
      window.clearInterval(t);
    };
  }, [facts]);

  const open = useCallback(async (pin: string) => {
    const took = await askUnlock(pin);
    if (took) {
      stirredAt.current = Date.now();
      setLocked(false);
    }
    return took;
  }, []);

  const opened = useCallback(() => {
    stirredAt.current = Date.now();
    setLocked(false);
  }, []);

  const reread = useCallback(() => setAsked((n) => n + 1), []);

  return { facts, settled, locked, open, opened, now, reread };
}
