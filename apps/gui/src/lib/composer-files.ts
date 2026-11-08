/** A session owns its unsent files, including work still in flight.
 * Views subscribe; leaving a view does not cancel or reassign its uploads. */
export type PendingFile={key:string;name:string;size:number|null;thumb:string|null;inline:[string,string]|null;path:string|null;error:string|null;encoding?:boolean};
export class ComposerFiles {
  private items:PendingFile[]=[];
  private listeners=new Set<()=>void>();
  private sequence=0;
  private retired=false;
  private revoke:(url:string)=>void;
  constructor(revoke:(url:string)=>void=url=>URL.revokeObjectURL(url)){this.revoke=revoke;}
  readonly snapshot=()=>this.items;
  readonly subscribe=(listener:()=>void)=>{this.listeners.add(listener);return()=>{this.listeners.delete(listener);};};
  key(){return `attachment:${++this.sequence}`;}
  add(items:PendingFile[]){if(this.retired){for(const item of items)if(item.thumb)this.revoke(item.thumb);return;}this.update([...this.items,...items]);}
  patch(key:string,patch:Partial<PendingFile>){
    if(!this.items.some(item=>item.key===key))return;
    this.update(this.items.map(item=>item.key===key?{...item,...patch}:item));
  }
  remove(key:string){const item=this.items.find(item=>item.key===key);if(item?.thumb)this.revoke(item.thumb);this.update(this.items.filter(item=>item.key!==key));}
  clear(){for(const item of this.items)if(item.thumb)this.revoke(item.thumb);this.update([]);}
  retire(){this.retired=true;this.clear();}
  private update(items:PendingFile[]){this.items=items;for(const listener of this.listeners)listener();}
}
const owners=new Map<string,ComposerFiles>();
export function composerFiles(session:string){let files=owners.get(session);if(!files){files=new ComposerFiles();owners.set(session,files);}return files;}
export function discardComposerFiles(session:string){owners.get(session)?.retire();owners.delete(session);}
export function composerReady(items:readonly PendingFile[]){return items.every(item=>item.path!==null&&!item.encoding&&!item.error);}

export function composerFilesAtRisk(){return [...owners.values()].some(files=>files.snapshot().length>0);}

export function retainComposerFiles(sessions:readonly string[]){
 const live=new Set(sessions);
 for(const [owner,files] of owners)if(!live.has(owner)&&!(owner.startsWith('khor:')&&live.has(owner.slice(5)))){files.retire();owners.delete(owner);}
}
