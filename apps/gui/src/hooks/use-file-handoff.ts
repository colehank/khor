import { useEffect, useRef, useState, type RefObject } from "react";
import {
  fileHandoffSupported,
  fileHandoffStart,
  fileHandoffState,
  fileHandoffCancel,
  fileHandoffOpen,
  filePreviewSupported,
  filePreviewOpen,
  said,
} from "@/api";
import { FILE_HANDOFF_POLL_MS } from "@/lib/tempo";
import { visiblePageOwner } from "@/lib/native-page-yield";

type Progress = {
  phase: "idle" | "downloading" | "opening" | "error";
  moved: number;
  total: number;
  error: string | null;
  intent: "open" | "preview";
};
type Pending = {
  id: string;
  opening: boolean;
  timer?: ReturnType<typeof setTimeout>;
};
const idle: Progress = { phase: "idle", moved: 0, total: 0, error: null, intent: "open" };
let capability: Promise<boolean> | undefined;
let previewCapability: Promise<boolean> | undefined;

/** One original-file transfer belongs to one visible file page. A late
 * native response cannot open a file after its page has been left. */
export function useFileHandoff(machine: string, path: string, enabled = true, pageOwner?: RefObject<HTMLDivElement | null>) {
  const localOwner = useRef<HTMLDivElement>(null);
  const owner = pageOwner ?? localOwner;
  const pending = useRef<Pending | null>(null);
  const [supported, setSupported] = useState(false);
  const [previewSupported, setPreviewSupported] = useState<boolean | null>(null);
  const lastIntent = useRef<"open" | "preview">("open");
  const [progress, setProgress] = useState<Progress>(idle);

  const stop = () => {
    const job = pending.current;
    pending.current = null;
    if (job) {
      clearTimeout(job.timer);
      void fileHandoffCancel(job.id).catch(() => {});
    }
  };
  const cancel = () => {
    stop();
    setProgress(idle);
  };

  useEffect(() => {
    let alive = true;
    if (enabled) {
      (capability ??= fileHandoffSupported().catch(() => { capability = undefined; return false; })).then(value => {
        if (alive) setSupported(value);
      });
      (previewCapability ??= filePreviewSupported().catch(() => { previewCapability = undefined; return false; })).then(value => {
        if (alive) setPreviewSupported(value);
      });
    }
    return () => { alive = false; };
  }, [enabled]);

  useEffect(() => {
    setProgress(idle);
    const backgrounded = () => {
      // The OS sheet may itself make the WebView inactive. Its file
      // stays cached and its completion still belongs to this page.
      if (document.hidden && pending.current && !pending.current.opening) cancel();
    };
    document.addEventListener("visibilitychange", backgrounded);
    return () => {
      document.removeEventListener("visibilitychange", backgrounded);
      stop();
    };
  }, [machine, path]);

  const start = (intent: "open" | "preview") => {
    if (!(intent === "preview" ? previewSupported : supported) || pending.current) return;
    lastIntent.current = intent;
    const job: Pending = { id: crypto.randomUUID(), opening: false };
    pending.current = job;
    setProgress({ ...idle, phase: "downloading", intent });
    const active = () => pending.current === job && !!owner.current && visiblePageOwner(owner.current);
    const fail = (error: unknown) => {
      if (pending.current !== job) return;
      pending.current = null;
      setProgress(p => ({ ...p, phase: "error", error: said(error) }));
    };
    const accept = async (state: Awaited<ReturnType<typeof fileHandoffState>>) => {
      if (!active()) {
        if (pending.current === job) cancel();
        // Also covers cancel-before-start: start may have reached the
        // backend after the first cancel, so cancel the late job again.
        void fileHandoffCancel(job.id).catch(() => {});
        return;
      }
      if (state.error) { fail(state.error); return; }
      if (state.cancelled) { cancel(); return; }
      setProgress({
        phase: state.ready ? "opening" : "downloading",
        moved: state.moved,
        total: state.total,
        error: null,
        intent,
      });
      if (state.ready) {
        job.opening = true;
        try {
          await (intent === "preview" ? filePreviewOpen(job.id) : fileHandoffOpen(job.id));
          // Dismissing the system sheet is an ordinary end, with the
          // file, scroll position and deck height left where they were.
          if (pending.current === job) {
            pending.current = null;
            setProgress(idle);
          }
        } catch (error) { fail(error); }
      } else {
        job.timer = setTimeout(() => {
          void fileHandoffState(job.id).then(accept, fail);
        }, FILE_HANDOFF_POLL_MS);
      }
    };
    void fileHandoffStart(job.id, machine, path).then(accept, fail);
  };
  return { owner, supported, previewSupported, progress, open: () => start("open"), preview: () => start("preview"), retry: () => start(lastIntent.current), cancel };
}
export type FileHandoffControl = ReturnType<typeof useFileHandoff>;
