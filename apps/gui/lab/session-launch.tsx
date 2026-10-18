// The production page under its actual wide strip / narrow bottom deck.
// All API answers below are closed fixtures. No command reaches a khor node.
import {useEffect,useState} from 'react';
import {createRoot} from 'react-dom/client';
import {DirPicker} from '@/components/DirPicker';
import {ImportDialog} from '@/views/ImportDialog';
import {NewSessionPage} from '@/views/NewSessionPage';
import {SessionScreen,TabSheetRows} from '@/components/SessionBar';
import {TabStrip,type TabFace} from '@/browse/TabStrip';
import {newTabKind} from '@/components/SessionFace';
import {Button} from '@/components/ui/button';
import {PopupButton} from '@/components/ui/popup-button';
import {TooltipProvider} from '@/components/ui/tooltip';
import {NARROW_QUERY,useNarrow} from '@/hooks/use-narrow';
import {followVisualViewport} from '@/lib/viewport';
import {SettingsProvider} from '@/settings';
import type {Avatar} from '@/gen/bindings/Avatar';
import type {ConfigRow,LaunchPreset} from '@/api';
import {speakLang} from '@/lang';
import '@/styles/tokens.css';
import '@/styles/app.css';
import './session-launch.css';

speakLang('zh');
const face=(background:string,one:string,two:string):Avatar=>({canvas:80,background,radius_ratio:.5,mark:{ink:'light',veil:0},paint:{variant:'marble',blur:7,veins:[{vein:'shard',scale:1.2,rotate:20,dx:3,dy:-3,overlay:false,color:one},{vein:'sweep',scale:1.4,rotate:200,dx:-4,dy:6,overlay:true,color:two}]}});
const faces=[face('#5f6b2a','#c5c8a6','#a6b87a'),face('#2a4f6b','#8abeb7','#81a2be'),face('#6b3a2a','#f0c674','#cc6666')];
const devices=['MacLab','turing','Studio','hinton','lecun','gpu-01','gpu-02','gpu-03','lab-server','nas','build-node','devbox','test-linux','research-a','research-b'].map((name,i)=>({id:'device-'+i,name,shown:name,label:null,me:i===0,hosts:true,sponsor:null,face:faces[i%3],hop:i===3?'quiet':'direct',heard_ms:i===3?600000:0,down_ms:i===3?60000:null,paused:false,home:i===0?'/Users/zgh':i===2?'C:\\Users\\zgh':'/home/zgh'}));
const candidates=devices.slice(0,4).flatMap((d,i)=>Array.from({length:6},(_,n)=>({id:'shell/import-'+i+'-'+n,form:'shell',machine:d.name,device:d.id,title:['research','khor','notes','analysis','daily','project'][n],place:d.home+'/Projects',at_ms:Date.now()-n*60000,category:null,agent:null,command:n===0?'codex':'zsh'})));
const STORE='khor.lab.launch-presets.v1';
let presets:LaunchPreset[];
try{presets=JSON.parse(localStorage.getItem(STORE)||'null')||[{id:'research',name:'科研环境',command:'tmux new-session -A -s research'}]}catch{presets=[]}
let opened:(id:string,args:Record<string,unknown>)=>void=()=>{};
let serial=0;
const json=(value:unknown)=>new Response(JSON.stringify(value),{headers:{'Content-Type':'application/json'}});
window.fetch=async(input,init)=>{
 const url=new URL(input instanceof Request?input.url:String(input),location.href);
 const cmd=url.pathname.replace('/api/','');
 const args=input instanceof Request?await input.clone().json():JSON.parse(String(init?.body||'{}'));
 if(cmd==='devices')return json(devices);
 if(cmd==='import_candidates')return json(candidates);
 if(cmd==='import_sessions')return json(args.ids);
 if(cmd==='agent_seen')return json({said:'unasked'});
 if(cmd==='agents')return json([{name:'custom-agent',command:'my-agent --acp'}]);
 if(cmd==='launch_presets')return json(presets);
 if(cmd==='launch_preset_save'){
  const p=args.preset as LaunchPreset;
  if(!p.name.trim()||!p.command.trim())return new Response('填写名称和启动命令',{status:400});
  if(presets.some(old=>old.id!==p.id&&old.name===p.name))return new Response('这个启动方式名称已存在',{status:400});
  presets=[...presets.filter(old=>old.id!==p.id),p];localStorage.setItem(STORE,JSON.stringify(presets));return json(presets);
 }
 if(cmd==='launch_preset_delete'){presets=presets.filter(p=>p.id!==args.id);localStorage.setItem(STORE,JSON.stringify(presets));return json(presets)}
 if(cmd==='ls'){
  const m=devices.find(d=>d.name===args.machine||d.id===args.machine)||devices[0];
  const norm=(s:string)=>s.replace(/\\/g,'/').replace(/\/+$/,'');
  const path=String(args.path||m.home),home=norm(m.home),here=norm(path);
  const names=here===home?['Desktop','Documents','Downloads','Library','Movies','Music','Pictures','Projects','Public','Research',...Array.from({length:24},(_,i)=>'project-'+String(i+1).padStart(2,'0'))]:here===home+'/Desktop'?['ACL2027','khor']:here===home+'/Projects'?['khor','notes']:here.endsWith('/khor')?['apps','crates','docs','scripts','target','vendor']:[];
  return json({path,entries:names.map(name=>({name,dir:true,bytes:0,mtime:0})),truncated:false});
 }
 if(cmd==='open_shell'||cmd==='open_session'){const id=(cmd==='open_shell'?'shell/':'agent/')+'lab-'+(++serial);opened(id,args);return json(id)}
 if(cmd==='agents_known')return json([]);
 if(cmd==='agents_known_taken')return json(['fixture',0,0]);
 return new Response('Lab does not implement '+cmd,{status:400});
};
const settings=[{name:'session-form',value:'term'}] as ConfigRow[];
function Lab(){
 const [phone,setPhone]=useState(new URLSearchParams(location.search).get('view')==='phone'),[light,setLight]=useState(new URLSearchParams(location.search).get('theme')==='light'),[entry,setEntry]=useState('global');
 const [active,setActive]=useState('new-tab:lab'),[draft,setDraft]=useState(true),[epoch,setEpoch]=useState(0),[list,setList]=useState(false);
 const [dialog,setDialog]=useState<'workspace'|'import'|null>(null),[pickedDirectory,setPickedDirectory]=useState('');
 const [created,setCreated]=useState<{id:string;args:Record<string,unknown>}[]>([]);
 const narrow=useNarrow()||phone;
 useEffect(followVisualViewport,[]);
 useEffect(()=>{document.documentElement.dataset.theme=light?'light':'dark'},[light]);
 useEffect(()=>{const q=matchMedia(NARROW_QUERY);const sync=()=>{document.documentElement.toggleAttribute('data-narrow',phone||q.matches);document.documentElement.toggleAttribute('data-lab-phone',phone&&!q.matches)};sync();q.addEventListener('change',sync);return()=>q.removeEventListener('change',sync)},[phone]);
 useEffect(()=>{opened=(id,args)=>setCreated(was=>[...was,{id,args}]);return()=>{opened=()=>{}}},[]);
 useEffect(()=>{const el=document.querySelector('.launch-window');if(!el)return;const size=()=>document.documentElement.style.setProperty('--lab-frame-bottom',Math.max(0,innerHeight-el.getBoundingClientRect().bottom)+'px');const ro=new ResizeObserver(size);ro.observe(el);size();return()=>ro.disconnect()},[phone]);
 const newTab=()=>{setDraft(true);setEpoch(n=>n+1);setActive('new-tab:lab');setList(false)};
 const tabFaces:TabFace[]=[{id:'khor-codex',label:'khor-codex',face:devices[0].face,path:'~/working/khor',session:{kind:'tui',category:'codex',word:'idle'}},...created.map(c=>({id:c.id,label:String(c.args.title||c.args.command||c.args.agent||'Shell').split('\n')[0],face:devices.find(d=>d.name===c.args.machine)?.face||devices[0].face,path:String(c.args.cwd||c.args.dir||''),session:{kind:'shell',category:null,word:'idle'}})),...(draft?[{id:'new-tab:lab',label:'新标签页',face:null,path:'',mark:newTabKind()}]:[])];
 const focus=(id:string)=>{setActive(id);setList(false)};
 const close=(id:string)=>{if(id==='new-tab:lab')setDraft(false);else setCreated(was=>was.filter(c=>c.id!==id));if(active===id)setActive('khor-codex')};
 const current=created.find(c=>c.id===active);
 const page=active==='new-tab:lab'?<NewSessionPage key={entry+epoch} machine={entry==='global'?'':'MacLab'} dir={entry==='global'?'':'/Users/zgh/Desktop/working/khor'} onCancel={()=>{setDraft(false);setActive('khor-codex');setList(entry==='global')}} onOpened={id=>{setDraft(false);setActive(id)}}/>:<div className="launch-preview"><div className="command-preview">{String(current?.args.machine||'MacLab')} {String(current?.args.cwd||current?.args.dir||'~/working/khor')} % {String(current?.args.command||current?.args.agent||'')}</div><span className="preview-notice">仅预览，未执行命令</span><Button onClick={newTab}>新建会话</Button></div>;
 return <main className="launch-lab"><header className="lab-tools"><span className="lab-label">选择交互 <span>Lab · 正式组件</span></span><div className="lab-scenarios"><Button onClick={()=>setDialog('workspace')}>添加工作区</Button><Button onClick={()=>setDialog('import')}>导入会话</Button><span className="text-muted-foreground">入口</span><PopupButton label="预览入口" value={entry} choices={[{value:'global',label:'全局 ＋'},{value:'workspace',label:'工作区 · khor'},{value:'terminal',label:'当前终端 · khor'}]} onPick={value=>{setEntry(value);newTab()}}/><Button variant="ghost" onClick={()=>setPhone(!phone)}>{phone?'桌面预览':'窄屏预览'}</Button><Button variant="ghost" onClick={()=>setLight(!light)}>{light?'深色':'浅色'}</Button></div></header>
 <section className="launch-window" data-phone={phone||undefined} aria-label="新建会话预览">{narrow?list?<div className="launch-session-list"><Button onClick={newTab}>新建会话</Button><TabSheetRows faces={tabFaces.filter(f=>f.id!=='new-tab:lab')} active={active} onFocus={focus} onClose={close}/></div>:<SessionScreen faces={tabFaces} active={active} onFocus={focus} onClose={close} onBack={()=>setList(true)} onNew={newTab} newLabel="新会话" folded={false}>{page}</SessionScreen>:<><header className="launch-titlebar"><TabStrip faces={tabFaces} active={active} onFocus={focus} onClose={close} onNew={newTab}/></header>{page}</>}</section><footer className="lab-footer">{pickedDirectory || '正式组件，API 为假数据；不会执行任何命令'}</footer><DirPicker open={dialog==='workspace'} onOpenChange={open=>{if(!open)setDialog(null)}} onPick={pick=>{if(pick.path.includes('missing'))throw new Error('目录不存在，请修改路径后重试');setPickedDirectory(pick.machine+':'+pick.path)}}/><ImportDialog open={dialog==='import'} onOpenChange={open=>{if(!open)setDialog(null)}} onImported={ids=>setPickedDirectory('已导入：'+ids.join(', '))}/></main>;
}
createRoot(document.getElementById('root')!).render(<SettingsProvider value={settings}><TooltipProvider><Lab/></TooltipProvider></SettingsProvider>);
