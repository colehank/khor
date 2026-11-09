import { useEffect, useLayoutEffect, useRef, useState } from "react";
import type { SessionRow } from "@/api";
import { SessionFace } from "@/components/SessionFace";
import { IconClose, IconSessions } from "@/components/icons";
import { gui } from "@/gen/catalog";
import type { NotificationBanner, NotificationState } from "@/lib/notification-banners";
import { configureNotifications, dismissNotification, focusNotifications, pauseNotification } from "@/lib/notifications";

export function notificationText(item: NotificationBanner): string {
  if (item.members) return gui.notification_view_sessions;
  switch (item.kind) {
    case "needs_input": return gui.notification_needs_input;
    case "turn_completed": return gui.notification_completed;
    case "failed": return gui.notification_failed;
    case "command_finished": return item.exit !== null && item.exit !== 0 ? gui.notification_failed : gui.notification_command;
    case "notice": return item.text ?? gui.notification_notice;
  }
}
const title = (item: NotificationBanner): string => item.members
  ? gui.notification_summary(String(item.members.length)) : item.title;
function Banner({item,row,onOpen}: {item:NotificationBanner;row?:SessionRow;onOpen:(item:NotificationBanner)=>boolean}) {
  const [missing,setMissing]=useState(false),[offset,setOffset]=useState(0),[closing,setClosing]=useState(false),[arrived,setArrived]=useState(false),[dragging,setDragging]=useState(false);
  const element=useRef<HTMLDivElement>(null);
  const hovered=useRef(false),focused=useRef(false),touching=useRef(false),swallow=useRef(false);
  const drag=useRef<{id:number;x:number;y:number;dy:number;moved:boolean}|null>(null);
  const pause=()=>pauseNotification(item.key,closing||hovered.current||focused.current||touching.current);
  const close=()=>{pauseNotification(item.key,true);setClosing(true);};
  useEffect(()=>{
    if(!closing)return;
    // Animation events are not guaranteed when reduced motion or window visibility changes.
    const duration=parseFloat(getComputedStyle(element.current!).animationDuration)*1000||0;
    const timer=setTimeout(()=>dismissNotification(item.key),duration+100);
    return()=>clearTimeout(timer);
  },[closing,item.key]);
  const release=(cancelled:boolean)=>{
    const current=drag.current;if(!current)return;
    drag.current=null;touching.current=false;setDragging(false);swallow.current=current.moved||cancelled;
    const height=element.current?.getBoundingClientRect().height??0;
    if(!cancelled&&current.dy < -height*.45){close();return;}
    setOffset(0);pause();
  };
  return <div ref={element} data-notification-banner={item.key} data-notification-closing={closing||undefined}
    data-notification-arrived={arrived||undefined} data-notification-dragging={dragging||undefined}
    style={offset?{transform:`translateY(${offset}px)`,opacity:Math.max(.3,1-Math.abs(offset)/(element.current?.offsetHeight??1))}:undefined}
    className="notification-banner glass flex min-w-0 items-start gap-in rounded-2xl p-row"
    onPointerEnter={event=>{if(event.pointerType==="mouse"){hovered.current=true;pause();}}}
    onPointerLeave={event=>{if(event.pointerType==="mouse"){hovered.current=false;pause();}}}
    onFocusCapture={()=>{focused.current=true;pause();}} onBlurCapture={event=>{if(!event.currentTarget.contains(event.relatedTarget)){focused.current=false;pause();}}}
    onPointerDown={event=>{
      if(!event.isPrimary||event.button!==0||closing)return;
      swallow.current=false;touching.current=true;setDragging(true);setArrived(true);
      drag.current={id:event.pointerId,x:event.clientX,y:event.clientY,dy:0,moved:false};pause();
    }}
    onPointerMove={event=>{
      const current=drag.current;if(!current||current.id!==event.pointerId)return;
      const dx=event.clientX-current.x,dy=event.clientY-current.y;
      if(Math.hypot(dx,dy)>6)current.moved=true;
      if(!current.moved||Math.abs(dy)<Math.abs(dx))return;
      current.dy=dy;event.preventDefault();
      event.currentTarget.setPointerCapture(event.pointerId);
      setOffset(dy<0?dy:Math.sqrt(dy)*2);
    }}
    onPointerUp={event=>{if(drag.current?.id===event.pointerId)release(false);}}
    onPointerCancel={()=>release(true)} onLostPointerCapture={event=>{if(event.target===event.currentTarget)release(true);}}
    onClickCapture={event=>{if(swallow.current&&event.detail!==0){event.preventDefault();event.stopPropagation();}}}
    onKeyDown={event=>{if(event.key==="Escape"){event.preventDefault();event.stopPropagation();close();}}}
    onAnimationEnd={event=>{if(event.target!==event.currentTarget)return;if(closing)dismissNotification(item.key);else setArrived(true);}}>
    <button type="button" className="flex min-h-touch min-w-0 flex-1 items-center gap-row rounded-xl text-start focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring" data-notification-open
      onClick={()=>{if(closing)return;if(onOpen(item))close();else setMissing(true);}}>
      {row&&!item.members?<SessionFace face={row.face} word={row.word} kind={row.kind} category={row.category} size="row"/>:
        <span className="grid size-list-face flex-none place-items-center rounded-full bg-fill text-muted-foreground"><IconSessions className="size-kind-mark"/></span>}
      <span className="flex min-w-0 flex-1 flex-col gap-xxs">
        <span className="flex min-w-0 items-baseline gap-in">
          <span className="min-w-0 flex-1 truncate text-body font-title">{title(item)}</span>
          {!item.members&&item.preview?<span className="flex-none text-caption text-muted-foreground">{notificationText(item)}</span>:null}
        </span>
        <span className="line-clamp-2 break-words text-callout text-muted-foreground">{missing?gui.notification_missing:item.members?notificationText(item):item.preview??notificationText(item)}</span>
      </span>
    </button>
    <button type="button" data-notification-dismiss aria-label={gui.close} className="group flex min-h-touch min-w-touch flex-none items-center justify-center rounded-full focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-ring"
      onClick={close}><span aria-hidden className="grid size-ctl-sm place-items-center rounded-full bg-fill text-muted-foreground group-hover:bg-fill-strong group-hover:text-foreground"><IconClose className="size-affordance"/></span></button>
  </div>;
}
/** App chrome only: no saved list, no focus grab, and no banner while locked. */
export function NotificationBanners({state,sessions,onOpen,narrow}: {state:NotificationState;sessions:SessionRow[];onOpen:(item:NotificationBanner)=>boolean;narrow:boolean}) {
  useLayoutEffect(()=>configureNotifications(narrow?1:3),[narrow]);
  useEffect(()=>{
    const blur=()=>focusNotifications(false),focus=()=>focusNotifications(!document.hidden);
    const visibility=()=>{if(!document.hidden)focusNotifications(document.hasFocus());};
    focusNotifications(!document.hidden&&document.hasFocus());
    window.addEventListener("blur",blur);window.addEventListener("focus",focus);
    document.addEventListener("visibilitychange",visibility);
    return()=>{window.removeEventListener("blur",blur);window.removeEventListener("focus",focus);document.removeEventListener("visibilitychange",visibility);};
  },[]);
  const latest=state.items.at(-1);
  return <aside data-notification-banners role="region" aria-label={gui.notifications} className="notification-banners pointer-events-none fixed z-40 flex flex-col gap-in">
    <span className="sr-only" role="status" aria-live="polite" aria-atomic="true">{latest?`${title(latest)} · ${notificationText(latest)}${latest.members||!latest.preview?"":` · ${latest.preview}`}`:""}</span>
    {state.items.map(item=>{
      const row=sessions.find(row=>row.id===item.session&&row.execution_home===item.home);
      return row||item.members?<Banner key={item.key} item={item} row={row} onOpen={onOpen}/>:null;
    })}
  </aside>;
}
