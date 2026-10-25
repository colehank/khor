import {useLayoutEffect,useState} from 'react';
import {createPortal} from 'react-dom';
import {IconBack,IconCompose} from '@/components/icons';
import type {SessionHeaderProps} from '@/components/SessionBar';
import {gui} from '@/gen/catalog';
import {SessionFanNavigator} from './session-nav/fan';
import '@/styles/session-layout.css';

export function SessionHeader(props:SessionHeaderProps){
  const {barRef,onBack,onNew,newLabel,children,hidden}=props,[slot,setSlot]=useState<Element|null>(null);
  useLayoutEffect(()=>{
    const root=barRef.current?.closest('[data-session-screen]');if(!root)return;
    const find=()=>setSlot(root.querySelector('[data-lab-composer-session-slot]'));find();
    const observer=new MutationObserver(find);observer.observe(root,{childList:true,subtree:true});return()=>observer.disconnect();
  },[barRef,props.active]);
  return <>
    <header ref={barRef} data-session-bar data-session-deck data-lab-model-header style={hidden?{visibility:'hidden'}:undefined}>
      <button type="button" className="lab-header-icon" data-lab-model-back aria-label={gui.back} onClick={onBack}><IconBack/></button>
      <div data-lab-model-settings>{children}</div>
      {onNew&&<button type="button" className="glass lab-compose-new" data-lab-new-session aria-label={newLabel??'新建会话'} onClick={onNew}><IconCompose/></button>}
    </header>
    {slot&&createPortal(<SessionFanNavigator {...props}/>,slot)}
  </>;
}
