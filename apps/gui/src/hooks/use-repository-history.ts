/** Both workspace seats keep the same repository-history return route. */
import {useEffect,useState} from 'react';
import type {GitCommit} from '@/gen/bindings/GitCommit';
export function useRepositoryHistory(machine:string,root:string) {
  const[historyOpen,setHistoryOpen]=useState(false);
  const[commit,setCommit]=useState<GitCommit|null>(null);
  useEffect(()=>{setCommit(null);setHistoryOpen(false);},[machine,root]);
  const backToHistory=()=>{setCommit(null);setHistoryOpen(true);};
  return {historyOpen,setHistoryOpen,commit,setCommit,backToHistory};
}
