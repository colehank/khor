/** Fallback for browsers without File System Access. Never publish a partial
 * download: retain bounded bytes and create the link only after commit. */
export function browserDownload(name:string,type='application/octet-stream',limit=64*1024*1024){
 let chunks:Uint8Array<ArrayBuffer>[]=[];let bytes=0,done=false;
 return {
  async write(value:Uint8Array){
   if(done)throw new Error('download already finished');
   if(bytes+value.byteLength>limit)throw new Error('此浏览器的单次导出超过64 MiB，请使用桌面应用');
   chunks.push(Uint8Array.from(value));bytes+=value.byteLength;
  },
  async close(){
   if(done)throw new Error('download already finished');done=true;
   const blob=new Blob(chunks,{type});chunks=[];
   const url=URL.createObjectURL(blob),link=document.createElement('a');link.href=url;link.download=name;
   try{document.body.appendChild(link);link.click();}finally{link.remove();setTimeout(()=>URL.revokeObjectURL(url),30000);}
  },
  async abort(){done=true;chunks=[];bytes=0;},
 };
}
