/** A native API is available only in a native host. Developer entry points
 * may replace this transport boundary while retaining the production UI. */
export function hasNativeBackend(): boolean {
  return '__TAURI_INTERNALS__' in window;
}

export const hasAppUpdates = hasNativeBackend;
