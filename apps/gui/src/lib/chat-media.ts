import type {AcpResource} from './acp-view';
const images=new Set(['image/png','image/jpeg','image/webp','image/gif','image/avif']);
const audio=new Set(['audio/mpeg','audio/mp3','audio/wav','audio/ogg','audio/mp4','audio/webm']);
/** Inline data is already part of the trusted source read; a URI never
 * becomes an automatic remote request. SVG/HTML are not image documents. */
export function inlineMediaSource(item:AcpResource):string|null{
 const allowed=item.kind==='image'?images:item.kind==='audio'?audio:null;
 if(!allowed||!item.mime||!allowed.has(item.mime)||!item.data||item.data.length>24*1024*1024||!/^[A-Za-z0-9+/]*={0,2}$/.test(item.data)||item.data.length%4!==0)return null;
 return `data:${item.mime};base64,${item.data}`;
}
export function resourceFilePath(uri?:string):string|null{
 if(!uri)return null;
 if(uri.startsWith('/'))return uri;
 try{const url=new URL(uri);return url.protocol==='file:'&&(!url.hostname||url.hostname==='localhost')?decodeURIComponent(url.pathname):null;}catch{return null;}
}
