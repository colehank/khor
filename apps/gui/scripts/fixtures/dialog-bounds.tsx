/** Production components with disposable data; served only by check-dialogs. */
import React, {useState} from 'react';
import {createRoot} from 'react-dom/client';
import {NewSessionPage} from '../../src/views/NewSessionPage';
import {TerminalThemeSection} from '../../src/views/more/TerminalThemeSection';
import {ImportDialog} from '../../src/views/ImportDialog';
import {Dialog, DialogContent, TaskOverlay, DialogBody, DialogTitle, DialogFooter} from '../../src/components/ui/dialog';
import {Button} from '../../src/components/ui/button';
import {DirPicker} from '../../src/components/DirPicker';
import {PathField} from '../../src/components/PathField';
import {OmniMenu} from '../../src/components/OmniMenu';
import {useContentSheet} from '../../src/hooks/use-content-sheet';
import {SettingsProvider} from '../../src/settings';
import {TooltipProvider} from '../../src/components/ui/tooltip';
import {speakLang} from '../../src/lang';
import '../../src/styles/tokens.css';
import '../../src/styles/app.css';
speakLang('zh');
new ResizeObserver(()=>document.documentElement.toggleAttribute('data-narrow',innerWidth<760)).observe(document.documentElement);

/** Synthetic touch sequences exercise the production listeners, not a copy
 * of their ownership rules. Buttons make the exact same probe available to
 * CUA and the regression gate; native momentum is a separate device check. */
