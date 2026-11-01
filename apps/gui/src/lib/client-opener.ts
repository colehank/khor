import {inApp,launchOn,openLink} from '@/api';
import {msg} from '@/gen/catalog';
import {fileUri} from '@/lib/content-link-intent';

export function assertPageUrl(url:string):void {
  if(!/^https?:\/\//i.test(url))throw new Error(msg.not_a_page(url));
}

export async function openSystemUrl(url:string):Promise<void> {
  assertPageUrl(url);
  if(inApp){await openLink(url);return;}
  window.open(url,'_blank','noopener');
}

export async function openSystemPath(path:string):Promise<void> {
  if(inApp){await launchOn('',path);return;}
  // Browser file access may be refused by that client. Never route a
  // local handoff to the web server's operating-system opener instead.
  window.open(fileUri(path),'_blank','noopener');
}

/** The system receives the original URI, including its authority and drive. */
export async function openSystemFileUri(uri:string):Promise<void> {
  if(new URL(uri).protocol!=='file:')throw new Error(msg.not_a_file_path(uri));
  if(inApp){await openLink(uri);return;}
  window.open(uri,'_blank','noopener');
}
