/** System document UI owns the screen until its dismissal promise settles. */
let count = 0;
export const nativeFilePreviewActive = () => count > 0;
export function holdNativeFilePreview(): () => void {
  count++; window.dispatchEvent(new Event("khor:native-file-preview"));
  let held = true;
  return () => { if (!held) return; held = false; count--; window.dispatchEvent(new Event("khor:native-file-preview")); };
}
