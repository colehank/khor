import { useEffect, useLayoutEffect, useRef, useState } from "react";

import { newShellRun, recoverSession, sessionRecovery, type RecoveryResult, type SessionRecovery, type ShellRunResult } from "@/api";
import { hearRecoveryIntent, readRecoveryIntent, RecoveryIntentError, rememberRecoveryIntent, settleRecoveryIntent, type RecoveryIntent } from "@/lib/session-recovery-intent";

type Outcome = RecoveryResult | ShellRunResult;
export type RecoveryOpened = { id: string; previous: string | null };
export type RecoveryProblem = "storage" | "invalid" | "occupied" | "capacity" | "unavailable";
const problemOf = (error: unknown) => error instanceof RecoveryIntentError ? error.code : null;

/** The mounted surface owns only delivery. The durable caller payload for this target
    outlives a view; Node remains the sole execution/attempt state owner. */
export function useSessionRecovery({ id, home, revision, onOpened }: {
  id: string;
  home: string;
  revision: string;
  onOpened: (opened: RecoveryOpened) => void;
}) {
  const [offer, setOffer] = useState<{ revision: string; value: SessionRecovery } | null>(null);
  const [intent, setIntent] = useState<RecoveryIntent | null>(null);
  const [intentReady, setIntentReady] = useState(false);
  const [outcome, setOutcome] = useState<Outcome | null>(null);
  const [fault, setFault] = useState<string | null>(null);
  const [problem, setProblem] = useState<RecoveryProblem | null>(null);
  const [busy, setBusy] = useState(false);
  const owner = useRef({ alive: false, reading: 0, intentReading: 0, busy: false, generation: 0, revision });
  owner.current.revision = revision;
  const opened = useRef(onOpened);
  opened.current = onOpened;
  useLayoutEffect(() => {
    const live = owner.current;
    live.alive = true; live.generation++; live.busy = false; setBusy(false);
    return () => { live.alive = false; live.busy = false; live.generation++; live.reading++; live.intentReading++; };
  }, []);

  const loadIntent = async () => {
    const live = owner.current, reading = ++live.intentReading;
    try {
      const value = await readRecoveryIntent(home, id);
      if (live.alive && reading === live.intentReading) { setIntent(value); setIntentReady(true); }
    } catch (error) {
      if (live.alive && reading === live.intentReading) { setIntentReady(false); setProblem(problemOf(error) ?? "storage"); }
    }
  };
  const refresh = async (): Promise<SessionRecovery | null> => {
    const live = owner.current, reading = ++live.reading, requested = revision;
    try {
      const answer = await sessionRecovery(home, id);
      if (!live.alive || reading !== live.reading || requested !== live.revision) return null;
      if (answer.id !== id || answer.home !== home) throw new Error("recovery offer changed its execution identity");
      setOffer({ revision: requested, value: answer });
      setFault(null);
      return answer;
    } catch (error) {
      if (live.alive && reading === live.reading && requested === live.revision) { setOffer(null); setFault(String(error)); }
      return null;
    }
  };
  // A lifecycle change invalidates an older offer. This is the read-only offer,
  // never Initialize, Load, Resume or an automatic retry of a caller intent.
  useEffect(() => { void refresh(); }, [id, home, revision]);
  useEffect(() => {
    void loadIntent();
    return hearRecoveryIntent(() => { void loadIntent(); });
  }, []);

  const act = async (cwd: string | null = null) => {
    const live = owner.current, generation = live.generation;
    if (live.busy || !live.alive || !intentReady) return;
    live.busy = true;
    setBusy(true); setFault(null); setProblem(null);
    try {
      let payload = await readRecoveryIntent(home, id);
      if (!live.alive || generation !== live.generation) return;
      if (payload) {
        if (payload.id !== id || payload.home !== home || (cwd !== null && payload.cwd !== cwd)) throw new RecoveryIntentError("occupied");
      } else {
        const current = await refresh();
        if (!current || !live.alive || generation !== live.generation) return;
        if (!current.next_attempt_epoch || current.execution === "offline" || current.action === "none") {
          setProblem("unavailable"); return;
        }
        const operation = current.action === "new_shell" ? "new_shell" : "native";
        if (cwd !== null && operation !== "new_shell") { setProblem("unavailable"); return; }
        const nonce = Array.from(crypto.getRandomValues(new Uint8Array(16)), n => n.toString(16).padStart(2, "0")).join("");
        payload = { home, id, operation, expected_run: operation === "new_shell" ? id : current.current_run,
          token: { epoch: current.next_attempt_epoch, nonce }, cwd };
      }
      // Native readwrite transaction serializes this target's slot. If it
      // cannot commit, nothing is dispatched. A stale surface never dispatches.
      await rememberRecoveryIntent(payload);
      if (!live.alive || generation !== live.generation) return;
      setIntent(payload);
      live.reading++; // no pre-action offer may overwrite the result's run
      const result: Outcome = payload.operation === "new_shell"
        ? await newShellRun(home, id, payload.expected_run, payload.token, payload.cwd)
        : await recoverSession(home, id, payload.expected_run, payload.token);
      // Pending/Unknown (and transport errors below) retain the exact payload.
      // Clear a final reply even after view retirement, but never navigate then.
      // Row status is not action ownership: this recovery itself may change
      // ended/can_control before the reply arrives. Target+mount own delivery;
      // expected_run and the actual result remain Node's execution authority.
      if (result.kind !== "pending" && result.kind !== "unknown") await settleRecoveryIntent(payload);
      if (!live.alive || generation !== live.generation) return;
      setOutcome(result);
      await loadIntent();
      if (!live.alive || generation !== live.generation) return;
      if (result.kind === "opened") opened.current({ id: result.id, previous: "previous" in result ? result.previous : null });
      else if (result.kind === "join") opened.current({ id: result.id, previous: null });
      void refresh();
    } catch (error) {
      if (live.alive && generation === live.generation) { setProblem(problemOf(error)); setFault(problemOf(error) ? null : String(error)); void loadIntent(); }
    } finally {
      if (generation === live.generation) { live.busy = false; if (live.alive) setBusy(false); }
    }
  };
  return { offer: offer?.revision === revision ? offer.value : null, intent, intentReady, outcome, fault, problem, busy, refresh: async () => { await Promise.all([refresh(), loadIntent()]); }, act };
}
