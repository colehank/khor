import type {DirListing} from '@/gen/bindings/DirListing';
import {directoryPrefix,homeRelative,parentPath,settledPath} from '@/lib/path-input';

type DirectoryIO={list:(path:string)=>Promise<DirListing>;mkdir:(path:string)=>Promise<unknown>};
export type SessionDirectory = {path:string;missing:string[]} & (
  {state:'ready'|'missing';error?:never}|{state:'unavailable';error:unknown}
);
const absolute=(path:string)=>path.startsWith('/')||/^[a-z]:[\\/]/i.test(path)||path.startsWith('\\\\');
const leaf=(path:string)=>path.split(/[\\/]/).at(-1)!;
const cancelled=(alive:()=>boolean)=>{if(!alive())throw new DOMException('Cancelled','AbortError');};

/** A missing name is proven by a successful, complete parent listing.
 * Listing/permission/network errors are never interpreted as nonexistence. */
export async function inspectSessionDirectory(typed:string,io:DirectoryIO,alive=()=>true):Promise<SessionDirectory>{
  let path=settledPath(typed);
  try{
    if(path==='~'||path.startsWith('~/')||path.startsWith('~\\')){
      const home=await io.list('');cancelled(alive);
      path=path==='~'?home.path:homeRelative(home.path,path.slice(2));
    }
    if(!absolute(path))throw new Error('An absolute directory is required');
    const live=await io.list(path);cancelled(alive);
    return {state:'ready',path:live.path,missing:[]};
  }catch(error){
    cancelled(alive);
    if(!absolute(path)||path.split(/[\\/]/).some(part=>part==='.'||part==='..'))return {state:'unavailable',path,missing:[],error};
    let child=path;
    const missing:string[]=[];
    for(let depth=0;depth<64;depth++){
      const parent=parentPath(child);
      if(parent===null)break;
      missing.unshift(child);
      let listing:DirListing;
      try{listing=await io.list(parent);cancelled(alive);}catch{cancelled(alive);child=parent;continue;}
      // A present but unreadable descendant is not a candidate for mkdir.
      if(listing.truncated||listing.entries.some(entry=>entry.name.toLocaleLowerCase()===leaf(child).toLocaleLowerCase()))break;
      const suffix=missing.map(leaf);
      const resolved=suffix.reduce((parent,name)=>directoryPrefix(parent)+name,listing.path);
      return {state:'missing',path:resolved,missing:suffix.map((_,index)=>suffix.slice(0,index+1).reduce((parent,name)=>directoryPrefix(parent)+name,listing.path))};
    }
    return {state:'unavailable',path,missing:[],error};
  }
}

/** Called only by Create. Re-check each mkdir refusal to accept a concurrent
 * directory creator, while preserving real permissions/file/network errors. */
export async function ensureSessionDirectory(typed:string,io:DirectoryIO,alive=()=>true):Promise<string>{
  const facts=await inspectSessionDirectory(typed,io,alive);cancelled(alive);
  if(facts.state==='unavailable')throw facts.error;
  for(const path of facts.missing){
    cancelled(alive);
    try{await io.mkdir(path);}catch(error){
      try{await io.list(path);}catch{throw error;}
    }
    cancelled(alive);
  }
  return facts.path;
}
