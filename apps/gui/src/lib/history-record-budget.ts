/** Shared display admission, separate from the small metadata/page cache.
 * At most two records decode at once; retained UTF16 bodies have one 64MiB
 * owner budget across mounted conversations and full-turn readers. */
let active=0,retained=0;
const waiting=new Set<()=>void>();
export async function admitHistoryRecord(signal:AbortSignal){
 while(active>=2){
  await new Promise<void>((resolve,reject)=>{
   const wake=()=>{waiting.delete(wake);signal.removeEventListener('abort',abort);resolve();};
   const abort=()=>{waiting.delete(wake);reject(new Error('已取消读取'));};
   if(signal.aborted){abort();return;}waiting.add(wake);signal.addEventListener('abort',abort,{once:true});
  });
 }
 if(signal.aborted)throw new Error('已取消读取');active++;
 let released=false;return ()=>{if(released)return;released=true;active--;for(const wake of waiting)wake();};
}
export function retainHistoryRecord(value:unknown){
 let bytes=0;
 const size=(part:unknown,depth=0)=>{
  if(depth>128)throw new Error('内容层级过深');
  if(typeof part==='string')bytes+=part.length*2;
  else if(Array.isArray(part))for(const child of part)size(child,depth+1);
  else if(part&&typeof part==='object')for(const child of Object.values(part))size(child,depth+1);
 };
 size(value);
 if(retained+bytes>64*1024*1024)throw new Error('附件较多，滚动后可继续读取');
 retained+=bytes;let released=false;return()=>{if(!released){released=true;retained-=bytes;}};
}
