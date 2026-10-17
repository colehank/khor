import {useEffect,useState} from 'react';
import {createRoot} from 'react-dom/client';
import {PathField} from '@/components/PathField';
import {OmniMenu} from '@/components/OmniMenu';
import {Button} from '@/components/ui/button';
import {DropdownMenu,DropdownMenuTrigger,DropdownMenuContent,DropdownMenuItem} from '@/components/ui/dropdown-menu';
import {ContextMenu,ContextMenuTrigger,ContextMenuContent,ContextMenuItem} from '@/components/ui/context-menu';
import {speakLang} from '@/lang';
import '@/styles/tokens.css';
import '@/styles/app.css';
import './session-launch.css';
speakLang('zh');
const directories=['Applications','Desktop','Documents','Downloads','Library','Movies','Music','Pictures','Projects','Public','Research',...Array.from({length:24},(_,i)=>'project-'+String(i+1).padStart(2,'0')),'最后一个目录'];
const baseFetch=window.fetch.bind(window);
window.fetch=async(input,init)=>{const url=new URL(input instanceof Request?input.url:String(input),location.href);if(url.pathname==='/api/ls'){return new Response(JSON.stringify({path:'/Users/zgh',entries:directories.map(name=>({name,dir:true,bytes:0,mtime:0})),truncated:new URLSearchParams(location.search).has('cut')}),{headers:{'Content-Type':'application/json'}})}return baseFetch(input,init)};
function PickerLab(){
 const [mode,setMode]=useState('path'),[value,setValue]=useState('/Users/zgh/'),[picked,setPicked]=useState(''),[light,setLight]=useState(false);
 useEffect(()=>{const q=matchMedia('(max-width:48rem)'),sync=()=>document.documentElement.toggleAttribute('data-narrow',q.matches);sync();q.addEventListener('change',sync);return()=>q.removeEventListener('change',sync)},[]);
 useEffect(()=>{document.documentElement.dataset.theme=light?'light':'dark'},[light]);
 return <main className="launch-lab"><header className="lab-tools"><span className="lab-label">长列表选择 <span>Lab</span></span><Button variant="ghost" onClick={()=>setLight(!light)}>{light?'深色':'浅色'}</Button></header><div className="flex flex-wrap gap-in">{[['path','目录推荐'],['omni','通用候选'],['drop','操作菜单'],['context','右键菜单']].map(([id,name])=><Button key={id} onClick={()=>setMode(id)} aria-pressed={mode===id}>{name}</Button>)}</div><section className="mx-auto w-full max-w-measure py-pane"><p className="mb-base text-callout text-muted-foreground">测试数据含 36 项，最后一项为「最后一个目录」</p>{mode==='path'?<PathField autoFocus machine="" value={value} onValueChange={v=>{setValue(v);setPicked(v)}}/>:mode==='omni'?<OmniMenu rows={{at:-1,groups:[{key:'all',label:'',candidates:directories.map(n=>({key:n,label:n}))}]}} onPick={setPicked}/>:mode==='drop'?<DropdownMenu><DropdownMenuTrigger asChild><Button>选择启动方式</Button></DropdownMenuTrigger><DropdownMenuContent>{directories.map(n=><DropdownMenuItem key={n} onSelect={()=>setPicked(n)}>{n}</DropdownMenuItem>)}</DropdownMenuContent></DropdownMenu>:<ContextMenu><ContextMenuTrigger asChild><div className="rounded-lg border p-pane text-body">在这里点右键</div></ContextMenuTrigger><ContextMenuContent>{directories.map(n=><ContextMenuItem key={n} onSelect={()=>setPicked(n)}>{n}</ContextMenuItem>)}</ContextMenuContent></ContextMenu>}<p className="mt-pane text-callout text-muted-foreground" aria-live="polite">{picked?'已选择：'+picked:'仅示意，不读取真实目录'}</p></section></main>
}
createRoot(document.getElementById('root')!).render(<PickerLab/>);
