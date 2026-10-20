/** Desktop session content. The session's terminal stays in its own pane;
 * these are its files and pages, using the same node tab handles throughout. */
import { useEffect, useLayoutEffect, useRef, useState, Fragment, type ComponentProps } from "react";
import { createPortal } from "react-dom";
import { recentVisit, type DeviceRow, type SessionRow } from "@/api";
import { TabStrip, TabDivider, type TabFace } from "@/browse/TabStrip";
import { gui } from "@/gen/catalog";
import { NarrowBox } from "@/hooks/use-narrow";
import { type ContentTab, type SessionContent } from "@/hooks/use-session-content";
import { BrowserPane } from "@/views/BrowserPane";
import { FilePage } from "@/views/FilePage";
import type { Draft } from "@/views/FileView";
import { releaseBorrows, releasePages } from "@/views/held";
import { useNewAct, useTabHost } from "@/lib/shortcuts";
import { sessionCwdOnScreen } from "@/lib/session-cwd";
import { cn } from "@/lib/utils";
import "@/styles/session-content.css";

import { contentView, contentName, contentTabTargets } from '@/lib/content';
export { contentView, contentName } from '@/lib/content';
import { contentPickerPosition, type ContentNavigation } from '@/hooks/use-content-navigation';
import { contentTabFace, ContentPicker, ContentOverview, ContentSessionOverview, contentFileDeck, useContentNeighbors } from '@/views/MobileSessionContent';
import { MobileContentPage } from '@/views/MobileContentPage';
const quiet = (work: Promise<unknown>) => { void work.catch(() => {}); };

/** Only the children scroll. The group head and the add button keep their
 * seats; entering/leaving the group fades the row without shrinking the outer capsule. */
export function ContentTabs({session,open,devices,content,navigation,onAdd,onFocus,onHide}:{session:string;open:boolean;devices:DeviceRow[];content:SessionContent;navigation:ContentNavigation;onAdd:()=>void;onFocus:()=>void;onHide:()=>void}) {
  const tabs=content.allTabs.filter(t=>t.session===session),state=navigation.get(session);
  const targets=contentTabTargets(tabs,content.handle);
  const faces=[...targets].map(([key,tab])=>{
    const picker=contentView(tab)==='workspace';
    return {...contentTabFace(tab,devices,content.pageIcon),id:key,...(picker?{label:gui.content_new}:{}),pinned:tab.pinned,
      attributes:{'data-content-tab':tab.id,'data-content-on':state.key===key,'data-content-browse':picker||undefined,'data-content-icon':!picker},
      menu:[{key:'pin',label:tab.pinned?gui.unpin:gui.pin,run:()=>content.pin(tab.id,!tab.pinned)},{key:'close',label:gui.close_tab,disabled:tab.pinned,run:()=>content.close(tab.id)}]};
  });
  const pick=(key:string)=>{const tab=targets.get(key);if(tab){onFocus();navigation.select(tab);quiet(content.focus(tab.id));}};
  const close=(key:string)=>{const tab=targets.get(key);if(!tab)return;
    if(state.key===key&&tabs.length===1){navigation.dismiss(session);onHide();}quiet(content.close(tab.id));};
  return <div data-content-tabs={session} data-open={open} aria-hidden={!open} inert={!open} className="session-content-reveal" style={{width:open?`calc(${tabs.length} * var(--tab-w-even) + 5 * var(--gap-in) + 3 * var(--hair) + var(--glass-tab))`:0}}>
    <TabDivider/><TabStrip nested grouped faces={faces} active={state.key||''} onFocus={pick} onClose={close} onNew={onAdd} newLabel={gui.content_new} onReorder={ids=>content.reorder(session,ids)}/>
  </div>;
}

/** Stable scroller names survive remounts and the wide/narrow portal move.
 * Native webpage views retain their own history/scroll under the tab handle. */
