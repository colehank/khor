/** Page intent is separate from its responsive presentation. These defaults
 * are shared by every task dialog; callers override only real exceptions. */
export type SheetDetent = 'half' | 'full';
export type TaskPurpose = 'edit' | 'selection' | 'preview';
export type OverlayPurpose = 'confirmation' | 'actions' | TaskPurpose;
export type OverlayHeight = 'fit' | 'medium' | 'large';
export type OverlayDismiss = 'standard' | 'outside' | 'explicit';
export type OverlayOptions = { purpose: OverlayPurpose; initialHeight?: OverlayHeight; maxHeight?: OverlayHeight; dismiss?: OverlayDismiss };

export function overlayPolicy(options: OverlayOptions, narrow: boolean) {
  const task = options.purpose === 'edit' || options.purpose === 'selection' || options.purpose === 'preview';
  const initial = options.initialHeight ?? (options.purpose === 'selection' ? 'medium' : options.purpose === 'preview' ? 'large' : 'fit');
  const maximum = options.maxHeight ?? (options.purpose === 'confirmation' || options.purpose === 'edit' ? 'fit' : 'large');
  return {
    task,
    presentation: task && narrow ? 'sheet' as const : options.purpose === 'actions' && narrow ? 'actions' as const : 'dialog' as const,
    width: options.purpose === 'preview' ? 'wide' : task ? 'task' : 'compact',
    initial,
    maximum,
    resizable: task && maximum !== 'fit',
    outside: options.dismiss === 'outside' || (options.dismiss !== 'explicit' && (options.purpose !== 'edit' || narrow)),
    dismissible: options.dismiss !== 'explicit',
    expandOnInput: task && narrow && maximum === "large",
    focus: !(narrow && (options.purpose === 'selection' || options.purpose === 'preview')),
  };
}

/** Physical limits first, preferred stops second. Safe-area and keyboard
 * space belongs to the host; content/fit is never a hardcoded percentage. */
export function overlayHeights(bounds: {height: number; top?: number; foot?: number}, options: {initial: OverlayHeight; maximum: OverlayHeight; content: number; mediumShare?: number; largeShare?: number}) {
  const nonnegative = (n: number) => Number.isFinite(n) ? Math.max(0,n) : 0;
  const height=nonnegative(bounds.height), foot=nonnegative(bounds.foot ?? 0);
  const available=Math.max(0,height-foot-nonnegative(bounds.top ?? 0));
  const large=Math.min(available, Math.max(0,height*(options.largeShare ?? 1)-foot));
  const fit=Math.min(large,nonnegative(options.content));
  const medium=Math.min(large,Math.max(0,height*(options.mediumShare ?? .5)-foot));
  const sizes={fit,medium,large};
  const full=sizes[options.maximum];
  const half=Math.min(full, options.initial==='large' ? medium : sizes[options.initial]);
  return {half,full};
}
