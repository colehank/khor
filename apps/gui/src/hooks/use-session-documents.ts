import {useSyncExternalStore,type Dispatch,type SetStateAction} from 'react';
type Documents=[string,string][];
const owners=new Map<string,{value:Documents;listeners:Set<()=>void>}>();
/** Documents share the composer's session lifetime, not the mounted textarea. */
export function useSessionDocuments(session:string,initial:()=>Documents):[Documents,Dispatch<SetStateAction<Documents>>]{
 let owner=owners.get(session);
 if(!owner){owner={value:initial(),listeners:new Set()};owners.set(session,owner);}
 const entry=owner;
 const value=useSyncExternalStore(listener=>{entry.listeners.add(listener);return()=>{entry.listeners.delete(listener);};},()=>entry.value);
 return [value,next=>{entry.value=typeof next==='function'?next(entry.value):next;for(const notify of entry.listeners)notify();}];
}
