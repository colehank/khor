import {useLayoutEffect} from 'react';
export const LAB_RECENTS='khor.lab.session-recents';
export function readLabRecents():string[]{
  try{const value=JSON.parse(sessionStorage.getItem(LAB_RECENTS)??'[]');return Array.isArray(value)?[...new Set(value.filter((id):id is string=>typeof id==='string'))].slice(0,32):[];}catch{return [];}
}
/** All App routes record the actual active tab; presentation never does. */
export function useLabSessionVisits(active:string|null){
  useLayoutEffect(()=>{if(!active||active.startsWith('new-tab:'))return;
    try{sessionStorage.setItem(LAB_RECENTS,JSON.stringify([active,...readLabRecents().filter(id=>id!==active)].slice(0,32)));}catch{/* preview can still navigate with storage disabled */}
  },[active]);
}