const CONTENT_SCROLLERS=['[data-file-preview]','[data-file-scroll]','.cm-scroller','[data-side-workspace] > div','[data-new-tab]','[data-files-list]','.mobile-content-overview-list'];
function ContentPosition({session,position,show,mobile,navigation,children,...props}: ComponentProps<'section'> & {
  session:string;position:string;show:boolean;mobile:boolean;navigation:ContentNavigation;
}) {
  const root=useRef<HTMLElement>(null),latest=useRef(navigation);latest.current=navigation;
  useLayoutEffect(()=>{
    const box=root.current;if(!show||!box)return;
    const done=new WeakSet<HTMLElement>();let stopped=false;
    const name=(selector:string,index:number)=>position+' '+selector+' '+index;
    const restore=()=>{
      if(stopped)return;
      for(const selector of CONTENT_SCROLLERS)[...box.querySelectorAll<HTMLElement>(selector)].forEach((el,index)=>{
        if(done.has(el)||!el.getClientRects().length)return;
        const saved=latest.current.scrollOf(session,name(selector,index));
        if(!saved){done.add(el);return;}
        if(saved.top>0&&el.scrollHeight<=el.clientHeight||saved.left>0&&el.scrollWidth<=el.clientWidth)return;
        el.scrollTop=saved.top;el.scrollLeft=saved.left;done.add(el);
      });
    };
    const capture=(event:Event)=>{
      const el=event.target;if(!(el instanceof HTMLElement)||!box.contains(el)||!el.getClientRects().length)return;
      const selector=CONTENT_SCROLLERS.find(selector=>el.matches(selector));if(!selector)return;
      const index=[...box.querySelectorAll(selector)].indexOf(el);
      latest.current.scroll(session,name(selector,index),{top:el.scrollTop,left:el.scrollLeft});
    };
    const interact=()=>{stopped=true;};
    const observer=new MutationObserver(restore);observer.observe(box,{childList:true,subtree:true});
    const resize=new ResizeObserver(restore);resize.observe(box);
    restore();box.addEventListener('scroll',capture,true);box.addEventListener('wheel',interact,{passive:true});box.addEventListener('pointerdown',interact,{passive:true});
    return()=>{observer.disconnect();resize.disconnect();box.removeEventListener('scroll',capture,true);box.removeEventListener('wheel',interact);box.removeEventListener('pointerdown',interact);};
  },[session,position,show,mobile]);
  return <section {...props} ref={root}>{children}</section>;
}

/** Both sizes use the workspace file page's actual deck. Session-only
 * actions live in its existing menu, leaving the file's navigation,
 * source/preview, saving and folding in one implementation. */
function SessionContentFile({tab,onOpen,onBrowse,...props}: ComponentProps<typeof FilePage> & {
  tab:ContentTab; onOpen:(path:string)=>void;onBrowse:()=>void;
}) {
  const {files,at}=useContentNeighbors(tab,props.machine);
  const acts=[
    {key:'content-browse',label:gui.content_find,run:onBrowse},
    ...(at>0?[{key:'content-previous',label:gui.content_previous,run:()=>onOpen(files[at-1])}]:[]),
    ...(at>=0&&at<files.length-1?[{key:'content-next',label:gui.content_next,run:()=>onOpen(files[at+1])}]:[]),
  ];
  return <FilePage {...props} moreActs={acts}/>;
}

