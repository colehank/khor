import React,{useEffect,useState} from 'react';
import {createRoot} from 'react-dom/client';
import {TooltipProvider} from '/src/components/ui/tooltip';
import {Button} from '/src/components/ui/button';
import {NotificationBanners} from '/src/views/NotificationBanners';
import {receiveNotifications,useNotifications,clearNotifications,setSessionNotifications} from '/src/lib/notifications';
import {useNarrow} from '/src/hooks/use-narrow';
import '/src/styles/tokens.css';
import '/src/styles/app.css';

document.documentElement.dataset.theme=new URL(location.href).searchParams.get('theme')||'dark';
const event=(id,kind)=>({id,kind,at_ms:Date.now(),resolved:false,text:null,duration_ms:null,exit:null});
const sessions=[
 {id:'gui/khor',title:'Khor-main',kind:'gui',category:'khor',word:'idle',execution_home:'a'.repeat(64),home:'Mac',face:null,imported:false,observation_owner:null,activity:[]},
 {id:'gui/codex',title:'AcaPartner',kind:'gui',category:'codex',word:'idle',execution_home:'b'.repeat(64),home:'Linux',face:null,imported:false,observation_owner:null,activity:[]},
 {id:'shell/build',title:'构建',kind:'shell',category:'shell',word:'idle',execution_home:'b'.repeat(64),home:'Linux',face:null,imported:false,observation_owner:null,activity:[]},
];
let serial=0;
receiveNotifications(sessions,()=>false);
function send(index,kind='turn_completed'){
 sessions[index].activity=[...sessions[index].activity,event('event-'+(++serial),kind)];receiveNotifications(sessions,()=>false);
}
function Lab(){
 const state=useNotifications(),narrow=useNarrow();
 const [opened,setOpened]=useState('');
 useEffect(()=>{const update=()=>{if(document.hidden)clearNotifications();};document.addEventListener('visibilitychange',update);return()=>document.removeEventListener('visibilitychange',update);},[]);
 return <TooltipProvider><main className="flex h-full flex-col gap-row p-pane">
  <h1 className="text-title-sm">临时通知 Lab</h1>
  <div data-lab-controls className="mt-32 flex flex-wrap items-center gap-in">
   <Button data-lab-complete onClick={()=>send(0)}>会话完成</Button>
   <Button data-lab-burst onClick={()=>{send(0,'needs_input');send(1);send(2);}}>三个会话</Button>
   <Button data-lab-resolve onClick={()=>{sessions[0].activity=sessions[0].activity.map(e=>({...e,resolved:true}));receiveNotifications(sessions,()=>false);}}>已处理</Button>
   <Button data-lab-mute onClick={()=>setSessionNotifications(sessions[0],false)}>静音</Button>
   <Button data-lab-follow onClick={()=>setSessionNotifications(sessions[0],true)}>开启通知</Button>
   <output data-lab-opened>{opened}</output>
  </div>
  <NotificationBanners state={state} sessions={sessions} narrow={narrow} onOpen={item=>{setOpened(item.session+'@'+item.home.slice(0,4));return true;}}/>
 </main></TooltipProvider>;
}
createRoot(document.getElementById('root')!).render(<Lab/>);