function pan(target: Element, up: boolean) {
  const bounds=target.getBoundingClientRect(), x=bounds.left+20, y=bounds.top+20;
  let prevented=false;
  for(const [phase,delta] of [['touchstart',0],['touchmove',up?-260:260],['touchend',up?-260:260]] as const) {
    const touch=new Touch({identifier:1,target,clientX:x,clientY:y+delta});
    const event=new TouchEvent(phase,{bubbles:true,cancelable:true,touches:phase==='touchend'?[]:[touch],changedTouches:[touch]});
    target.dispatchEvent(event);
    prevented ||= event.defaultPrevented;
  }
  return prevented;
}
function ContentPan() {
  const [el,setEl]=useState<HTMLDivElement|null>(null), [result,setResult]=useState(''), [closed,setClosed]=useState(false);
  useContentSheet(el, d=>setResult('detent:'+d), ()=>setClosed(true),()=>({height:760,foot:0}));
  const test=(menu:boolean)=>{
    if(!el)return;
    const target=el.querySelector(menu?'[data-menu-viewport] button':'[data-pan-body]')!;
    const prevented=pan(target,true);
    const dragged=el.hasAttribute('data-sheet-dragged');
    setResult(JSON.stringify({prevented,dragged,closed}));
  };
  return <><Button onClick={()=>test(true)}>上滑候选菜单</Button><Button onClick={()=>test(false)}>上滑主内容</Button><output data-pan-result>{result}</output><div ref={setEl} data-pan-sheet data-detent="half" style={{height:380,background:'var(--background)',overflow:'hidden',padding:20}}><h2 data-pan-body>内容底栏</h2><OmniMenu rows={{at:-1,groups:[{key:'paths',label:'',role:'text',candidates:Array.from({length:30},(_,i)=>({key:String(i),label:'目录 '+(i+1)}))}]}} onPick={()=>{}}/></div></>;
}
function TaskPan({open,onClose}:{open:boolean;onClose:()=>void}) {
  const[result,setResult]=useState('');
  const test=(menu:boolean,up:boolean)=>{
    const root=document.querySelector<HTMLElement>('[data-task-pan]')!;
    const target=root.querySelector(menu?'[data-menu-viewport] button':'[data-task-main]')!;
    const before=root.getBoundingClientRect().height,prevented=pan(target,up);
    setResult(JSON.stringify({before,prevented,dragged:root.hasAttribute('data-sheet-dragged')}));
  };
  return <Dialog open={open} onOpenChange={on=>{if(!on)onClose()}}><TaskOverlay purpose="selection" scrollable data-task-pan><DialogTitle>选择任务</DialogTitle><DialogBody><div data-task-main>主内容</div><OmniMenu rows={{at:-1,groups:[{key:'choices',label:'',role:'text',candidates:Array.from({length:30},(_,i)=>({key:String(i),label:'候选 '+(i+1)}))}]}} onPick={()=>{}}/><output data-task-pan-result>{result}</output></DialogBody><DialogFooter><Button onClick={()=>test(true,true)}>上滑菜单</Button><Button onClick={()=>test(false,true)}>上滑内容</Button><Button onClick={()=>test(false,false)}>下滑内容</Button><Button onClick={onClose}>取消</Button></DialogFooter></TaskOverlay></Dialog>;
}
function Proof(){
  const[mode,setMode]=useState('new'),[question,setQuestion]=useState(false),[imports,setImports]=useState(false),[pathOpen,setPathOpen]=useState(false),[path,setPath]=useState('~/'),[result,setResult]=useState(''),[directory,setDirectory]=useState(false),[task,setTask]=useState(false);
  return <SettingsProvider value={null}><TooltipProvider><main style={{height:'100dvh',padding:'var(--gap-pane)',color:'var(--foreground)'}}>
    <nav style={{display:'flex',gap:8,flexWrap:'wrap'}}><Button onClick={()=>setMode('new')}>新建页面</Button><Button onClick={()=>setImports(true)}>导入会话</Button><Button onClick={()=>setQuestion(true)}>确认弹窗</Button><Button onClick={()=>setPathOpen(true)}>路径表单</Button><Button onClick={()=>setMode('pan')}>手势检查</Button><Button onClick={()=>setMode('theme')}>终端配色</Button><Button onClick={()=>setDirectory(true)}>目录选择</Button><Button onClick={()=>setTask(true)}>任务手势</Button></nav>
    {mode==='new'?<NewSessionPage dir="~/" onCancel={()=>{}} onOpened={()=>{}}/>:mode==='theme'?<TerminalThemeSection/>:<ContentPan/>}
    <TaskPan open={task} onClose={()=>setTask(false)}/>
    <DirPicker open={directory} onOpenChange={setDirectory} machine="fixture" fixed onPick={()=>{}}/>
    <ImportDialog open={imports} onOpenChange={setImports} onImported={()=>{}}/>
    <Dialog open={question} onOpenChange={setQuestion}><DialogContent><DialogTitle>删除启动方式？</DialogTitle><p>测试条目</p><DialogFooter><Button onClick={()=>setQuestion(false)}>取消</Button><Button onClick={()=>setQuestion(false)}>删除</Button></DialogFooter></DialogContent></Dialog>
    <Dialog open={pathOpen} onOpenChange={setPathOpen}><TaskOverlay purpose="edit" data-pan-form><DialogTitle>选择目录</DialogTitle><PathField machine="fixture" value={path} onValueChange={setPath}/><output data-form-pan-result>{result}</output><DialogFooter><Button onMouseDown={e=>e.preventDefault()} onClick={()=>{
      const root=document.querySelector('[data-pan-form]')!, target=root.querySelector('[data-menu-viewport] button')!;
      const prevented=pan(target,false);setResult(JSON.stringify({prevented,pulling:root.hasAttribute('data-sheet-pulling'),dragged:root.hasAttribute('data-sheet-dragged'),pull:(root as HTMLElement).style.getPropertyValue('--sheet-pull')}));
    }}>下滑推荐菜单</Button><Button onClick={()=>setPathOpen(false)}>取消</Button></DialogFooter></TaskOverlay></Dialog>
  </main></TooltipProvider></SettingsProvider>;
}
createRoot(document.getElementById('root')!).render(<Proof/>);
