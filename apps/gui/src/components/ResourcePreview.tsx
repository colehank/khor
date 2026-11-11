import {useEffect,useRef} from 'react';
import {inApp} from '@/api';
import {gui} from '@/gen/catalog';
import {useResourcePreview} from '@/hooks/use-resource-preview';
import {BrowserPane} from '@/views/BrowserPane';
import {releasePages} from '@/views/held';
import {Button} from '@/components/ui/button';

const ignore=()=>{};
export function ResourcePreview({machine,path,html,media}:{machine:string;path:string;html?:string;media?:'video'|'audio'}) {
  const state=useResourcePreview(machine,path,html);
  const element=useRef<HTMLMediaElement|null>(null);
  const id=state.preview?.id;
  const native=media===undefined&&inApp&&id!==undefined;
  useEffect(()=>{
    if(!id)return;
    const player=element.current;
    return()=>{
      if(player){player.pause();player.removeAttribute('src');player.load();}
      if(native)releasePages([],`file-preview-${id}`);
    };
  },[id,native]);
  return <div data-resource-preview={media??'html'} className="relative flex min-h-0 min-w-0 flex-1 flex-col">
    {state.error?<div role="alert" className="flex flex-col items-center justify-center gap-3 p-4"><p className="break-words text-body">{state.error}</p><Button onClick={state.retry}>{gui.region_retry}</Button></div>:state.preview?
      media==='video'?<video ref={element} src={state.preview.url} controls playsInline preload="metadata" className="min-h-0 w-full flex-1 object-contain" onError={()=>{element.current?.setAttribute('data-media-failed','');}}/>:
      media==='audio'?<div className="flex min-h-0 flex-1 items-center justify-center p-4"><audio ref={element} src={state.preview.url} controls preload="metadata" className="w-full"/></div>:
      native?<BrowserPane tab={`file-preview-${id}`} retentionOwner={`file-preview-${id}`} machine="" device="" here url={state.preview.url} onOpened={ignore} onWent={ignore} onPopped={ignore} onTitle={ignore}/>:
      <iframe data-html-frame src={state.preview.url} sandbox="allow-scripts allow-same-origin allow-forms allow-modals allow-popups" title={path.split('/').pop()??path} referrerPolicy="no-referrer" className="min-h-0 w-full flex-1 border-0"/>
      :<div role="status" aria-label={gui.file_resource_loading} className="flex-1"/>}
  </div>;
}
