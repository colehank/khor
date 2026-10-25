/** Ephemeral text drafts for the layout experiment. Nothing goes to storage
 * or to a host; reloading the whole Lab clears these drafts. */
import {useCallback,useState,type Dispatch,type SetStateAction} from 'react';
const drafts=new Map<string,string>();
export function useLabSessionDraft(key:string):[string,Dispatch<SetStateAction<string>>]{
  const [state,setState]=useState(()=>({key,text:drafts.get(key)??''}));
  const text=state.key===key?state.text:drafts.get(key)??'';
  const update=useCallback<Dispatch<SetStateAction<string>>>(value=>{
    const next=typeof value==='function'?value(drafts.get(key)??''):value;
    if(next)drafts.set(key,next);else drafts.delete(key);
    setState({key,text:next});
    // Keep the comparison frame's cache current after sends and tab changes.
    if(window.parent!==window)window.parent.postMessage({type:'khor.layout.draft',session:key.slice(key.indexOf(':')+1),text:next},location.origin);
  },[key]);
  return [text,update];
}
