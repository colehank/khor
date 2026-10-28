import {inApp,launchOn,openLink} from '@/api';

export type LinkModifiers = {metaKey?:boolean;ctrlKey?:boolean};
export const linkIntent = (event:LinkModifiers):'internal'|'system' => event.metaKey||event.ctrlKey?'system':'internal';

/** Represent the same filesystem target for a browser's local file handoff. */
export function fileUri(path:string):string {
  const windows=/^[a-z]:[\\/]/i.test(path)||path.startsWith('\\\\');
  const normalized=windows?path.replaceAll('\\','/'):path;
  const uri=new URL('file:///');
  let pathname=normalized;
  if(normalized.startsWith('//')){
    const slash=normalized.indexOf('/',2);
    uri.hostname=slash<0?normalized.slice(2):normalized.slice(2,slash);
    pathname=slash<0?'/':normalized.slice(slash);
  }else if(/^[a-z]:\//i.test(normalized))pathname='/'+normalized;
  uri.pathname=pathname.split('/').map(encodeURIComponent).join('/').replace(/^\/([a-z])%3A\//i,'/$1:/');
  return uri.href;
}

export async function openSystemUrl(url:string):Promise<void> {
  if(inApp){await openLink(url);return;}
  window.open(url,'_blank','noopener');
}

export async function openSystemPath(path:string):Promise<void> {
  if(inApp){await launchOn('',path);return;}
  // Browser file access may be refused by that client. Never route a
  // local handoff to the web server's operating-system opener instead.
  window.open(fileUri(path),'_blank','noopener');
}
