/** Shared filesystem-object verbs for trees, directory lists and search hits.
 * Management verbs are supplied by the owner; file contents have their own menu. */
import {useEffect,useState} from 'react';
import {fetchRevealHere,reveal} from '@/api';
import type {RowAction} from '@/components/RowMenu';
import {gui} from '@/gen/catalog';

export function fileManagerLabel() {
  const platform=navigator.platform;
  return /Mac/i.test(platform)?gui.chat_file_reveal:/Win/i.test(platform)?gui.file_reveal_windows:gui.file_reveal_linux;
}

export function relativeFilePath(root:string,path:string):string {
  if (!root) return path;
  const windows=/^[a-z]:[\\/]|^\\\\/i.test(root);
  const clean=(value:string)=>windows?value.replaceAll('\\','/'):value;
  const base=clean(root).replace(/\/+$/,''),full=clean(path);
  const a=windows?base.toLowerCase():base,b=windows?full.toLowerCase():full;
  if(a===b)return '.';
  return b.startsWith(a+'/')?full.slice(base.length+1):path;
}

/** null means outside the object, including similarly prefixed sibling names. */
export function movedFilePath(path:string,from:string,to:string):string|null {
  const relative=relativeFilePath(from,path);
  if(relative===path)return null;
  return relative==='.'?to:to.replace(/[\\/]+$/,'')+'/'+relative;
}

export function useNativeFileManager() {
  const [available,setAvailable]=useState(false);
  useEffect(()=>{let live=true;fetchRevealHere().then(value=>{if(live)setAvailable(value)},()=>{});return()=>{live=false}},[]);
  return available;
}

export function useFileObjectActions(here:boolean,root:string) {
  const native=useNativeFileManager();
  const [failure,setFailure]=useState<{id:string;note?:string}|null>(null);
  const attempt=async(id:string,run:()=>Promise<unknown>,copy=false)=>{
    try { await run(); setFailure(null); }
    catch(error) { setFailure({id,note:copy?undefined:String(error instanceof Error?error.message:error)}); throw error; }
  };
  return (path:string):RowAction[]=>{
    const copy=(key:string,label:string,text:string):RowAction=>({key,label:failure?.id===path+key?gui.copy_failed:label,failed:failure?.id===path+key||undefined,run:()=>attempt(path+key,()=>navigator.clipboard.writeText(text),true)});
    return [
      copy('copy-path',gui.tab_copy_path,path),
      copy('copy-rel',gui.tab_copy_rel,relativeFilePath(root,path)),
      ...(native&&here?[{key:'reveal',label:fileManagerLabel(),failed:failure?.id===path+'reveal'||undefined,note:failure?.id===path+'reveal'?failure.note:undefined,run:()=>attempt(path+'reveal',()=>reveal(path))}]:[]),
    ];
  };
}