export function SessionContentPane({ container, content, session, devices, rows, visible, onClose, mobile = false, navigation, sessionFaces, onInspect, onCloseSession, onNewSession }: {
  container: HTMLElement; content: SessionContent; session: string | null;
  devices: DeviceRow[]; rows: SessionRow[]; visible: boolean; mobile?: boolean;
  sessionFaces: TabFace[];onInspect:(id:string)=>void;onCloseSession:(id:string)=>void;onNewSession:()=>void;
  navigation: ContentNavigation; onClose: () => void;
}) {
  const visited = useRef(new Set<string>()), pickers = useRef(new Set<string>());
  // Strip order is navigation, not DOM order. Moving retained editor/page
  // fibers needlessly reconnects effects (including StrictMode replay).
  const viewOrder=useRef<string[]>([]);
  const byHandle=new Map(content.allTabs.map(tab=>[content.handle(tab),tab]));
  viewOrder.current=viewOrder.current.filter(key=>byHandle.has(key));
  for(const key of byHandle.keys())if(!viewOrder.current.includes(key))viewOrder.current.push(key);
  const views=viewOrder.current.map(key=>byHandle.get(key)!);
  const [drafts, setDrafts] = useState<Record<string, Draft | undefined>>({});
  const [focused, setFocused] = useState(false);
  const state = session ? navigation.get(session) : null;
  const selected = content.active(session);
  const active=state?.scene==='document'||state?.scene==='picker'?state.key:null;
  if (visible && active) visited.current.add(active);
  if (visible && active && state?.scene === 'picker') pickers.current.add(active);
  useEffect(()=>{if(visible&&session&&content.ready&&!navigation.has(session))navigation.resume(session);},[visible,session,content.ready]);
  const fromNode = useRef(new Map<string, string>());
  useEffect(() => {
    if (!session || !selected) return;
    const key = content.handle(selected), before = fromNode.current.get(session);
    fromNode.current.set(session, key);
    if(before&&before!==key&&state?.scene!=='overview'&&state?.scene!=='sessions'&&(state?.scene!=='document'||content.allTabs.some(t=>content.handle(t)===state.key)))navigation.select(selected);
  }, [session, selected?.id, mobile, visible]);
  useEffect(() => {
    if(!content.ready||!visible||!session)return;
    const live=navigation.get(session);
    if(live.key&&!content.allTabs.some(t=>t.session===session&&content.handle(t)===live.key)){
      const next=content.active(session);
      if(next)navigation.select(next);
      else{navigation.dismiss(session);content.setMaximized(null);onClose();}
    }
  }, [mobile, visible, session, active, content.ready, content.allTabs.map(t=>t.id).join(',')]);
  useEffect(() => {
    const locate = (event: Event) => {
      if (event.target instanceof Element) setFocused(!!event.target.closest('[data-content-visible="true"], [data-content-picker], [data-content-tabs], [data-content-panel]'));
    };
    document.addEventListener('focusin', locate); document.addEventListener('pointerdown', locate, true);
    return () => { document.removeEventListener('focusin', locate); document.removeEventListener('pointerdown', locate, true); };
  }, []);
  const peers = content.allTabs.filter(t => t.session === session);
  const current = peers.find(t => content.handle(t) === active);
  const pick = (tab: ContentTab) => {
    navigation.select(tab);
    quiet(content.focus(tab.id));
  };
  const openDocument = (owner: string, device: DeviceRow, path: string, view: 'file'|'page', line?: number|null) => {
    (document.activeElement as HTMLElement|null)?.blur();
    navigation.document(content.openFromPicker(owner,navigation.get(owner).key,device.id,path,view,line));
  };
  useTabHost(visible && focused ? {
    count:peers.length,front:!!current||state?.scene==='picker',
    close:()=>{if(state?.scene==='picker'){if(!navigation.closePicker(session!))onClose();}else if(current)quiet(content.close(current.id));},
    step: by => { const at = peers.findIndex(t => content.handle(t) === active); const next = peers[(at + by + peers.length) % peers.length]; if (next) pick(next); },
  } : null);
  const faceFor=(id:string):TabFace|undefined=>{
    const opened=sessionFaces.find(f=>f.id===id);if(opened)return opened;
    const row=rows.find(r=>r.id===id);return row?{id:row.id,label:row.title,path:sessionCwdOnScreen(row)||'',face:row.face,session:{word:row.word,kind:row.kind,category:row.category}}:undefined;
  };
  const face=session?faceFor(session):undefined;
  const owner = rows.find(r => r.id === session), home = devices.find(d => d.name === owner?.home);
  useNewAct(visible && focused && session && home ? { label:gui.content_new,tab:true,run:()=>navigation.add(session) } : null);
  const pages = content.allTabs.filter(t => contentView(t) === 'page');
  const pageIds = pages.map(content.handle).sort().join(',');
  const machines = [...new Set(pages.map(t => t.device))].sort().join(',');
  useEffect(() => { if (content.ready) releasePages(pageIds ? pageIds.split(',') : [], 'content'); }, [content.ready, pageIds]);
  useEffect(() => { if (content.ready) releaseBorrows(machines ? machines.split(',') : [], 'content'); }, [content.ready, machines]);
  const point = (tab: ContentTab, device: DeviceRow, path: string, title = '') => quiet(content.point(tab.id, device.id, path, contentView(tab), title));
  return createPortal(<NarrowBox data-session-content-pane data-mobile={mobile || undefined} className="session-content-pane" aria-hidden={!visible} inert={!visible}>

    {content.error && <div role="alert" className="session-content-error">{content.error}</div>}
    {views.map(tab => {
      const key=content.handle(tab),scoped=navigation.forTab(tab.session,key),p=scoped.get(tab.session);
      const isFront=visible&&tab.session===session&&key===active;
      const picking=p.scene==='picker'||contentView(tab)==='workspace';
      const show=isFront&&!picking,pickerShow=isFront&&picking;
      if(pickerShow)pickers.current.add(key);
      const row=rows.find(r=>r.id===tab.session),face=faceFor(tab.session);
      const device=devices.find(d=>d.id===tab.device);
      // A tab keeps its identity while its destination changes. Drafts belong
      // to the destination so leaving one file cannot seed another file.
      const draftKey=JSON.stringify([key,tab.device,tab.url]);
      const browse=()=>{scoped.update(tab.session,{source:contentView(tab)==='page'?'browser':'workspace'});navigation.browse(tab.session);};
      const browser=(onBusy?:(busy:boolean)=>void)=><BrowserPane onIcon={content.recordPageIcon} onBusy={onBusy} tab={key} device={device?.id||tab.device} machine={device?.name||''} here={!!device?.me} url={tab.url} onOpened={url=>{void recentVisit('page',device?.name||'',url,tab.session).catch(()=>{});releasePages(pageIds?pageIds.split(','):[],'content');}} onWent={url=>quiet(content.follow(tab.id,url))} onTitle={title=>quiet(content.rename(tab.id,title))} onPopped={url=>navigation.document(content.open(tab.session,device!.id,url,'page',undefined,tab.id))}/>;
      return <Fragment key={key}>
        {row&&face&&pickers.current.has(key)&&<ContentPosition session={tab.session} position={contentPickerPosition(key,p)} show={pickerShow} mobile={mobile} navigation={scoped} data-content-picker={tab.session} data-content-picker-handle={key} className={cn('session-content-view',!pickerShow&&'session-content-retained')} aria-hidden={!pickerShow} inert={!pickerShow}><ContentPicker desktop={!mobile} face={face} content={content} onClose={onClose} row={row} devices={devices} navigation={scoped} onOpen={(device,path,view,line)=>openDocument(tab.session,device,path,view,line)}/></ContentPosition>}
        <ContentPosition session={tab.session} position={JSON.stringify(['document',key,tab.url])} show={show} mobile={mobile} navigation={scoped} data-session-content={tab.id} data-content-handle={key} data-content-visible={show} className={cn('session-content-view',!show&&'session-content-retained')} aria-hidden={!show} inert={!show}>
          {!device ? contentView(tab)!=='workspace'&&<div className="session-content-unknown">{gui.content_machine_gone}</div> : <>
            {contentView(tab)==='file'?(show||visited.current.has(key))&&<SessionContentFile key={draftKey} tab={tab} onBrowse={browse} onOpen={path=>openDocument(tab.session,device,path,'file')} machine={device.name} path={tab.url} here={device.me} git draft={drafts[draftKey]} onDraft={draft=>{setDrafts(all=>all[draftKey]===(draft||undefined)?all:{...all,[draftKey]:draft||undefined});}} goLine={content.lines[key]||content.lines[tab.id]} contentDeck={mobile&&face?contentFileDeck({session:tab.session,face,content,navigation:scoped,onClose,title:contentName(tab),devices,tab}):undefined} onBack={()=>{if(!navigation.back(tab.session))onClose();}} onGone={()=>quiet(content.close(tab.id))} onMoved={(_from,to)=>point(tab,device,to)}/>:contentView(tab)==='page'?show&&face&&<MobileContentPage desktop={!mobile} tab={tab} handle={key} device={device} devices={devices} face={face} content={content} navigation={scoped} onClose={onClose} onPoint={(d,url)=>point(tab,d,url)}>{onBusy=>browser(onBusy)}</MobileContentPage>:null}
          </>}
        </ContentPosition>
      </Fragment>;
    })}
    {session&&state?.scene==='sessions'&&<ContentPosition session={session} position="sessions" show={visible} mobile={mobile} navigation={navigation} data-content-sessions className="session-content-view mobile-content-overview"><ContentSessionOverview faces={sessionFaces} active={session} onFocus={onInspect} onCloseSession={onCloseSession} onNew={onNewSession} onClose={onClose} query={state.filter} onQuery={filter=>navigation.update(session,{filter})}/></ContentPosition>}
    {session && <ContentPosition session={session} position="overview" show={visible&&state?.scene==='overview'} mobile={mobile} navigation={navigation} data-content-overview className={cn('session-content-view', !(visible && state?.scene==='overview') && 'session-content-retained')} aria-hidden={state?.scene!=='overview'} inert={state?.scene!=='overview'}><ContentOverview face={face!} onClose={onClose} session={session} row={owner} devices={devices} content={content} navigation={navigation} onPick={pick}/></ContentPosition>}
  </NarrowBox>, container);
}
