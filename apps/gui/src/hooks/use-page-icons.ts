/** The main browser and session content read the same node favicon cache.
 * Live answers are keyed by their requested origin, never by the currently
 * selected tab: a slow previous page must not repaint a new destination. */
import {useCallback, useEffect, useState} from 'react';
import {fetchPageIcons} from '@/api';

export function pageOrigin(path: string): string {
  try {
    const url = new URL(path);
    return url.protocol === 'http:' || url.protocol === 'https:' ? url.origin : '';
  } catch { return ''; }
}
export function usePageIcons() {
  const [icons, setIcons] = useState<Record<string, string>>({});
  useEffect(() => {
    let live = true;
    void fetchPageIcons().then(kept => {
      if (live) setIcons(current => ({...kept, ...current}));
    }, () => {});
    return () => { live = false; };
  }, []);
  const record = useCallback((src: string, at: string) => {
    const origin = pageOrigin(at);
    if (origin && src) setIcons(current => current[origin] === src ? current : {...current, [origin]: src});
  }, []);
  return {iconOf: (path: string) => icons[pageOrigin(path)], record};
}
