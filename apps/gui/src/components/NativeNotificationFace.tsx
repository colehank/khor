import {useCallback,useEffect,useLayoutEffect,useRef} from "react";
import * as native from "@/api";
import type {NativeMenuState} from "@/api";
import {gui} from "@/gen/catalog";
import {notificationDOM} from "@/lib/native-notification-dom";
/** Scriptless frozen document: no site, app controller, or IPC in the frame. */
export function NativeNotificationFace({state}:{state:NativeMenuState}) {
  const frame=useRef<HTMLIFrameElement>(null),html=useRef(state.html??"");
  const bridge=useRef<ReturnType<typeof notificationDOM>|null>(null);
  html.current=state.html??"";
  useLayoutEffect(()=>{
    const elements=[document.documentElement,document.body];
    const previous=elements.map(el=>el.style.background);
    elements.forEach(el=>{el.style.background="transparent";});
    return()=>elements.forEach((el,index)=>{el.style.background=previous[index];});
  },[]);
  const mount=useCallback(()=>{
    const scope=frame.current?.contentWindow;if(!scope)return;
    bridge.current?.dispose();
    bridge.current=notificationDOM(scope,value=>{
      if("input"in value)void native.menuInput(state.id,value.input).catch(()=>{});
      else {if(frame.current)frame.current.style.height=`${value.h}px`;void native.sizeMenu(state.id,value.h).catch(()=>{});}
    });
    bridge.current.update(html.current);
  },[state.id]);
  useEffect(()=>{bridge.current?.update(html.current);},[state.html]);
  useEffect(()=>{
    // StrictMode can retire effects after an early srcDoc load. Reattach
    // to the already loaded document as well as listening for a fresh load.
    if(frame.current?.contentDocument?.readyState==="complete")mount();
    return()=>{bridge.current?.dispose();bridge.current=null;};
  },[mount]);
  return <iframe ref={frame} data-native-notification-face title={gui.notifications} sandbox="allow-same-origin" className="block w-full border-0 bg-transparent" srcDoc={`<!doctype html><html><head><meta http-equiv="Content-Security-Policy" content="default-src 'none';style-src 'unsafe-inline';img-src data:;script-src 'none'"></head><body></body></html>`}
    onLoad={mount}/>;
}
