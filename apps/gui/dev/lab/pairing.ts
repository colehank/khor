import type {DeviceRow} from '../../src/gen/bindings/DeviceRow';
import {labPairingTicket} from './world';

/** Isolated transport facts for the production pairing form. Never a live ticket. */
export function pairingFixture(devices:DeviceRow[], peers:DeviceRow[]) {
  let invitations=0,joins=0,fail=false,pending=false;
  const remoteTicket='khorLAB_REMOTE_NETWORK';
  const addPeer=()=>{
    const peer=peers.find(row=>!row.me&&!devices.some(known=>known.id===row.id));
    if(peer)devices.push(structuredClone(peer));
  };
  const commands=new Set(['invite','pair','ssh_config_hosts','scanner_here','__ai_preview/pairing']);
  return {
    handles:(path:string)=>commands.has(path),
    request(path:string,body:Record<string,unknown>) {
      if(path==='__ai_preview/pairing') {
        if(typeof body.fail==='boolean')fail=body.fail;
        if(typeof body.pending==='boolean')pending=body.pending;
        if(body.inviteUsed===true)addPeer();
        return {invitations,joins,remoteTicket};
      }
      if(path==='scanner_here')return false;
      if(path==='ssh_config_hosts')return ['lab-server','lab-build'];
      if(path==='invite'){invitations++;return labPairingTicket;}
      joins++;
      if(fail||body.ticket!==remoteTicket)throw new Error('配对凭据无效或已过期，请重新获取');
      if(pending)return '加入申请已提交，等待网络批准';
      addPeer();
      return '已加入示例网络';
    },
  };
}
