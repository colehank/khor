import React,{useState} from 'react';
import {createRoot} from 'react-dom/client';
import {useSessionContent} from '/src/hooks/use-session-content';
import {useContentNavigation} from '/src/hooks/use-content-navigation';
import {SessionContentPane,ContentTabs} from '/src/views/SessionContent';
import {WorkspacePane} from '/src/views/WorkspacePane';
import {SideSeatsProvider,useSideSeats} from '/src/components/SideHead';
import {WorkspaceMemory} from '/src/views/workspace/workspace-memory';
import {useListWidth} from '/src/hooks/use-list-width';
import {TooltipProvider} from '/src/components/ui/tooltip';
import {followScrollEdge} from '/src/lib/scroll-edge';
import {TabStrip} from '/src/browse/TabStrip';
import {SessionFrame} from '/src/components/SessionFrame';
import '/src/styles/tokens.css';import '/src/styles/app.css';
import '/src/styles/session-preferences.css';
followScrollEdge();
const device={id:'fixture',name:'fixture',shown:'Fixture Mac',me:true,face:null};
const row={id:'session',title:'Khor-main',kind:'shell',category:'shell',home:'fixture',place:'/fixture',face:null,word:'idle'};
const face={id:row.id,label:row.title,path:row.place,face:null};
document.documentElement.dataset.theme=new URL(location.href).searchParams.get('theme')||'dark';
function Probe(){
 const content=useSessionContent(false),navigation=useContentNavigation(content);
 const [sampleKept,setSampleKept]=useState(false);
 const [host,setHost]=useState<HTMLElement|null>(null);
 const sideSeats=useSideSeats();
 const [memory]=useState(()=>new WorkspaceMemory()),pane=useListWidth('ws-tree');
 const workspace=new URL(location.href).searchParams.has('workspace'),mobile=new URL(location.href).searchParams.has('mobile');
 document.documentElement.toggleAttribute('data-narrow',mobile);
 window.__workspaceProbe={content,navigation};
 if(new URL(location.href).searchParams.has('preview-samples'))return <TabStrip faces={[{id:'preview',label:'Temporary commit / file',sub:'src/docs/example.md',preview:!sampleKept,dirty:true,rang:true,face:null,path:'/fixture/a.md',onPin:()=>setSampleKept(true)},{id:'fixed',label:'Kept document',sub:'src/docs/kept.md',face:null,path:'/fixture/b.md'}]} active="preview" onFocus={()=>{}} onClose={()=>{}}/>;
 if(workspace)return <SideSeatsProvider value={sideSeats.seats}><div style={{display:'flex',height:'100vh'}}><aside style={{width:'var(--list-w)',display:'flex',flexDirection:'column'}}><div ref={sideSeats.setHead} style={{height:'var(--pane-head)',display:'flex',paddingInline:'var(--page-inset)'}}/><div ref={sideSeats.setList} style={{display:'flex',flex:1,minHeight:0}}/></aside><main style={{flex:1,minWidth:0}}><WorkspacePane memory={memory} machine="fixture" here root="/fixture" rows={[]} channel="fixture" machineFace={null} onPin={async()=>{}} onClose={async()=>{}} pinFailed={new Set()} closeRefused={new Map()} arrange={{label:'Arrange',options:[],chosen:'',onChoose:()=>{}}} onNewSession={()=>{}} opened={null} onOpenedSeen={()=>{}} reveal={null} onRevealSeen={()=>{}} narrow={false} side={{on:true,pane,set:()=>{}}} listOpen={false} onBack={()=>{}}/></main></div></SideSeatsProvider>;
 return <div data-session-screen style={{height:'100vh',display:'flex',flexDirection:'column'}}>

   {mobile?<div ref={setHost} style={{display:'flex',flex:1,minHeight:0}}/>:<SessionFrame grouped faces={[{...face,again:{hint:'Browse',expanded:true,toggle:()=>navigation.browse(row.id)}}]} active={row.id} onFocus={()=>navigation.browse(row.id)} onClose={()=>{}} stripData={{'data-session-tabs':''}}
     aside={<div data-session-side={row.id} className="flex min-h-0 flex-col border-l" style={{flex:'0 1 var(--session-side-w)',minWidth:0,maxWidth:'65%'}}><ContentTabs session={row.id} open devices={[device]} content={content} navigation={navigation} onAdd={()=>navigation.add(row.id)} onFocus={()=>{}} onHide={()=>{}}><div ref={setHost} className="flex min-h-0 flex-1 flex-col"/></ContentTabs></div>}>
     <div className="p-pane text-muted-foreground">Terminal retained</div>
   </SessionFrame>}
   {host&&<SessionContentPane container={host} content={content} session={row.id} devices={[device]} rows={[row]} mobile={mobile} visible onClose={()=>{}} navigation={navigation} sessionFaces={[face]} onSelectSession={()=>{}} onCloseSession={()=>{}} onNewSession={()=>{}}/>}
 </div>;
}
createRoot(document.getElementById('root')!).render(<TooltipProvider><Probe/></TooltipProvider>);
