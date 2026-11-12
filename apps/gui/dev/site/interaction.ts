import {roots} from './world';
import {downloadUrl} from './urls';

// Only the finite, complete parts of the website demo are interactive.
// The production App and its controls stay unchanged.
const unavailable=[
  '[data-pane-new]','[data-new-item]','[data-rail-more]',
  '[data-tab-new-seat] button','[data-chat-add]','[data-chat-mode-menu]',
  '[data-lab-session-settings-slot] button','[data-chat-who]',
  '[data-machine-head-buttons] button','[data-promote]',
  '[data-watch-edit]','[data-watch-add]','[data-ws-more]','[data-ws-new-session]',
  '[data-ws-code-control] button','[data-code]','[data-file-more]',
  'button[aria-haspopup]','[data-content-scope] button','[data-content-source]',
  '[data-content-location-edit]','[data-tab-close]','[data-file-face-toggle]',
  '[data-file-refresh]','[data-file-reread]','[data-file-save]','[data-tab-act]',
  '[data-workspace-view]','[data-content-commit-location]',
  '[data-link]',
  '[data-khor-agent-turn] button','[data-khor-agent-settings]',
].join(',');

export function constrainDemoInteractions() {
  let pending=false;
  const apply=()=>{
    pending=false;
    const disable=(element:HTMLElement,row=false)=>{
      const button=element instanceof HTMLButtonElement?element:element.closest('button')??element.querySelector('button');
      const target=button??element;
      if(button)button.disabled=true;
      else{target.setAttribute('aria-disabled','true');target.tabIndex=-1;}
      target.dataset.siteUnavailable='';
      if(row)target.dataset.siteRow='';
    };
    for(const element of document.querySelectorAll<HTMLElement>(unavailable)){
      // The fixed reply's download is a real website action, even inside a turn.
      if(element.dataset.mdLink===downloadUrl)continue;
      disable(element);
    }
    for(const selector of ['[data-row] [data-row-open]','[data-quota-row]','[data-ws-row]']){
      const groups=new Map<Element,HTMLElement[]>();
      document.querySelectorAll<HTMLElement>(selector).forEach(element=>{
        const list=selector==='[data-row] [data-row-open]'?(element.closest('[data-list]')??document.documentElement):document.documentElement;
        groups.set(list,[...(groups.get(list)??[]),element]);
      });
      for(const rows of groups.values())rows.forEach((element,index)=>{if(index>0)disable(element,true);});
    }
    document.querySelectorAll<HTMLElement>('[data-tab-open]').forEach(button=>{if(button.closest('[data-tab]')?.getAttribute('data-on')!=='true')disable(button,true);});
    document.querySelectorAll<HTMLElement>('[data-file-path]').forEach(file=>{if(file.dataset.filePath!==roots.web+'/README.md')disable(file,true);});
    document.querySelectorAll<HTMLTextAreaElement>('.xterm-helper-textarea').forEach(input=>{input.readOnly=true;});
  };
  const observer=new MutationObserver(()=>{if(!pending){pending=true;queueMicrotask(apply);}});
  observer.observe(document.body,{childList:true,subtree:true});
  document.addEventListener('contextmenu',event=>{event.preventDefault();event.stopImmediatePropagation();},true);
  document.addEventListener('click',event=>{
    const target=event.target as Element;
    if(target.closest('[data-site-unavailable]')){event.preventDefault();event.stopImmediatePropagation();return;}
    const button=target.closest('[data-tab-open]');
    if(button?.closest('[data-tab]')?.getAttribute('data-on')==='true'){event.preventDefault();event.stopImmediatePropagation();}
  },true);
  document.addEventListener('dragstart',event=>{event.preventDefault();event.stopImmediatePropagation();},true);
  document.addEventListener('keydown',event=>{
    const element=event.target as HTMLElement;
    const editing=element.matches('input,textarea,[contenteditable="true"]');
    const fixedNavigation=!editing&&element.closest('[data-tab-open],[data-list]')&&['ArrowLeft','ArrowRight','ArrowUp','ArrowDown','Home','End'].includes(event.key);
    if(fixedNavigation||element.closest('[data-terminal]')||((event.metaKey||event.ctrlKey)&&['n','o','k',',','Tab','[',']'].includes(event.key))||(!editing&&['Delete','F2'].includes(event.key))){event.preventDefault();event.stopImmediatePropagation();}
  },true);
  apply();
  window.addEventListener('pagehide',()=>observer.disconnect(),{once:true});
}
