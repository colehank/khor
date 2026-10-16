/** Desktop session content. The session's terminal stays in its own pane;
 * these are its files and pages, using the same node tab handles throughout. */
import { useEffect, useLayoutEffect, useRef, useState, type ComponentProps } from "react";
import { createPortal } from "react-dom";
import { recentVisit, type DeviceRow, type SessionRow } from "@/api";
import { MidName, type TabFace } from "@/browse/TabStrip";
import { Button } from "@/components/ui/button";
import { GlassCircle } from "@/components/Glass";
import { RowMenu } from "@/components/RowMenu";
import { FileKindIcon } from "@/components/FileKindIcon";
import { IconBrowser, IconClose, IconPlus } from "@/components/icons";
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

import { contentView, contentName } from '@/lib/content';
export { contentView, contentName } from '@/lib/content';
import type { ContentNavigation } from '@/hooks/use-content-navigation';
import { ContentPicker, ContentOverview, contentFileDeck, useContentNeighbors } from '@/views/MobileSessionContent';
import { MobileContentPage } from '@/views/MobileContentPage';
import { TabSheetRows, TabsGlyph } from '@/components/SessionBar';
import { IconBack } from '@/components/icons';
const quiet = (work: Promise<unknown>) => { void work.catch(() => {}); };
const glyph = (tab: ContentTab) => contentView(tab) === "workspace" ? null : contentView(tab) === "page" ? <IconBrowser /> : <FileKindIcon name={tab.url} />;

/** Only the children scroll. The group head and the add button keep their
 * seats while a measured width animates the capsule's opening and closing. */
