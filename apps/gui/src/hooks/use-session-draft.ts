import {useCallback,useState,type Dispatch,type SetStateAction} from 'react';
const drafts=new Map<string,string>();
export function retainSessionDrafts(ids:readonly string[]){
  const open=new Set(ids);for(const key of drafts.keys())if(!open.has(key.slice(key.indexOf(':')+1)))drafts.delete(key);
}
export function useSessionDraft(key:string):[string,Dispatch<SetStateAction<string>>]{
  const [state,setState]=useState(()=>({key,text:drafts.get(key)??''}));
  const text=state.key===key?state.text:drafts.get(key)??'';
  const update=useCallback<Dispatch<SetStateAction<string>>>(value=>{
    const next=typeof value==='function'?value(drafts.get(key)??''):value;
    if(next)drafts.set(key,next);else drafts.delete(key);
    setState({key,text:next});
  },[key]);
  return [text,update];
}
