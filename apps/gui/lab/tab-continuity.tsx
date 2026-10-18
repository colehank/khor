/** Disposable interaction lab. Content is fixture data; components are production. */
import {useEffect,useRef,useState} from 'react';
import {createRoot} from 'react-dom/client';
import {TabStrip,TabDivider,type TabFace} from '@/browse/TabStrip';
import {fileKind,pageKind,newTabKind,SessionFace} from '@/components/SessionFace';
import {SessionScreen,TabSheetRows,ContentTabsGlyph} from '@/components/SessionBar';
import {GlassCircle} from '@/components/Glass';
import {IconBack,IconPlus,IconReload,IconSessions,IconFiles} from '@/components/icons';
import {TooltipProvider} from '@/components/ui/tooltip';
import {ContentPicker,MobileContentSheet,contentFileDeck} from '@/views/MobileSessionContent';
import {MobileContentPage} from '@/views/MobileContentPage';
import {FilePage} from '@/views/FilePage';
import {NewSessionPage} from '@/views/NewSessionPage';
import {SettingsProvider} from '@/settings';
import {speakLang} from '@/lang';
import {useNarrow,NarrowBox} from '@/hooks/use-narrow';
import type {ContentNavigation} from '@/hooks/use-content-navigation';
import type {ContentTab,SessionContent} from '@/hooks/use-session-content';
import fixtureData from './tab-continuity-fixture.json';
import type {Avatar} from '@/gen/bindings/Avatar';
import type {DeviceRow,SessionRow} from '@/api';
import {NEW_CONTENT,rootTab,addChild,changeChild,pointChild,closeChild,reorder,type Root,type Child} from './tab-continuity-model';
import '@/styles/tokens.css';
import '@/styles/app.css';
import '@/styles/session-content.css';
import './tab-continuity.css';
speakLang('zh');
type RootUpdate=Root|{id:string;change:(r:Root)=>Root};
const devices=fixtureData.devices as unknown as DeviceRow[];
const avatar=devices[0].face as Avatar;
const siteIcon='data:image/svg+xml,'+encodeURIComponent('<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><rect width="24" height="24" rx="6" fill="#5d7e62"/><path d="M7 6v12m9-12-7 6 7 6" fill="none" stroke="white" stroke-width="2"/></svg>');
const faceOf=(r:Root):TabFace=>({id:r.id,label:r.title,path:'/Users/zgh/Projects/khor',face:r.kind==='new'?null:avatar,...(r.kind==='new'?{mark:newTabKind()}:{session:{word:'idle',kind:r.agent==='terminal'?'shell':'agent',category:r.agent==='terminal'?null:r.agent}})});
const childFace=(t:Child):TabFace=>({id:t.id,label:t.title,path:t.path,face:t.kind==='new'?null:avatar,...(t.kind==='new'?{hideFace:true}:{mark:t.kind==='file'?fileKind(t.path):pageKind(siteIcon)})});
function initial(){const scenario=new URLSearchParams(location.search).get('scenario');let first=rootTab();if(scenario==='crowded'){for(let i=0;i<8;i++){first=addChild(first);if(i<5)first=pointChild(first,first.active!,['README.md','SHIP.md','App.tsx','identity.rs','Cargo.toml'][i],'file');}first.tabs=first.tabs.map(t=>t.kind==='file'?{...t,path:'/Users/zgh/Projects/khor/'+t.path}:t);}return scenario==='empty'?[first]:[first,rootTab('ACL2027','claude'),rootTab('research','terminal')];}
function NestedStrip({root,onChange,onAdd}:{root:Root;onChange:(r:Root)=>void;onAdd:()=>void}){
 return <div data-content-tabs={root.id} data-open="true" className="session-content-reveal lab-nested" style={{width:`calc(${root.tabs.length} * var(--tab-w-even) + 5 * var(--gap-in) + 3 * var(--hair) + var(--glass-tab))`}}><TabDivider/><TabStrip nested grouped motion="travel" faces={root.tabs.map(childFace)} active={root.active||''} onFocus={id=>onChange({...root,active:id})} onClose={id=>onChange(closeChild(root,id))} onNew={onAdd} onReorder={ids=>onChange({...root,tabs:reorder(root.tabs,ids)})}/></div>;
}
function Transcript({root}:{root:Root}){return <div className="lab-transcript"><p className="lab-muted">{root.agent==='claude'?'Claude Code':'OpenAI Codex'}　/Users/zgh/Projects/khor</p><p className="lab-request">› 把工作区和网页放在当前会话旁边</p><p>会话会留在这里。你可以同时查看代码、目录和网页。</p><p>打开多个新会话标签页，再分别进入不同目录，或输入网址。</p><pre>{'  src/\n    App.tsx\n    components/\n    views/\n  docs/\n    SHIP.md'}</pre><p className="lab-muted">修改已完成，等待下一步</p><div className="lab-prompt">› <span className="lab-cursor"/></div><p className="lab-muted">{root.title} · main · 空闲</p></div>}
function Content({root,tab,put,close,overview,narrow}:{root:Root;tab:Child;put:(r:RootUpdate)=>void;close:()=>void;overview:()=>void;narrow:boolean}){
 const scrolls=useRef<Record<string,{top:number;left:number}>>({});
 const mutate=(change:(r:Root)=>Root)=>put({id:root.id,change});
 const patch=(p:Partial<Child>|((t:Child)=>Partial<Child>))=>mutate(r=>{const before=r.tabs.find(t=>t.id===tab.id);return before?changeChild(r,tab.id,typeof p==='function'?p(before):p):r});
 const browse=()=>patch({kind:'new',title:NEW_CONTENT});
 const records:ContentTab[]=root.tabs.map(t=>({id:t.id,session:root.id,device:'mac',url:t.path,title:t.title,view:t.kind==='new'?'workspace':t.kind,pinned:false,touched:0}));
 const nav={get:()=>({...tab.place,scene:tab.kind==='new'?'picker':'document',key:tab.id,filter:'',from:'picker',trail:[],recent:tab.id,documentTrail:[],scrolls:scrolls.current,browseBefore:null,lastContent:null}),update:(_id:string,p:Record<string,unknown>)=>patch(t=>({place:{...t.place,...p}})),editor:(id:string)=>tab.editors[id]||{},edit:(id:string,p:Record<string,unknown>)=>patch(t=>({editors:{...t.editors,[id]:{...t.editors[id],...p}}})),browse,overview,sessions:overview,back:()=>{browse();return true},document:(t:ContentTab)=>mutate(r=>({...r,active:t.id})),scrollOf:(_id:string,key:string)=>scrolls.current[key],scroll:(_id:string,key:string,value:{top:number;left:number})=>{scrolls.current[key]=value}} as unknown as ContentNavigation;
 const content={allTabs:records,tabs:records,handle:(t:ContentTab)=>t.id,active:()=>records.find(t=>t.id===root.active),focus:async(id:string)=>mutate(r=>({...r,active:id})),close:async(id:string)=>mutate(r=>closeChild(r,id)),pageIcon:()=>siteIcon} as unknown as SessionContent;
 const face=faceOf(root),row={id:root.id,title:root.title,home:'MacLab',place:'/Users/zgh/Projects/khor',face:avatar,kind:root.agent==='terminal'?'shell':'agent',category:root.agent,word:'idle'} as SessionRow;
 const owner={session:root.id,face,content,navigation:nav,onClose:close};
 const open=(_d:DeviceRow,path:string,kind:'file'|'page')=>mutate(r=>pointChild(r,tab.id,path,kind));
 if(tab.kind==='new')return <ContentPicker {...owner} desktop={!narrow} row={row} devices={devices} onOpen={open}/>;
 if(tab.kind==='file')return <FilePage machine="MacLab" path={tab.path} here embedded={!narrow} draft={tab.draft} onDraft={draft=>{if(draft?.text!==tab.draft?.text||draft?.was!==tab.draft?.was||draft?.by!==tab.draft?.by)patch({draft:draft||undefined})}} onBack={browse} onGone={()=>mutate(r=>closeChild(r,tab.id))} onMoved={(_from,to)=>patch({path:to,title:to.split('/').pop()||to})} contentDeck={narrow?contentFileDeck({...owner,title:tab.title}):undefined}/>;
 return <MobileContentPage {...owner} desktop={!narrow} tab={records.find(t=>t.id===tab.id)!} handle={tab.id} device={devices[0]} devices={devices} onPoint={(d,path)=>open(d,path,'page')}>{()=> <iframe className="lab-site" title={tab.title} src="/lab-site"/>}</MobileContentPage>;
}
function Lab(){
 const [roots,setRoots]=useState(initial),[active,setActive]=useState(()=>roots[0]?.id||''),[sheet,setSheet]=useState<'tabs'|'sessions'|null>(null),[light,setLight]=useState(false);
 const narrow=useNarrow(),root=roots.find(r=>r.id===active),serialRef=useRef(0);
 useEffect(()=>{let runs=0;const observer=new MutationObserver(records=>{for(const record of records)for(const node of record.addedNodes){if(!(node instanceof HTMLElement)||!node.hasAttribute('data-sel-ghost'))continue;const nested=!!node.closest('[data-tabs-nested]');window.parent.postMessage({type:'lab-motion',layer:nested?'内容标签':'会话标签',ms:node.getAnimations()[0]?.effect?.getTiming().duration,runs:++runs},location.origin)}});observer.observe(document.getElementById('root')!,{childList:true,subtree:true});return()=>observer.disconnect()},[]);
 const put=(next:RootUpdate)=>{setRoots(rows=>rows.map(r=>r.id===next.id?('change' in next?next.change(r):next):r));if(!('change' in next)&&!next.tabs.length&&!next.expanded)setSheet(null)};
 const addRoot=()=>{const next={...rootTab('新建会话','terminal'),kind:'new' as const};setRoots(rows=>[...rows,next]);setActive(next.id);setSheet(null)};
 const closeRoot=(id:string)=>{const at=roots.findIndex(r=>r.id===id),next=roots.filter(r=>r.id!==id);setRoots(next);if(active===id)setActive(next[Math.min(at,next.length-1)]?.id||'');setSheet(null)};
 const add=()=>{if(root){put(addChild(root));setSheet(null)}};
 const toggle=()=>{if(!root||root.kind==='new')return;put(root.expanded?{...root,expanded:false}:root.tabs.length?{...root,expanded:true}:addChild(root));setSheet(null)};
 useEffect(()=>{const theme=(e:MessageEvent)=>{if(e.data==='theme')setLight(v=>!v)};window.addEventListener('message',theme);return()=>window.removeEventListener('message',theme)},[]);
 useEffect(()=>{document.documentElement.dataset.theme=light?'light':'dark';document.documentElement.toggleAttribute('data-narrow',narrow)},[light,narrow]);
 const faces=roots.map(faceOf),childFaces=root?.tabs.map(childFace)||[];
 const closeContent=()=>{if(root)put({...root,expanded:false});setSheet(null)};
 const overview=root&&<div className="mobile-content-overview"><div className="mobile-content-overview-list"><TabSheetRows faces={childFaces} active={root.active||''} onFocus={id=>{put({...root,active:id});setSheet(null)}} onClose={id=>put(closeChild(root,id))} onNew={add} newLabel={NEW_CONTENT}/></div><div className="mobile-content-deck"><GlassCircle aria-label="返回内容" onClick={()=>setSheet(null)}><IconBack/></GlassCircle><div className="glass mobile-content-pill lab-overview-label">会话标签页</div><GlassCircle aria-label="会话标签页" onClick={()=>setSheet(null)}><ContentTabsGlyph n={root.tabs.length}/></GlassCircle></div></div>;
 const sessions=<div className="mobile-content-overview"><div className="mobile-content-overview-list"><TabSheetRows faces={faces} active={active} onFocus={id=>{setActive(id);setSheet(null)}} onClose={closeRoot} onNew={addRoot} newLabel="新建会话"/></div><div className="mobile-content-deck"><GlassCircle aria-label="返回会话" onClick={()=>setSheet(null)}><IconBack/></GlassCircle><div className="glass mobile-content-pill lab-overview-label">查找会话</div></div></div>;
 const pages=root&&<NarrowBox className="lab-content-host"><div className="lab-content-host">{sheet==='tabs'?overview:sheet==='sessions'?sessions:root.tabs.map(t=><section className="lab-child-view" hidden={t.id!==root.active} key={t.id}><Content root={root} tab={t} put={put} close={closeContent} overview={()=>setSheet('tabs')} narrow={narrow}/></section>)}</div></NarrowBox>;
 const newSession=root&&root.kind==='new'&&<NewSessionPage dir="~/" onCancel={()=>closeRoot(root.id)} onOpened={()=>put({...root,kind:'session',title:'shell '+(++serialRef.current),agent:'terminal'})}/>;
 return <SettingsProvider value={null}><TooltipProvider><div className="lab-shell" data-lab-tabs>
 {!narrow&&<aside className="lab-sidebar"><div className="lab-rail"><IconSessions/><IconFiles/></div><div className="lab-session-heading"><h1>会话</h1><GlassCircle aria-label="新建会话" onClick={addRoot}><IconPlus/></GlassCircle></div><p className="lab-group">空闲</p>{roots.filter(r=>r.kind==='session').map(r=><button key={r.id} className="lab-session-row" data-selected={r.id===active} onClick={()=>setActive(r.id)}><SessionFace face={avatar} word="idle" kind={r.agent==='terminal'?'shell':'agent'} category={r.agent==='terminal'?null:r.agent}/><span><strong>{r.title}</strong><small>/Users/zgh/Projects/khor</small></span><small>空闲</small></button>)}</aside>}
 <main className="lab-main">{!narrow?<><TabStrip grouped motion="travel" faces={faces.filter(f=>!root?.expanded||f.id===active).map(f=>({...f,again:roots.find(r=>r.id===f.id)?.kind==='session'?{hint:root?.expanded?'收起会话标签页':'展开会话标签页',expanded:root?.expanded,toggle}:undefined,group:f.id===active&&root?.expanded?<NestedStrip root={root} onChange={put} onAdd={add}/>:undefined}))} active={active} onFocus={setActive} onClose={closeRoot} onNew={root?.expanded?undefined:addRoot} onReorder={ids=>setRoots(reorder(roots,ids))} tail={<GlassCircle aria-label="重新连接（终端为演示内容）" disabled><IconReload/></GlassCircle>}/><div className="lab-panes">{newSession||root&&<><div className="lab-terminal"><Transcript root={root}/></div><aside className="lab-side" hidden={!root.expanded}>{pages}</aside></>}</div></>:root?<SessionScreen faces={faces} active={active} onFocus={setActive} onClose={closeRoot} onNew={addRoot} onBack={()=>{}} onTitle={toggle} onTabs={()=>setSheet('sessions')} away={root.expanded||!!sheet} contentMode={root.expanded||!!sheet} folded={false}>{newSession||<Transcript root={root}/ >}{(root.expanded||sheet)&&<MobileContentSheet onClose={closeContent}>{sheet==='sessions'?sessions:pages}</MobileContentSheet>}</SessionScreen>:<button onClick={addRoot}>新建会话</button>}</main>
 </div></TooltipProvider></SettingsProvider>;
}
function HaloGallery(){
 const states=[['busy','忙碌'],['blocked','等你'],['done','完成'],['errored','异常'],['failed','失败'],['idle','空闲']];
 const [active,setActive]=useState('busy'),[light,setLight]=useState(false);
 useEffect(()=>{const theme=(e:MessageEvent)=>{if(e.data==='theme')setLight(v=>!v)};window.addEventListener('message',theme);return()=>window.removeEventListener('message',theme)},[]);
 useEffect(()=>{document.documentElement.dataset.theme=light?'light':'dark'},[light]);
 return <SettingsProvider value={null}><TooltipProvider><div className="lab-halo-gallery"><h1>头像状态环</h1><p>同一个 SessionFace · 标签与列表 · 原始尺寸</p><TabStrip grouped motion="travel" faces={states.map(([id,label])=>({id,label,face:avatar,path:'',session:{word:id,kind:'agent',category:'codex'}}))} active={active} onFocus={setActive} onClose={()=>{}}/><div className="lab-halo-rows">{states.map(([word,label])=><div key={word}><SessionFace face={avatar} word={word} kind="agent" category="codex"/><span>{label}</span></div>)}</div></div></TooltipProvider></SettingsProvider>;
}
const mount=import.meta.hot?.data.mount||createRoot(document.getElementById('root')!);
if(import.meta.hot)import.meta.hot.data.mount=mount;
mount.render(new URLSearchParams(location.search).get('scenario')==='halo'?<HaloGallery/>:<Lab/>);
