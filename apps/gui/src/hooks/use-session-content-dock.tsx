/** One retained content tree moves between session hosts. A late release
 * from an old column cannot steal the current column's editors/webviews. */
import {useCallback,useLayoutEffect,useMemo,useRef,useState,useSyncExternalStore} from 'react';
import type {TabFace} from '@/browse/TabStrip';

export type SessionContentScope={
  session:string;mobile:boolean;faces:TabFace[];
  onSelectSession:(id:string)=>void;onCloseSession:(id:string)=>void;
  onNewSession:()=>void;onClose:()=>void;
};
type Seat={token:object;identity:string;read:()=>SessionContentScope};

export class SessionContentDock {
  readonly host:HTMLDivElement;
  private parking:HTMLElement|null=null;
  private seat:Seat|null=null;
  private listeners=new Set<()=>void>();
  constructor(){this.host=document.createElement('div');this.host.className='session-content-host';}
  readonly subscribe=(notify:()=>void)=>{this.listeners.add(notify);return()=>{this.listeners.delete(notify);};};
  readonly snapshot=()=>this.seat;
  parkAt(element:HTMLElement|null){this.parking=element;if(!this.seat&&element)this.move(element);}
  present(token:object,element:HTMLElement,identity:string,read:()=>SessionContentScope){
    this.move(element);
    if(this.seat?.token===token&&this.seat.identity===identity)return;
    this.seat={token,identity,read};this.changed();
  }
  release(token:object){
    if(this.seat?.token!==token)return;
    this.seat=null;if(this.parking)this.move(this.parking);this.changed();
  }
  private move(element:HTMLElement){if(this.host.parentElement!==element)element.appendChild(this.host);}
  private changed(){for(const notify of this.listeners)notify();}
}

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