export function ContentTabs({session,open,content,navigation,onAdd,onFocus,onHide}:{session:string;open:boolean;content:SessionContent;navigation:ContentNavigation;onAdd:()=>void;onFocus:()=>void;onHide:()=>void}) {
  const measure = useRef<HTMLDivElement>(null), scroller = useRef<HTMLDivElement>(null), action = useRef<HTMLDivElement>(null);
  const [width, setWidth] = useState(0);
  const [cut, setCut] = useState({ left: false, right: false });
  const state=navigation.get(session);
  const browsing:ContentTab={id:'browse:'+session,session,device:'',url:'',title:gui.borrow_web,view:'workspace',pinned:false,touched:0};
  const tabs=[...content.allTabs.filter(t=>t.session===session),...(state.scene==='picker'?[browsing]:[])];
  const current=content.allTabs.find(t=>t.session===session&&content.handle(t)===state.key);
  const active=state.scene==='picker'?browsing.id:current?.id;
  const pick=(tab:ContentTab)=>{onFocus();if(tab.id===browsing.id)navigation.browse(session);else{navigation.document(tab);if(!content.isPreview(tab))quiet(content.focus(tab.id));}};
  const close=(tab:ContentTab)=>{if(tab.id===browsing.id){if(!navigation.back(session))onHide();}else quiet(content.close(tab.id));};
  const seek = () => {
    const box = scroller.current, tab = measure.current?.querySelector<HTMLElement>('[data-content-on="true"]');
    if (!box || !tab || !open) return;
    const b = box.getBoundingClientRect(), r = tab.getBoundingClientRect();
    if (r.left < b.left) box.scrollLeft += r.left - b.left;
    else if (r.right > b.right) box.scrollLeft += r.right - b.right;
  };
  const edges = () => {
    const el = scroller.current;
    if (el) { const next = { left: el.scrollLeft > 1, right: el.scrollWidth - el.clientWidth - el.scrollLeft > 1 }; setCut(was => was.left === next.left && was.right === next.right ? was : next); }
  };
  useLayoutEffect(() => {
    const read = () => setWidth((measure.current?.getBoundingClientRect().width || 0) + (action.current?.getBoundingClientRect().width || 0));
    read();
    const observer = new ResizeObserver(read);
    if (measure.current) observer.observe(measure.current);
    if (action.current) observer.observe(action.current);
    return () => observer.disconnect();
  }, []);
  useLayoutEffect(() => {
    const frame = requestAnimationFrame(() => { seek(); edges(); });
    return () => cancelAnimationFrame(frame);
  }, [active, open, width, tabs.length]);
  useEffect(() => {
    const el = scroller.current;
    if (!el) return;
    const observer = new ResizeObserver(() => { seek(); edges(); });
    observer.observe(el);
    const wheel = (e: WheelEvent) => {
      if (el.scrollWidth - el.clientWidth <= 1 || Math.abs(e.deltaX) >= Math.abs(e.deltaY)) return;
      e.preventDefault(); e.stopPropagation(); el.scrollLeft += e.deltaY;
    };
    el.addEventListener("wheel", wheel, { passive: false });
    return () => { observer.disconnect(); el.removeEventListener("wheel", wheel); };
  }, [open]);
  return <div data-content-tabs={session} data-open={open} aria-hidden={!open} inert={!open} className="session-content-reveal" style={{ width: open ? width : 0 }} onTransitionEnd={seek}>
    <div ref={scroller} onScroll={edges} className="session-content-scroll" style={cut.left || cut.right ? { maskImage: `linear-gradient(to right, ${cut.left ? 'transparent, black var(--fade-tabs)' : 'black, black'}, ${cut.right ? 'black calc(100% - var(--fade-tabs)), transparent' : 'black, black'})` } : undefined}>
      <div ref={measure} className="session-content-tabs" role="tablist" aria-label={gui.session_content}>
        {tabs.map(tab => <div key={tab.id} data-content-tab={tab.id} data-content-on={tab.id === active} data-content-preview={content.isPreview(tab) || undefined} data-content-browse={tab.id===browsing.id||undefined} className="session-content-tab">
          <RowMenu items={tab.id===browsing.id?[{key:"close",label:gui.close_tab,run:()=>close(tab)}]:[content.isPreview(tab) ? { key: "keep", label: gui.content_keep, run: () => content.keep(tab.id).then(() => {}) } : { key: "pin", label: tab.pinned ? gui.unpin : gui.pin, run: () => content.pin(tab.id, !tab.pinned) }, { key: "close", label: gui.close_tab, run: () => content.close(tab.id) }]}><button role="tab" aria-selected={open && tab.id === active} tabIndex={tab.id === active ? 0 : -1} title={tab.url} className="session-content-open" onClick={()=>pick(tab)} onDoubleClick={()=>{if(content.isPreview(tab))quiet(content.keep(tab.id));}} onKeyDown={e => {
            const at = tabs.findIndex(t => t.id === tab.id);
            const to = e.key === 'ArrowRight' ? (at + 1) % tabs.length : e.key === 'ArrowLeft' ? (at + tabs.length - 1) % tabs.length : e.key === 'Home' ? 0 : e.key === 'End' ? tabs.length - 1 : -1;
            if (to < 0) return;
            e.preventDefault();pick(tabs[to]);
            measure.current?.querySelector<HTMLElement>(`[data-content-tab="${CSS.escape(tabs[to].id)}"] [role="tab"]`)?.focus();
          }}>{glyph(tab)}<MidName text={contentName(tab)} /></button></RowMenu>
          {!tab.pinned && <button className="session-content-close" aria-label={gui.close_tab} title={gui.close_tab} onClick={()=>close(tab)}><IconClose /></button>}
        </div>)}
      </div>
    </div>
    <div ref={action} className="session-content-add"><button aria-label={gui.content_new} onClick={onAdd}><IconPlus /></button></div>
  </div>;
}

/** Both sizes use the workspace file page's actual deck. Session-only
 * actions live in its existing menu, leaving the file's navigation,
 * source/preview, saving and folding in one implementation. */
