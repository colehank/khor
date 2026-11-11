/** Frontend fixtures remain HTTP-only even when viewed inside Khor's own
 * browser. Do not delete, replace or invoke the containing app's runtime. */
let mode: 'frontend' | 'full' | null = null;

export function configureLabBackend(next: 'frontend' | 'full'): void {
  if (mode !== null && mode !== next) throw new Error('Lab backend mode cannot change after startup');
  mode = next;
}

export function hasNativeBackend(): boolean {
  if (mode === null) throw new Error('Configure the Lab backend before entering the App');
  return mode === 'full' && '__TAURI_INTERNALS__' in window;
}

/** Exercise the real updater UI through fixture HTTP, never the host App. */
export function hasAppUpdates(): boolean {
  if (mode === null) throw new Error('Configure the Lab backend before entering the App');
  return mode === 'frontend' || hasNativeBackend();
}
