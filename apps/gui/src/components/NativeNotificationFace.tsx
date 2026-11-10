import {useLayoutEffect,useRef} from "react";
import * as native from "@/api";
import type {NativeMenuState} from "@/api";
import {notificationDOM} from "@/lib/native-notification-dom";
/** Trusted owner-sanitized DOM in a private style tree. No site, navigation,
 * second app controller, or script-bearing HTML enters this renderer. */
export function NativeNotificationFace({state}:{state:NativeMenuState}) {
  const host=useRef<HTMLDivElement>(null),html=useRef(state.html??"");
  const bridge=useRef<ReturnType<typeof notificationDOM>|null>(null);
  html.current=state.html??"";
  useLayoutEffect(()=>{
    const el=host.current;if(!el)return;
    const root=el.shadowRoot??el.attachShadow({mode:"open"});
    const head=document.createElement("div"),body=document.createElement("div");
    root.replaceChildren(head,body);
    const elements=[document.documentElement,document.body],previous=elements.map(el=>el.style.background);
    elements.forEach(el=>{el.style.background="transparent";});
    const face=notificationDOM({DOMParser,ResizeObserver,getComputedStyle:window.getComputedStyle.bind(window),document:{
      documentElement:el,head,body,querySelector:root.querySelector.bind(root),
      addEventListener:root.addEventListener.bind(root),removeEventListener:root.removeEventListener.bind(root),
    }},value=>{
      if("input"in value)void native.menuInput(state.id,value.input).catch(()=>{});
      else void native.sizeMenu(state.id,value.h).catch(()=>{});
    });
    bridge.current=face;face.update(html.current);
    return()=>{face.dispose();bridge.current=null;root.replaceChildren();elements.forEach((el,index)=>{el.style.background=previous[index];});};
  },[state.id]);
  useLayoutEffect(()=>{bridge.current?.update(html.current);},[state.html]);
  return <div ref={host} data-native-notification-face className="block w-full bg-transparent"/>;
}