function SessionContentFile({tab,temporary,onKeep,onOpen,onBrowse,...props}: ComponentProps<typeof FilePage> & {
  tab:ContentTab;temporary:boolean; onKeep:()=>void; onOpen:(path:string)=>void;onBrowse:()=>void;
}) {
  const {files,at}=useContentNeighbors(tab,props.machine);
  const acts=[
    ...(temporary?[{key:'content-keep',label:gui.content_keep,run:onKeep}]:[]),
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
  const [drafts, setDrafts] = useState<Record<string, Draft | undefined>>({});
  const [focused, setFocused] = useState(false);
  const state = session ? navigation.get(session) : null;
  const selected = content.active(session);
  const active=state?.scene==='document'?state.key:null;
  if (visible && active) visited.current.add(active);
  if (visible && session && state?.scene === 'picker') pickers.current.add(session);
  useEffect(()=>{if(visible&&session&&content.ready&&!navigation.has(session))navigation.resume(session);},[visible,session,content.ready]);
  useEffect(()=>{if(!mobile&&visible&&session&&(state?.scene==='overview'||state?.scene==='sessions'))navigation.resume(session);},[mobile,visible,session,state?.scene]);
  const fromNode = useRef(new Map<string, string>());
  useEffect(() => {
    if (!session || !selected) return;
    const key = content.handle(selected), before = fromNode.current.get(session);
    fromNode.current.set(session, key);
    if(before&&before!==key&&state?.scene!=='overview'&&(state?.scene!=='document'||content.allTabs.some(t=>content.handle(t)===state.key)))navigation.document(selected);
  }, [session, selected?.id, mobile, visible]);
  useEffect(() => {
    if (visible && session && state?.scene === 'document' && !content.allTabs.some(t => t.session === session && content.handle(t) === state.key)) navigation.back(session)||navigation.browse(session);
  }, [mobile, visible, session, active, content.allTabs.map(t=>t.id).join(',')]);
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
    navigation.document(tab);
    if (!content.isPreview(tab)) quiet(content.focus(tab.id));
  };
  const preview = (owner: string, device: DeviceRow, path: string, view: 'file'|'page', line?: number|null) => {
    (document.activeElement as HTMLElement|null)?.blur();
    navigation.document(content.peek(owner, device.id, path, view, line));
  };
  useTabHost(visible && focused ? {
    count:peers.length,front:!!current||state?.scene==='picker',
    close:()=>{if(state?.scene==='picker'){if(!navigation.back(session!))onClose();}else if(current)quiet(content.close(current.id));},
    step: by => { const at = peers.findIndex(t => content.handle(t) === active); const next = peers[(at + by + peers.length) % peers.length]; if (next) pick(next); },
  } : null);
  const faceFor=(id:string):TabFace|undefined=>{
    const opened=sessionFaces.find(f=>f.id===id);if(opened)return opened;
    const row=rows.find(r=>r.id===id);return row?{id:row.id,label:row.title,path:sessionCwdOnScreen(row)||'',face:row.face,session:{word:row.word,kind:row.kind,category:row.category}}:undefined;
  };
  const face=session?faceFor(session):undefined;
  const owner = rows.find(r => r.id === session), home = devices.find(d => d.name === owner?.home);
  useNewAct(visible && focused && session && home ? { label:gui.content_find,tab:true,run:()=>navigation.browse(session) } : null);
  const pages = content.allTabs.filter(t => contentView(t) === 'page');
  const pageIds = pages.map(content.handle).sort().join(',');
  const machines = [...new Set(pages.map(t => t.device))].sort().join(',');
  useEffect(() => { if (content.ready) releasePages(pageIds ? pageIds.split(',') : [], 'content'); }, [content.ready, pageIds]);
  useEffect(() => { if (content.ready) releaseBorrows(machines ? machines.split(',') : [], 'content'); }, [content.ready, machines]);
  useEffect(()=>{if(!mobile&&visible&&content.ready&&session&&state?.scene==='document'&&!peers.length){content.setMaximized(null);onClose();}},[mobile,visible,content.ready,session,state?.scene,peers.length]);
  const point = (tab: ContentTab, device: DeviceRow, path: string, title = '') => quiet(content.point(tab.id, device.id, path, contentView(tab), title));
  return createPortal(<NarrowBox data-session-content-pane data-mobile={mobile || undefined} className="session-content-pane" aria-hidden={!visible} inert={!visible}>

    {content.error && <div role="alert" className="session-content-error">{content.error}</div>}
    {rows.filter(row => pickers.current.has(row.id)).map(row => {
      const show=visible&&row.id===session&&state?.scene==='picker';
      return <section key={'picker:'+row.id} data-content-picker={row.id} className={cn('session-content-view', !show && 'session-content-retained')} aria-hidden={!show} inert={!show}><ContentPicker desktop={!mobile} face={faceFor(row.id)!} content={content} onClose={onClose} row={row} devices={devices} navigation={navigation} onOpen={(device,path,view,line)=>preview(row.id,device,path,view,line)}/></section>;
    })}
    {content.allTabs.map(tab => {
      const key = content.handle(tab), show = visible && tab.session === session && key === active;
      const device = devices.find(d => d.id === tab.device);
      const temporary = content.isPreview(tab);
      const browse = () => {
        navigation.update(tab.session, { source: contentView(tab)==='page'?'browser':'workspace' });
        navigation.browse(tab.session);
      };
      const browser=(onBusy?: (busy:boolean)=>void)=><BrowserPane onBusy={onBusy} tab={key} device={device?.id||tab.device} machine={device?.name||''} here={!!device?.me} url={tab.url} onOpened={url=>{void recentVisit('page',device?.name||'',url,tab.session).catch(()=>{});releasePages(pageIds?pageIds.split(','):[],'content');}} onWent={url=>quiet(content.follow(tab.id,url))} onTitle={title=>quiet(content.rename(tab.id,title))} onPopped={url=>quiet(content.add(tab.session,device!.id,url,'page',temporary?undefined:tab.id).then(id=>navigation.update(tab.session,{scene:'document',key:id})))}/>;
      return <section key={key} data-session-content={tab.id} data-content-handle={key} data-content-visible={show} data-content-preview={temporary || undefined} className={cn('session-content-view', !show && 'session-content-retained')} aria-hidden={!show} inert={!show}>
        {!device ? <div className="session-content-unknown">{gui.content_machine_gone}</div> : <>

          {contentView(tab) === 'file' ? (show || visited.current.has(key)) && <SessionContentFile tab={tab} temporary={temporary} onKeep={()=>quiet(content.keep(tab.id))} onBrowse={browse} onOpen={path=>preview(tab.session,device,path,'file')} machine={device.name} path={tab.url} here={device.me} git draft={drafts[key]} onDraft={draft => {
            setDrafts(all => all[key] === (draft || undefined) ? all : { ...all, [key]: draft || undefined });
            if (draft && temporary) quiet(content.keep(tab.id));
          }} goLine={content.lines[key] || content.lines[tab.id]} contentDeck={mobile&&faceFor(tab.session)?contentFileDeck({session:tab.session,face:faceFor(tab.session)!,content,navigation,onClose,title:contentName(tab)}):undefined} onBack={()=>{if(!navigation.back(tab.session))onClose();}} onGone={() => quiet(content.close(tab.id))} onMoved={(_from,to) => point(tab,device,to)} /> : contentView(tab) === 'page' ? show && (faceFor(tab.session)?<MobileContentPage desktop={!mobile} tab={tab} handle={key} device={device} devices={devices} face={faceFor(tab.session)!} content={content} navigation={navigation} onClose={onClose} onPoint={(d,url)=>point(tab,d,url)} onKeep={()=>quiet(content.keep(tab.id))}>{onBusy=>browser(onBusy)}</MobileContentPage>:null) : null}
        </>}
      </section>;
    })}
    {mobile&&session&&state?.scene==='sessions'&&<section data-content-sessions className="session-content-view mobile-content-overview"><div className="mobile-content-overview-head"><h2>{gui.session_tabs}</h2><Button variant="ghost" size="sm" onClick={onClose}>{gui.edit_done}</Button></div><div className="mobile-content-overview-list"><TabSheetRows faces={sessionFaces} active={session} onFocus={onInspect} onClose={onCloseSession} onNew={onNewSession} newLabel={gui.new_session}/></div><div className="mobile-content-deck"><GlassCircle aria-label={gui.back} onClick={onClose}><IconBack/></GlassCircle><div className="glass mobile-content-pill"><span className="px-row text-body">{gui.session_tabs}</span></div><GlassCircle aria-label={gui.switch_tabs(sessionFaces.length)} onClick={onClose}><TabsGlyph n={sessionFaces.length}/></GlassCircle></div></section>}
    {mobile && session && <section data-content-overview className={cn('session-content-view', !(visible && state?.scene==='overview') && 'session-content-retained')} aria-hidden={state?.scene!=='overview'} inert={state?.scene!=='overview'}><ContentOverview face={face!} onClose={onClose} session={session} row={owner} devices={devices} content={content} navigation={navigation} onPick={pick}/></section>}
  </NarrowBox>, container);
}
