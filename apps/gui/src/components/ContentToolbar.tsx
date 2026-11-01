/** One desktop content toolbar, retained across picker, file and webpage. */
import {useCallback,type ReactNode} from 'react';
import {ContentScope} from '@/components/ContentScope';
import {GlassCircle} from '@/components/Glass';
import {IconBack,IconClose} from '@/components/icons';
import {gui} from '@/gen/catalog';
import type {ContentSource} from '@/hooks/use-content-navigation';
import {band} from '@/lib/scroll-edge';

export function ContentToolbar({kind,source,onSource,children,actions,onClose,onBack,pinned=false}:{
  kind:'picker'|'file'|'page';source:ContentSource;onSource:(source:ContentSource)=>void;
  children:ReactNode;actions?:ReactNode;onClose:()=>void;onBack?:()=>void;pinned?:boolean;
}) {
  const bind=useCallback((head:HTMLDivElement|null)=>{
    const locateLanding=kind==='page'?()=>head?.parentElement?.querySelector<HTMLElement>(':scope > [data-content-page-landing]')??null:undefined;
    return band(head,locateLanding);
  },[kind]);
  return <div ref={bind} data-band data-content-toolbar={kind} data-content-page-bar={kind==='page'||undefined} className="session-content-location-bar">
    <ContentScope value={source} onChange={onSource} segmented/>
    {onBack&&<GlassCircle data-content-back aria-label={gui.back} tipSide="top" onClick={onBack}><IconBack/></GlassCircle>}
    <div data-content-location-layer className="glass content-location-pill">{children}</div>
    {actions}
    <GlassCircle data-content-close disabled={pinned} aria-label={gui.close_tab} tipSide="top" onClick={onClose}><IconClose/></GlassCircle>
  </div>;
}
