/** Links retain the session they came from, including workspace embeds. */
import {useMemo,type ReactNode} from 'react';
import type {SessionRow} from '@/api';
import {askOpenPath,askOpenUrl} from '@/lib/composer-handoff';
import {sessionCwdOnScreen} from '@/lib/session-cwd';
import {LinkHomeProvider,ShowFileProvider,useKnownFiles,type LinkHome} from '@/views/linkHome';

export function SessionLinkScope({row,children}:{row:SessionRow|null;children:ReactNode}){
  const machine=row?.home??'',root=sessionCwdOnScreen(row)??'';
  const {known,see}=useKnownFiles(machine,root);
  const home=useMemo<LinkHome|null>(()=>row?{
    machine,root,known,see,
    open:(path,line)=>askOpenPath(machine,path,false,line,row.id),
    openPage:url=>askOpenUrl(machine,url,row.id),
  }:null,[row?.id,machine,root,known,see]);
  return <LinkHomeProvider value={home}><ShowFileProvider value={(source,path)=>{
    if(row)askOpenPath(source,path,false,null,row.id);
  }}>{children}</ShowFileProvider></LinkHomeProvider>;
}
