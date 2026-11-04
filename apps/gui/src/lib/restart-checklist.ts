import type { UpdateBlocker } from "@/gen/bindings/UpdateBlocker";

export class RestartChecklist {
  private checks = new Map<symbol, () => UpdateBlocker | null>();

  register(check: () => UpdateBlocker | null): () => void {
    const owner = Symbol();
    this.checks.set(owner, check);
    return () => { this.checks.delete(owner); };
  }

  prepare(): UpdateBlocker[] {
    const blocked = new Set<UpdateBlocker>();
    for (const check of this.checks.values()) {
      try { const reason = check(); if (reason) blocked.add(reason); }
      catch { blocked.add("draft-storage"); }
    }
    return [...blocked];
  }
}
