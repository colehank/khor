// Real editable CodeEditor inside the production sheet hook. The control
// dispatches a reproducible touch sequence; ordinary editing stays live.
import {useState} from 'react';
import {createRoot} from 'react-dom/client';
import {CodeEditor} from '@/components/CodeEditor';
import {useContentSheet} from '@/hooks/use-content-sheet';
import type {PlaceDetent} from '@/views/SessionPlaceSheet';
import '@/styles/tokens.css';
import '@/styles/app.css';
import './content-pan.css';
const text=Array.from({length:200},(_,n)=>`const line${n+1} = "纵向拖动与文字编辑 ${n+1}";`).join('\n');
function Lab(){
 const [sheet,setSheet]=useState<HTMLDivElement|null>(null),[detent,setDetent]=useState<PlaceDetent>('half'),[result,setResult]=useState('尚未运行'),[closed,setClosed]=useState(false);
 useContentSheet(sheet,setDetent,()=>setClosed(true),()=>({height:800,foot:0}));
 const run=async()=>{
  if(!sheet)return;
  const log:string[]=[],check=(yes:boolean,word:string)=>{if(!yes)throw Error(word);log.push('PASS '+word);};
  const frame=()=>new Promise<void>(r=>requestAnimationFrame(()=>requestAnimationFrame(()=>r())));
  const touch=(type:string,y:number,target:Element,x=100)=>{const e=new Event(type,{bubbles:true,cancelable:true});Object.defineProperty(e,'touches',{value:type==='touchend'||type==='touchcancel'?[]:[{identifier:1,target,clientX:x,clientY:y}]});target.dispatchEvent(e);};
  const code=sheet.querySelector('.cm-content')!,scroll=sheet.querySelector<HTMLElement>('.cm-scroller')!,height=()=>sheet.getBoundingClientRect().height;
  try{
   setDetent('half');setClosed(false);window.getSelection()?.removeAllRanges();await frame();scroll.scrollTop=120;await frame();
   const start=height();touch('touchstart',400,code);touch('touchmove',320,code);
   check(Math.abs(height()-start-80)<2,'可编辑代码区上滑即扩展 80');
   check(scroll.scrollTop===120,'展开保留代码滚动位置 '+scroll.scrollTop);touch('touchcancel',320,code);check(!sheet.style.height,'取消清理中间高度');
   touch('touchstart',400,code);touch('touchmove',40,code);touch('touchend',40,code);await frame();check(sheet.dataset.detent==='full','同一次上滑停靠展开');
   scroll.scrollTop=100;touch('touchstart',200,code);touch('touchmove',230,code);check(!sheet.hasAttribute('data-sheet-pulling'),'满高下拉先滚内容');scroll.scrollTop=0;touch('touchmove',232,code);check(sheet.hasAttribute('data-sheet-pulling'),'到顶后同一次手势收起');touch('touchcancel',232,code);
   const view=(sheet.querySelector('[data-code-view]') as HTMLElement & {__view: import('@codemirror/view').EditorView}).__view;view.dispatch({selection:{anchor:view.state.doc.length}});view.focus();scroll.scrollTop=0;await frame();
   touch('touchstart',300,code);touch('touchmove',586,code);await frame();check(scroll.scrollTop===0,'收起时不跳回旧光标');touch('touchend',586,code);await frame();check(sheet.dataset.detent==='half'&&scroll.scrollTop===0,'停靠半高后不跳回旧光标');view.dispatch({selection:{anchor:0}});view.contentDOM.blur();
   const input=sheet.querySelector('input')!,h=height();touch('touchstart',300,input);touch('touchmove',420,input);check(height()===h,'普通输入框保留手势');touch('touchcancel',420,input);
   touch('touchstart',300,code);touch('touchmove',304,code,180);check(!sheet.hasAttribute('data-sheet-pulling'),'横向拖动保留给内容');touch('touchcancel',304,code);
   const node=document.createTreeWalker(code.querySelector('.cm-line')!,NodeFilter.SHOW_TEXT).nextNode()!,selection=window.getSelection()!,range=document.createRange();range.setStart(node,0);range.setEnd(node,Math.min(3,node.textContent!.length));selection.removeAllRanges();selection.addRange(range);
   touch('touchstart',300,code);touch('touchmove',420,code);check(!sheet.hasAttribute('data-sheet-pulling'),'选中文字时保留选区操作');touch('touchcancel',420,code);selection.removeAllRanges();
   setResult(log.join('\n'));
  }catch(error){touch('touchcancel',0,code);setResult([...log,'FAIL '+String(error)].join('\n'));}
 };
 return <main className="pan-lab"><header><button onClick={()=>void run()}>运行真实代码区回归</button><button onClick={()=>{setDetent('half');setClosed(false)}}>恢复半高</button><output>{detent}{closed?' · 已关闭':''}</output></header><pre data-pan-results>{result}</pre><div ref={setSheet} className="pan-sheet" data-detent={detent} data-content-panel><div data-place-grab className="pan-grab"/><input aria-label="普通输入框" placeholder="输入框保留编辑手势"/><div className="pan-editor"><CodeEditor path="gesture.ts" text={text} editable/></div></div></main>;
}
createRoot(document.getElementById('root')!).render(<Lab/>);
