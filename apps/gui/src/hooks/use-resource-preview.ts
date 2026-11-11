import {useEffect,useState} from 'react';
import {fileResourceOpen,fileResourceClose,fileResourceTouch,inApp,said,type ResourcePreview} from '@/api';

/** One visible preview owns one resource origin, including late openings. */
export function useResourcePreview(machine:string,path:string,html?:string) {
  const key=JSON.stringify([machine,path,html]);
  const [state,setState]=useState<{key:string;preview?:ResourcePreview;error?:string}>({key});
  const [retry,setRetry]=useState(0);
  useEffect(()=>{
    let live=true,owned:ResourcePreview|undefined;
    setState({key});
    const release=(preview:ResourcePreview)=>void fileResourceClose(preview.id).catch(()=>{});
    const timer=setTimeout(()=>void fileResourceOpen(machine,path,html).then(preview=>{
      if(!live){release(preview);return;}
      owned=preview;
      const url=new URL(preview.url);
      // A LAN web face's backend is on that face's host, not this browser's
      // loopback. Native previews always use their own backend's loopback.
      if(!inApp && /^https?:$/.test(location.protocol))url.hostname=location.hostname;
      setState({key,preview:{...preview,url:url.href}});
    },error=>{if(live)setState({key,error:said(error)});}),html===undefined?0:300);
    const pulse=()=>{if(owned)void fileResourceTouch(owned.id).catch(error=>{if(live){setState({key,error:said(error)});}});};
    const heartbeat=setInterval(pulse,30_000);
    document.addEventListener('visibilitychange',pulse);
    return()=>{live=false;clearTimeout(timer);clearInterval(heartbeat);document.removeEventListener('visibilitychange',pulse);if(owned)release(owned);};
  },[machine,path,html,key,retry]);
  return {preview:state.key===key?state.preview:undefined,error:state.key===key?state.error:undefined,retry:()=>setRetry(n=>n+1)};
}
