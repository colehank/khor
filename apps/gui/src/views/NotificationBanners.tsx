import { useRef, useState } from "react";
import type { SessionRow } from "@/api";
import { SessionFace } from "@/components/SessionFace";
import { IconClose } from "@/components/icons";
import { gui } from "@/gen/catalog";
import type { NotificationBanner, NotificationState } from "@/lib/notification-banners";
import { dismissNotification, pauseNotification } from "@/lib/notifications";

export function notificationText(item: NotificationBanner): string {
  switch (item.kind) {
    case "needs_input": return gui.notification_needs_input;
    case "turn_completed": return gui.notification_completed;
    case "failed": return gui.notification_failed;
    case "command_finished": return item.exit !== null && item.exit !== 0 ? gui.notification_failed : gui.notification_command;
    case "notice": return item.text ?? gui.notification_notice;
  }
}
function Banner({item,row,onOpen}: {item:NotificationBanner;row:SessionRow;onOpen:(item:NotificationBanner)=>boolean}) {
  const [missing,setMissing]=useState(false);
  const hovered=useRef(false),focused=useRef(false);
  const pause=()=>pauseNotification(item.key,hovered.current||focused.current);
  return <div data-notification-banner={item.key} className="notification-banner glass flex min-w-0 items-center gap-in rounded-lg p-in shadow-(--shadow-float)"
    onMouseEnter={()=>{hovered.current=true;pause();}} onMouseLeave={()=>{hovered.current=false;pause();}}
    onFocusCapture={()=>{focused.current=true;pause();}} onBlurCapture={event=>{if(!event.currentTarget.contains(event.relatedTarget)){focused.current=false;pause();}}}>
    <button type="button" className="flex min-h-touch min-w-0 flex-1 items-center gap-in text-start" data-notification-open
      onClick={()=>{if(onOpen(item))dismissNotification(item.key);else setMissing(true);}}>
      <SessionFace face={row.face} word={row.word} kind={row.kind} category={row.category} size="tab"/>
      <span className="flex min-w-0 flex-1 flex-col">
        <span className="truncate text-body font-label">{row.title}</span>
        <span className="text-callout text-muted-foreground">{missing?gui.notification_missing:notificationText(item)}</span>
      </span>
    </button>
    <button type="button" data-notification-dismiss aria-label={gui.close} className="flex min-h-touch min-w-touch items-center justify-center rounded-full"
      onClick={()=>dismissNotification(item.key)}><IconClose className="size-affordance"/></button>
  </div>;
}
/** App chrome only: no saved list, no focus grab, and no banner while locked. */
export function NotificationBanners({state,sessions,onOpen,narrow}: {state:NotificationState;sessions:SessionRow[];onOpen:(item:NotificationBanner)=>boolean;narrow:boolean}) {
  return <aside data-notification-banners role="region" aria-label={gui.notifications} className="notification-banners pointer-events-none fixed z-40 flex flex-col gap-in">
    <span className="sr-only" role="status" aria-live="polite" aria-atomic="true">{state.items.at(-1) ? `${state.items.at(-1)!.title} · ${notificationText(state.items.at(-1)!)}` : ""}</span>
    {(narrow?state.items.slice(-1):state.items).map(item=>{
      const row=sessions.find(row=>row.id===item.session&&row.execution_home===item.home);
      return row?<Banner key={item.key} item={item} row={row} onOpen={onOpen}/>:null;
    })}
  </aside>;
}
