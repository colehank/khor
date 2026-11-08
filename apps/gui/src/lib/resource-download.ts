import {inApp} from '@/api';
import {browserDownload} from './browser-download';
import {nativeFileWriter} from './native-file-writer';
const extensions:Record<string,string>={'image/png':'png','image/jpeg':'jpg','image/webp':'webp','image/gif':'gif','image/avif':'avif','application/pdf':'pdf','audio/mpeg':'mp3','audio/mp4':'m4a','audio/wav':'wav','audio/ogg':'ogg','text/plain':'txt'};
export function downloadableResource(data?:string){return !!data&&data.length<=24*1024*1024&&data.length%4===0&&/^[A-Za-z0-9+/]*={0,2}$/.test(data);}
/** Only explicit user saving decodes inline bytes. Unknown formats are saved
 * as binary; they are never inserted into a webview as active content. */
export async function saveResource(data:string,mime:string|undefined,signal:AbortSignal){
 if(!downloadableResource(data))throw new Error('附件超过存储范围或内容不完整');
 const name=`attachment.${extensions[mime??'']??'bin'}`;
 const writer=inApp?nativeFileWriter(name,signal):browserDownload(name,mime);
 let committed=false;
 try{
  for(let from=0;from<data.length;from+=65536){
   if(signal.aborted)throw new Error('已取消存储');
   const raw=atob(data.slice(from,from+65536));
   await writer.write(Uint8Array.from(raw,char=>char.charCodeAt(0)));
  }
  if(signal.aborted)throw new Error('已取消存储');await writer.close();committed=true;
 }finally{if(!committed)await writer.abort();}
}
