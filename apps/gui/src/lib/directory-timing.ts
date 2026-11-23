import { faceNote, inApp } from "@/api";

// Enabled for diagnostic builds. Keep only the most recent navigation and
// report durations without retaining paths or device names in the log.
const enabled = import.meta.env.VITE_KHOR_DIRECTORY_TIMING === "1";

interface Navigation {
  machine: string;
  path: string;
  clicked: number;
  submitted?: number;
  replied?: number;
}

let pending: Navigation | undefined;

export function beginDirectoryTiming(machine: string, path: string) {
  if (enabled) pending = { machine, path, clicked: performance.now() };
}

export function pendingDirectoryTiming(machine: string) {
  return pending?.machine === machine ? pending : undefined;
}

export function startDirectoryRequest(machine: string, path: string) {
  const navigation = pendingDirectoryTiming(machine);
  if (navigation?.path !== path || navigation.submitted !== undefined) return;
  navigation.submitted = performance.now();
  return () => { navigation.replied = performance.now(); };
}

export function reportDirectoryTiming(navigation: Navigation, outcome: "visible" | "refused") {
  if (pending !== navigation) return;
  pending = undefined;
  const now = performance.now();
  const { clicked, submitted, replied } = navigation;
  const result = {
    at: Date.now(),
    outcome,
    clickToEndMs: Math.round(now - clicked),
    clickToSubmitMs: submitted === undefined ? null : Math.round(submitted - clicked),
    submitToReplyMs: submitted === undefined || replied === undefined ? null : Math.round(replied - submitted),
    replyToEndMs: replied === undefined ? null : Math.round(now - replied),
  };
  const line = "directory timing " + JSON.stringify(result);
  if (inApp) void faceNote(line).catch(() => {});
  else console.info(line);
}
