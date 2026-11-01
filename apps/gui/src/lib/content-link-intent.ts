export type LinkIntent = 'internal'|'system';
export type LinkModifiers = {metaKey?:boolean;ctrlKey?:boolean};
export const linkIntent = (event:LinkModifiers):LinkIntent => event.metaKey||event.ctrlKey?'system':'internal';

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

/** xterm's linkifier also activates on a mouse-up inside a selection.
 * Keep selection and application mouse input separate from link intent. */
export function canActivateTerminalLink(event:LinkModifiers&{button?:number},term:{hasSelection:()=>boolean;modes:{mouseTrackingMode:string}}):boolean {
  if(event.button!==undefined&&event.button!==0||term.hasSelection())return false;
  return linkIntent(event)==='system'||term.modes.mouseTrackingMode==='none';
}
