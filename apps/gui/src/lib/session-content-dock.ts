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
  readonly current=()=>this.seat?.read()??null;
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
