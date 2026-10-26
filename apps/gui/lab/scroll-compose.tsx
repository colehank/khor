import {useLayoutEffect,useRef,useState} from 'react';
import {createRoot} from 'react-dom/client';
import {flushSync} from 'react-dom';
import {BarSeat,SessionScreen} from '@/components/SessionBar';
import {Composer,COMPOSER_TEXT,useGrowingText} from '@/views/Composer';
import {useChatScroll} from '@/hooks/use-chat-scroll';
import '@/styles/tokens.css';
import '@/styles/app.css';
import '@/styles/session-layout.css';

type Sample={reason:string;at:number;top:number;view:number;total:number;gap:number;cap:number;row:number;field:number;styleHeight:string;expanded:boolean;active:string;first:string|null;firstOffset:number|null};
let readings:Sample[]=[];
function snapshot(reason:string):Sample {
  const box=document.querySelector<HTMLElement>('[data-compose-scroll]')!;
  const field=document.querySelector<HTMLTextAreaElement>('[data-composer-text]')!;
  const cap=field.closest<HTMLElement>('[data-chat-box]')!;
  const bounds=box.getBoundingClientRect();
  const first=[...box.querySelectorAll('p')].find(node=>node.getBoundingClientRect().bottom>bounds.top);
  const value={reason,at:performance.now(),top:box.scrollTop,view:box.clientHeight,total:box.scrollHeight,
    gap:box.scrollHeight-box.clientHeight-box.scrollTop,cap:cap.getBoundingClientRect().height,
    row:cap.parentElement!.getBoundingClientRect().height,field:field.getBoundingClientRect().height,
    styleHeight:field.style.height,expanded:cap.hasAttribute('data-lab-compose-expanded'),
    active:document.activeElement?.tagName??'',first:first?.textContent??null,
    firstOffset:first?first.getBoundingClientRect().top-bounds.top:null};
  readings.push(value);return value;
}

// The optional diagnostic probe observes the existing scrollHeight read.
// It returns the original getter's exact value and never writes layout.
if(new URLSearchParams(location.search).get('probe')==='1'){
  const original=Object.getOwnPropertyDescriptor(Element.prototype,'scrollHeight')!;
  let observing=false;
  Object.defineProperty(Element.prototype,'scrollHeight',{...original,get:function(this:Element){
    const value=original.get!.call(this);
    if(!observing&&this.matches('[data-composer-text]')&&document.querySelector('[data-compose-scroll]')){
      observing=true;try{snapshot('production-field-scrollHeight-read');}finally{observing=false;}
    }
    return value;
  }});
}

function Conversation({revision,epoch}:{revision:object;epoch:number}){
  const [draft,setDraft]=useState('');
  const field=useRef<HTMLTextAreaElement>(null);
  const {scroller,column,onScroll,toBottom}=useChatScroll(revision);
  useGrowingText(field,draft);
  useLayoutEffect(()=>{
    if(field.current&&document.querySelector('[data-chat-box]'))snapshot('conversation-layout-effect');
  });
  useLayoutEffect(()=>{window.__composeFixture.toBottom=toBottom;},[toBottom]);
  return <div data-compose-epoch={epoch} className="flex min-h-0 flex-1 flex-col">
    <div className="relative flex min-h-0 flex-1 flex-col">
      <div ref={scroller} data-compose-scroll onScroll={onScroll} className="min-h-0 flex-1 overflow-y-auto px-4 py-3 [overflow-anchor:none]">
        <div ref={column} className="mx-auto w-full max-w-measure space-y-2 text-read">
          {Array.from({length:60},(_,i)=><p key={i}>第 {i+1} 段　输入框测量不应让阅读位置离开末尾。读历史时，保持这一段的位置。</p>)}
        </div>
      </div>
    </div>
    <BarSeat at="above"><div className="flex-none px-row pt-in pb-in"><div className="relative mx-auto flex w-full max-w-measure flex-col text-read">
      <Composer lead={<button type="button" data-lab-add className="size-composer-item" aria-label="附件">＋</button>}
        trail={<button type="button" data-chat-send className="size-composer-item" aria-label="发送">↑</button>}>
        <textarea ref={field} data-composer-text rows={1} value={draft} onChange={event=>setDraft(event.target.value)} className={COMPOSER_TEXT}/>
      </Composer>
    </div></div></BarSeat>
  </div>;
}

type Fixture={toBottom:()=>void;snapshot:typeof snapshot;rerender:(changeRevision:boolean)=>Sample;observe:(ms:number)=>Promise<Sample[]>;readings:()=>Sample[]};
declare global{interface Window{__composeFixture:Fixture}}
window.__composeFixture={toBottom:()=>{},snapshot,rerender:()=>{throw new Error('not mounted')},observe:async()=>[],readings:()=>readings};

function Fixture(){
  const [epoch,setEpoch]=useState(0),[revision,setRevision]=useState<object>({});
  useLayoutEffect(()=>{
    window.__composeFixture.rerender=(changeRevision:boolean)=>{
      readings=[];
      snapshot('before-parent-render');
      flushSync(()=>{setEpoch(value=>value+1);if(changeRevision)setRevision({});});
      return snapshot('after-parent-render');
    };
    window.__composeFixture.observe=async(ms:number)=>{
      const start=performance.now();
      do{await new Promise(requestAnimationFrame);snapshot('animation-frame');}while(performance.now()-start<ms);
      return readings;
    };
  });
  return <SessionScreen deckAt="top" navigation faces={[]} active="q010-compose" onFocus={()=>{}} onClose={()=>{}} onBack={()=>{}}
    renderHeader={({children})=><div style={{height:60,flex:'none'}}>{children}</div>}>
    <Conversation revision={revision} epoch={epoch}/>
  </SessionScreen>;
}
document.documentElement.dataset.theme='light';
createRoot(document.getElementById('root')!).render(<Fixture/>);
