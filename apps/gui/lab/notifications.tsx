import React, {useState} from 'react';
import {createRoot} from 'react-dom/client';
import {TooltipProvider} from '/src/components/ui/tooltip';
import {Button} from '/src/components/ui/button';
import {NotificationsPage} from '/src/views/NotificationsPage';
import {receiveNotifications,useNotificationInbox} from '/src/lib/notifications';
import {gui} from '/src/gen/catalog';
import '/src/styles/tokens.css';
import '/src/styles/app.css';

document.documentElement.dataset.theme=new URL(location.href).searchParams.get('theme')||'dark';
const now=Date.now();
const event=(id,kind,at=now)=>({id,kind,at_ms:at,resolved:false,text:null,duration_ms:null,exit:null});
const sessions=[
 {id:'gui/khor',title:'Khor-main',kind:'gui',category:'khor',execution_home:'a'.repeat(64),home:'Mac',face:null,imported:false,observation_owner:null,activity:[]},
 {id:'gui/codex',title:'AcaPartner',kind:'gui',category:'codex',execution_home:'b'.repeat(64),home:'Linux',face:null,imported:false,observation_owner:null,activity:[]},
 {id:'shell/build',title:'构建',kind:'shell',category:'shell',execution_home:'b'.repeat(64),home:'Linux',face:null,imported:false,observation_owner:null,activity:[]},
];
receiveNotifications(sessions,()=>false);
sessions[0].activity=[event('ask','needs_input',now-20_000)];
sessions[1].activity=[event('done','turn_completed',now-30_000)];
sessions[2].activity=[{...event('command','command_finished',now-45_000),duration_ms:75_000,exit:0,text:'cargo test'}];
receiveNotifications(sessions,()=>false);
function Lab(){
 const inbox=useNotificationInbox();
 const [opened,setOpened]=useState('');
 return <div style={{height:'100dvh',display:'flex',flexDirection:'column'}}>
  <div data-lab-controls className="flex flex-wrap items-center gap-in p-row text-callout">
   <Button data-lab-resolve variant="ghost" onClick={()=>{sessions[0].activity=[{...sessions[0].activity[0],resolved:true}];receiveNotifications(sessions,()=>false);}}>模拟另一设备已处理</Button>
   <Button data-lab-complete variant="ghost" onClick={()=>{sessions[1].activity=[...sessions[1].activity,event('done-'+Date.now(),'turn_completed',Date.now())];receiveNotifications(sessions,()=>false);}}>模拟完成</Button>
   <span data-lab-opened>{opened}</span>
  </div>
  <div className="min-h-0 flex-1"><NotificationsPage inbox={inbox} sessions={sessions} onOpen={item=>{setOpened(item.session+'@'+item.home.slice(0,4));return true;}} onClose={()=>setOpened(gui.back)}/></div>
 </div>;
}
createRoot(document.getElementById('root')!).render(<TooltipProvider><Lab/></TooltipProvider>);
