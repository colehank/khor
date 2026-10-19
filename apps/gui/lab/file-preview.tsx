import {createRoot} from 'react-dom/client';
import {useState} from 'react';
import {FilePage} from '@/views/FilePage';
import {TooltipProvider} from '@/components/ui/tooltip';
import {Button} from '@/components/ui/button';
import {speakLang} from '@/lang';
import '@/styles/tokens.css';import '@/styles/app.css';import './file-preview.css';
speakLang('zh');
const wait=()=>new Promise<void>(r=>requestAnimationFrame(()=>requestAnimationFrame(()=>r())));
function Lab(){
 const[report,setReport]=useState(''),[embedded,setEmbedded]=useState(false);
 const inspect=()=>{const p=document.querySelector<HTMLElement>('[data-file-page]')!,s=p.querySelector<HTMLElement>('[data-file-preview],[data-file-scroll]');setReport(JSON.stringify({face:p.dataset.fileFace,folded:p.hasAttribute('data-folded'),top:s?.scrollTop,editor:!!p.querySelector('.cm-editor'),preview:!!p.querySelector('[data-file-preview]')},null,2));};
 const middle=async()=>{const p=document.querySelector<HTMLElement>('[data-file-page]')!,s=p.querySelector<HTMLElement>('[data-file-preview],[data-file-scroll]')!;s.scrollTop=1200;await wait();p.querySelector<HTMLButtonElement>('[data-deck-strip]')?.click();await wait();inspect();};
 const pan=async()=>{const p=document.querySelector<HTMLElement>('[data-file-page]')!,s=p.querySelector<HTMLElement>('[data-file-preview],[data-file-scroll]')!;const contact={identifier:1,clientX:100,clientY:300,target:s};const fire=(name:string,y:number)=>{const e=new Event(name,{bubbles:true});Object.defineProperty(e,'touches',{value:name==='touchend'?[]:[{...contact,clientY:y}]});s.dispatchEvent(e);};fire('touchstart',300);fire('touchmove',220);s.scrollTop+=120;fire('touchend',220);await wait();inspect();};
 return <TooltipProvider><main className="file-preview-lab"><aside><h1>文件预览与药丸</h1><p>真实 FilePage，传输层提供本地只读夹具</p><Button onClick={()=>void middle()}>定位中段并展开药丸</Button><Button onClick={()=>void pan()}>模拟向下阅读</Button><Button onClick={inspect}>检查状态</Button><Button onClick={()=>setEmbedded(!embedded)}>桌面内嵌／手机甲板</Button><output data-file-proof>{report}</output></aside><section className="file-preview-phone"><FilePage key={String(embedded)} embedded={embedded} machine="fixture" path="/notes.md" here={false} onBack={()=>{}} onGone={()=>{}} onMoved={()=>{}}/></section></main></TooltipProvider>;
}
const root=createRoot(document.getElementById('root')!);root.render(<Lab/>);if(import.meta.hot)import.meta.hot.dispose(()=>root.unmount());
