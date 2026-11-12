/// <reference types="astro/client" />
const base=import.meta.env.BASE_URL.replace(/\/$/,'');
/** Prefix site-owned routes; fragments and external URLs keep their meaning. */
export function sitePath(path:string) {
  return path.startsWith('/')&&!path.startsWith('//')?base+path:path;
}
