/** The same content column docks beside sessions in either landing. */
import type {DeviceRow,SessionRow} from '@/api';
import type {TabFace} from '@/browse/TabStrip';
import {termName} from '@/components/DoingLine';
import {setSessionSide,setSideWidth,useSideWidth} from '@/hooks/use-session-side';
import type {SessionContent} from '@/hooks/use-session-content';
import type {ContentNavigation} from '@/hooks/use-content-navigation';
import {SessionContentSeat,type SessionContentDock,type SessionContentScope} from '@/hooks/use-session-content-dock';
import {ContentTabs} from '@/views/SessionContent';
import {SessionSide} from '@/views/SessionSide';

export type SessionContentContext={content:SessionContent;navigation:ContentNavigation;dock:SessionContentDock;devices:DeviceRow[]};

export function SessionContentColumn({row,context,faces,onSelectSession,onCloseSession,onNewSession,boxRef}:{
  row:SessionRow;context:SessionContentContext;faces:TabFace[];
  onSelectSession:(id:string)=>void;onCloseSession:(id:string)=>void;onNewSession:()=>void;
  boxRef?:(node:HTMLDivElement|null)=>void;
}){
  const width=useSideWidth('workspace'),{content,navigation,devices,dock}=context;
  const close=()=>{content.setMaximized(null);setSessionSide(row.id,{open:false});};
  const scope:SessionContentScope={session:row.id,mobile:false,faces,onSelectSession,onCloseSession,onNewSession,onClose:close};
  return <SessionSide embedded session={row.id} width={width} seat="workspace"
    onSeat={seat=>setSessionSide(row.id,{seat})} onWidth={value=>setSideWidth('workspace',value)} onClose={close} boxRef={boxRef}
    who={{face:row.face,title:termName(row),word:row.word,kind:row.kind,category:row.category}}>
    <ContentTabs devices={devices} session={row.id} open content={content} navigation={navigation}
      onHide={close} onFocus={()=>setSessionSide(row.id,{open:true,seat:'workspace'})}
      onAdd={()=>{navigation.add(row.id);setSessionSide(row.id,{open:true,seat:'workspace'});}}>
      <SessionContentSeat dock={dock} scope={scope}/>
    </ContentTabs>
  </SessionSide>;
}
