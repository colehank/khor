import {createRoot} from 'react-dom/client';
import {useEffect,useRef,useState} from 'react';
import {PageField} from '@/browse/PageField';
import {PathField} from '@/components/PathField';
import {ChoiceField} from '@/components/ui/choice-field';
import {Omnibox} from '@/components/Omnibox';
import {TooltipProvider} from '@/components/ui/tooltip';
import {Button} from '@/components/ui/button';
import {followFieldSwipe} from '@/lib/field-swipe';
import {useSheetPan} from '@/hooks/use-sheet-pan';
import type {DeviceRow} from '@/api';
import {speakLang} from '@/lang';
import '@/styles/tokens.css';import '@/styles/app.css';import './suggestion-interaction.css';
speakLang('zh');
const device={id:'fixture',name:'fixture',shown:'测试机器',me:true,face:null,hop:null,paused:false} as DeviceRow;
function Lab(){
 const[kind,setKind]=useState('address'),[value,setValue]=useState('~/'),[query,setQuery]=useState(''),[seat,setSeat]=useState<HTMLDivElement|null>(null),[surface,setSurface]=useState<HTMLDivElement|null>(null),[detent,setDetent]=useState<'half'|'full'>('full'),[result,setResult]=useState(''),[count,setCount]=useState(0),[opened,setOpened]=useState('');
 const output=useRef<HTMLOutputElement>(null);
 useEffect(()=>followFieldSwipe(),[]);
 useSheetPan({element:surface,onDetent:setDetent,onClose:()=>setOpened('sheet closed'),geometry:()=>({half:340,full:600})});
 const pan=(outside=false)=>{const menu=document.querySelector('[data-menu-viewport]'),target=(outside?document.querySelector('[data-probe-body]'):menu?.querySelector('button')||menu) as HTMLElement|null;if(!target)return;const before=document.activeElement;let prevented=false;for(const[type,y]of[['touchstart',120],['touchmove',200],['touchend',200]]as const){const e=new Event(type,{bubbles:true,cancelable:true});Object.defineProperty(e,'touches',{value:type==='touchend'?[]:[{clientX:100,clientY:y}]});target.dispatchEvent(e);prevented ||=e.defaultPrevented;}const report={kind,area:outside?'body':'menu',focusedBefore:before?.tagName,focusKept:before===document.activeElement,prevented,sheetDragged:surface?.hasAttribute('data-sheet-pulling')||surface?.hasAttribute('data-sheet-dragged')};setResult(JSON.stringify(report,null,2));};
 return <TooltipProvider><main className="suggestion-lab"><aside><h1>输入推荐交互</h1><p>正式地址／目录／选择组件与全局手势监听</p>{[['address','网址（推荐在独立座位）'],['path','目录'],['choice','机器与启动方式'],['omni','搜索补全']].map(([id,name])=><Button key={id} onClick={()=>setKind(id)}>{name}</Button>)}<Button onMouseDown={e=>e.preventDefault()} onClick={()=>pan()}>下滑推荐列表</Button><Button onMouseDown={e=>e.preventDefault()} onClick={()=>pan(true)}>下滑正文</Button><Button onMouseDown={e=>e.preventDefault()} onClick={()=>setCount(n=>n+1)}>保留焦点的外部操作</Button><output ref={output} data-suggestion-proof>{result}</output><p>已打开：{opened||'无'} · 外部操作：{count}</p></aside><section ref={setSurface} data-detent={detent} data-suggestion-sheet className="suggestion-sheet"><div data-probe-body className="suggestion-body" onClick={()=>setCount(n=>n+1)}><p>点这里是输入框外的普通区域</p><p>外部操作应在同一次点击生效</p></div><div ref={setSeat}/><div className="suggestion-field" key={kind}>{kind==='address'?<PageField tab="fixture" machine={device} devices={[device,{...device,id:'remote',name:'remote',shown:'另一台机器',me:false,hop:'direct'}]} at="https://example.org/" onGo={url=>setOpened(url)} steps={false} listSeat={seat} inPill/>:kind==='path'?<PathField aria-label="目录" machine="fixture" value={value} onValueChange={setValue}/>:kind==='choice'?<ChoiceField label="机器" value={query} options={Array.from({length:20},(_,i)=>({id:'machine-'+i,label:'机器 '+(i+1)}))} onPick={setQuery}/>:<Omnibox label="搜索" query={query} onQuery={setQuery} axes={[{key:'names',label:'推荐',candidates:Array.from({length:20},(_,i)=>({key:'item-'+i,label:'候选 '+(i+1)}))}]} chosen={[]} onToggle={setOpened}/>}</div></section></main></TooltipProvider>;
}
const root=createRoot(document.getElementById('root')!);root.render(<Lab/>);if(import.meta.hot)import.meta.hot.dispose(()=>root.unmount());
