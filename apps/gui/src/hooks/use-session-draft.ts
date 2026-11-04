import {useCallback,useState,type Dispatch,type SetStateAction} from 'react';
import {SessionDraftStore,type DraftStorage} from '@/lib/session-drafts';
const storage=():DraftStorage|null=>{try{return window.localStorage;}catch{return null;}};
const drafts=new SessionDraftStore(storage());
export function retainSessionDrafts(ids:readonly string[]){
  drafts.retain(ids);
}
export const checkpointSessionDrafts=():boolean=>drafts.checkpoint();
export function useSessionDraft(key:string):[string,Dispatch<SetStateAction<string>>]{
  const [state,setState]=useState(()=>({key,text:drafts.get(key)??''}));
  const text=state.key===key?state.text:drafts.get(key)??'';
  const update=useCallback<Dispatch<SetStateAction<string>>>(value=>{
    const next=typeof value==='function'?value(drafts.get(key)??''):value;
    drafts.set(key,next);
    setState({key,text:next});
  },[key]);
  return [text,update];
}
