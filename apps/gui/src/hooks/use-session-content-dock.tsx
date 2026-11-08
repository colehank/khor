/** One retained content tree moves between session hosts. A late release
 * from an old column cannot steal the current column's editors/webviews. */
import {useCallback,useLayoutEffect,useMemo,useRef,useState,useSyncExternalStore} from 'react';

import {SessionContentDock,type SessionContentScope} from '@/lib/session-content-dock';
export {SessionContentDock,type SessionContentScope} from '@/lib/session-content-dock';

export function useSessionContentDock(){
  const [dock]=useState(()=>new SessionContentDock());
  const seat=useSyncExternalStore(dock.subscribe,dock.snapshot);
  return {dock,scope:seat?.read()??null};
}

export function SessionContentSeat({dock,scope}:{dock:SessionContentDock;scope:SessionContentScope}){
  const token=useRef<object>({}),node=useRef<HTMLDivElement|null>(null),latest=useRef(scope);
  latest.current=scope;
  // Functions live in latest; only actual presentation facts request a render.
  const identity=JSON.stringify([scope.session,scope.mobile,scope.faces.map(face=>({
    id:face.id,label:face.label,sub:face.sub,path:face.path,face:face.face,
    session:face.session,loading:face.loading,share:face.share,rang:face.rang,
  }))]);
  const read=useCallback(()=>latest.current,[]);
  const attach=useCallback((element:HTMLDivElement|null)=>{
    node.current=element;if(!element)dock.release(token.current);
  },[dock]);
  const seat=useMemo(()=>({identity,read}),[identity,read]);
  useLayoutEffect(()=>{if(node.current)dock.present(token.current,node.current,seat.identity,seat.read);},[dock,seat]);
  useLayoutEffect(()=>()=>dock.release(token.current),[dock]);
  return <div ref={attach} className="flex min-h-0 min-w-0 flex-1 flex-col"/>;
}
