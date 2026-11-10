import {useEffect,useState} from 'react';
import {createRoot} from 'react-dom/client';
import {Button} from '../../src/components/ui/button';

type Driver={ready:()=>boolean;trigger:(burst:boolean)=>string[]};
function NoticeTools({driver}:{driver:Driver}){
  const [ready,setReady]=useState(driver.ready),[result,setResult]=useState('');
  useEffect(()=>{
    if(ready)return;
    const timer=setInterval(()=>{if(driver.ready()){setReady(true);clearInterval(timer);}},250);
    return()=>clearInterval(timer);
  },[ready,driver]);
  const trigger=(burst:boolean)=>{
    const targets=driver.trigger(burst);
    setResult(targets.length?`正在触发：${targets.join('、')}`:'没有可通知的其他会话，检查静音设置');
  };
  return <aside data-lab-notice-tools aria-label="Lab 通知演示" className="pointer-events-none fixed inset-x-row z-30 flex justify-center"
    style={{bottom:'calc(var(--nav-room) + var(--gap-row))'}}>
    <div className="glass pointer-events-auto flex items-center gap-in rounded-full px-row py-in">
      <span className="text-caption text-muted-foreground">Lab · 通知</span>
      <Button className="min-h-touch" data-lab-notice-trigger disabled={!ready} onClick={()=>trigger(false)}>再次触发</Button>
      <Button className="min-h-touch" data-lab-notice-burst disabled={!ready} onClick={()=>trigger(true)}>汇总</Button>
      <output data-lab-notice-result className="sr-only" aria-live="polite">{result}</output>
    </div>
  </aside>;
}
export function mountNoticeTools(driver:Driver){
  const host=document.createElement('div');host.dataset.labNoticeHost='';document.body.append(host);
  const root=createRoot(host);root.render(<NoticeTools driver={driver}/>);
  window.addEventListener('pagehide',()=>{root.unmount();host.remove();},{once:true});
}
