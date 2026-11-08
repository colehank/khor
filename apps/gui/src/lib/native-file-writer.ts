import {fileExportBegin,fileExportAppend,fileExportFinish,fileHandoffCancel,fileHandoffSave} from '@/api';
/** Streamed staging is distinct from presentation: failure never opens an OS
 * panel with a partial file. A late begin ACK is still owned and cancelled. */
export function nativeFileWriter(name:string,signal:AbortSignal){
 const id=crypto.randomUUID();let started:Promise<unknown>|null=null,done=false,offset=0;
 let cleanup:Promise<unknown>|null=null;
 const cancel=()=>cleanup??=(async()=>{if(started){await started.catch(()=>{});await fileHandoffCancel(id);}})();
 const check=()=>{if(signal.aborted||done)throw new Error('已取消导出');};
 const start=async()=>{check();started??=fileExportBegin(id,name);await started;check();};
 const abort=()=>{void cancel().catch(()=>{});};signal.addEventListener('abort',abort,{once:true});
 return {
  async write(bytes:Uint8Array){
   await start();
   for(let from=0;from<bytes.length;from+=65536){
    check();const piece=bytes.subarray(from,from+65536);let binary='';for(const byte of piece)binary+=String.fromCharCode(byte);
    await fileExportAppend(id,offset,btoa(binary));check();offset+=piece.length;
   }
  },
  async close(){
   try{await start();await fileExportFinish(id,offset);check();const saved=await fileHandoffSave(id);check();if(!saved)throw new Error('已取消存储');done=true;}
   finally{signal.removeEventListener('abort',abort);await cancel();}
  },
  async abort(){done=true;signal.removeEventListener('abort',abort);await cancel();},
 };
}
