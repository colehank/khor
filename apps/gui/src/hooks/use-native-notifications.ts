import {useEffect,type RefObject} from "react";
import {hear,hideMenu,inApp,showMenu} from "@/api";
import {nativeMenuSnapshot} from "@/lib/native-menu-snapshot";
import {nativeFilePreviewActive} from "@/lib/native-file-preview";
import {overlapsNativePage} from "@/lib/native-page-yield";
import type {NotificationInput} from "@/lib/native-notification-dom";

/** One owner-rendered face above a retained native page. The native host
 * mirrors its inert DOM; all input returns to the original React handlers. */
export function useNativeNotifications(root:RefObject<HTMLElement|null>) {
  useEffect(()=>{
    const el=root.current;if(!inApp||!el)return;
    const id=`notice:${crypto.randomUUID()}`;
    let alive=true,frame=0,last="",held=false,fallback=false;
    const hide=()=>{
      last="";el.removeAttribute("data-notification-native");el.removeAttribute("aria-hidden");el.removeAttribute("inert");
      if(held){
        held=false;
        for(const banner of el.querySelectorAll('[data-notification-banner]')){
          banner.dispatchEvent(new PointerEvent("pointerout",{bubbles:true,pointerType:"mouse"}));
          banner.dispatchEvent(new PointerEvent("pointercancel",{bubbles:true}));
          banner.dispatchEvent(new FocusEvent("focusout",{bubbles:true}));
        }
        void hideMenu(id).catch(()=>{});
      }
    };
    const blocked=()=>nativeFilePreviewActive()||document.hidden||!document.hasFocus()||!!document.querySelector('[data-omni-menu],[data-slot="dialog-overlay"],[data-scrim],[aria-modal="true"],[role="menu"],[role="listbox"]');
    const schedule=()=>{if(alive&&!frame)frame=requestAnimationFrame(update);};
    const update=()=>{
      frame=0;if(!alive)return;
      const keys=[...el.querySelectorAll<HTMLElement>('[data-notification-banner]')].map(node=>node.dataset.notificationBanner!);
      if(!keys.length||blocked()){fallback=false;el.removeAttribute("data-notification-yield");hide();return;}
      if(fallback)return;
      if(!overlapsNativePage(el)){hide();return;}
      const rect=el.getBoundingClientRect(),html=nativeMenuSnapshot(el);
      const geometry={x:rect.x,y:rect.y,w:rect.width,view:window.innerHeight,across:window.innerWidth,above:false};
      const fingerprint=JSON.stringify([html,geometry]);
      if(fingerprint!==last){
        last=fingerprint;held=true;
        void showMenu({id,at:geometry,html,rows:{notification:true,groups:[{key:"notifications",label:"",candidates:keys.map(key=>({key,label:key}))}],at:0}}).then(accepted=>{
          if(!alive||!held||last!==fingerprint)return;
          if(accepted){el.setAttribute("data-notification-native","");el.setAttribute("aria-hidden","true");el.setAttribute("inert","");}
          else {fallback=true;hide();el.setAttribute("data-notification-yield","");}
        },()=>{if(alive&&held&&last===fingerprint){fallback=true;hide();el.setAttribute("data-notification-yield","");}});
      }

    };
    const stop=hear<{id:string;event:NotificationInput}>("menu:input",message=>{
      if(message.id!==id||!held||blocked())return;
      const input=message.event;
      const banner=[...el.querySelectorAll<HTMLElement>('[data-notification-banner]')].find(node=>node.dataset.notificationBanner===input.key);
      if(!banner)return;
      const rect=el.getBoundingClientRect();
      const target=input.type==="click"?banner.querySelector(input.action==="dismiss"?'[data-notification-dismiss]':'[data-notification-open]'):banner;
      if(!target)return;
      if(input.type.startsWith("pointer"))target.dispatchEvent(new PointerEvent(input.type,{bubbles:true,cancelable:true,clientX:input.x+rect.x,clientY:input.y+rect.y,pointerId:input.pointerId,pointerType:input.pointerType,button:input.button,isPrimary:input.isPrimary}));
      else if(input.type==="click")target.dispatchEvent(new MouseEvent("click",{bubbles:true,cancelable:true,detail:input.detail}));
      else if(input.type==="keydown")target.dispatchEvent(new KeyboardEvent("keydown",{key:"Escape",bubbles:true,cancelable:true}));
      else target.dispatchEvent(new FocusEvent(input.type,{bubbles:true}));
      schedule();
    });
    const observer=new MutationObserver(records=>{if(records.some(record=>!(record.target instanceof Element)||!record.target.closest('[data-menu-native]')))schedule();});
    observer.observe(document.body,{attributes:true,childList:true,subtree:true,characterData:true});
    const size=new ResizeObserver(schedule);size.observe(el);
    for(const event of ["resize","focus","blur","khor:native-file-preview"])window.addEventListener(event,schedule);
    document.addEventListener("visibilitychange",schedule);schedule();
    return()=>{alive=false;cancelAnimationFrame(frame);stop();observer.disconnect();size.disconnect();hide();el.removeAttribute("data-notification-yield");for(const event of ["resize","focus","blur","khor:native-file-preview"])window.removeEventListener(event,schedule);document.removeEventListener("visibilitychange",schedule);};
  },[root]);
}
