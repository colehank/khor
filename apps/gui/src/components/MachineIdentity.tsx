import type {ReactNode} from 'react';
import type {DeviceRow} from '@/api';
import {MachineFace} from '@/components/SessionFace';

/** Machine page identity, also used by tasks browsing that machine. */
export function MachineIdentity({row,children}:{row:DeviceRow;children?:ReactNode}){
 return <>
  <MachineFace face={row.face} system={row.vitals?.vitals.system??null} size="head"/>
  <span data-head-name data-machine-identity className="flex min-w-0 flex-1 flex-col">
   <span className="min-w-0 truncate text-title-md font-title">{row.shown}</span>
   {children}
  </span>
 </>;
}
