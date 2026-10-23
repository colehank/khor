/** Shared session controls. The placement experiment remains on the Lab branch. */
import {useEffect,useLayoutEffect,useRef,useState,isValidElement,type ReactNode,type ComponentProps} from 'react';
import {createPortal} from 'react-dom';
import {BarSeat,useInBar} from '@/components/SessionBar';
import {Popover,PopoverTrigger,PopoverContent} from '@/components/ui/popover';
import {IconChevronDown} from '@/components/icons';
import {Button} from '@/components/ui/button';
import {Dialog,DialogContent,DialogTitle,DialogBody,DialogFooter} from '@/components/ui/dialog';
import {Switch} from '@/components/settings/Switch';
import {AddSeat,PlusSeat,SendSeat,StopSeat,ModeSeat,type AddOption,type Who} from '@/views/Composer';
import {piJob} from '@/lib/pi-accounts';
import type {AgentProtocol,SessionConfigOption} from '@/lib/acp-session';
import {khorAgent,type KhorAgentView} from '@/api';
import type {CommandRecord} from '@/gen/bindings/CommandRecord';
import '@/styles/session-preferences.css';

export const labUnified=new URLSearchParams(location.search).has('labSession')&&new URLSearchParams(location.search).get('composer')!=='classic';
const shortModel=(model:string|null)=>{
  if(!model)return '模型设置';
  return model.replace(/^claude-sonnet-(\d+)-(\d+).*$/,'Sonnet $1.$2').replace(/^claude-opus-(\d+)-(\d+).*$/,'Opus $1.$2').replace(/^grok-(\d+)\.(\d+)$/,'Grok $1.$2').replace(/^gpt-/,'GPT-');
};
function HeaderSeat({children}:{children:ReactNode}){
  const inBar=useInBar(),[wide,setWide]=useState<Element|null>(null);
  useLayoutEffect(()=>{
    if(inBar)return;
    const find=()=>setWide(document.querySelector('[data-lab-session-settings-slot]'));
    find();const observer=new MutationObserver(find);observer.observe(document.body,{childList:true,subtree:true});
    return()=>observer.disconnect();
  },[inBar]);
  return inBar?<BarSeat at="trail">{children}</BarSeat>:wide?createPortal(children,wide):null;
}
function Preferences({model,agent,mode,children,open,onOpen}:{model:string|null;agent:string;mode?:string;children:ReactNode;open:boolean;onOpen:(open:boolean)=>void}){
  const panel=useRef<HTMLDivElement>(null);
  return <HeaderSeat><Popover open={open} onOpenChange={onOpen}>
    <PopoverTrigger asChild><button type="button" data-lab-session-settings aria-label={`会话设置：${shortModel(model)}`} className="glass composer-lab-trigger">
      <span className="composer-lab-trigger-words"><span>{shortModel(model)}</span>{mode&&<small data-lab-execution-badge>{mode}</small>}</span><IconChevronDown/>
    </button></PopoverTrigger>
    <PopoverContent ref={panel} tabIndex={-1} onOpenAutoFocus={e=>{e.preventDefault();panel.current?.focus();}} side="bottom" align="end" sideOffset={8} data-lab-preferences className="composer-lab-preferences">
      <div className="composer-lab-preferences-heading"><strong>会话设置</strong><span>{agent}</span></div>
      {children}
      <details className="composer-lab-details"><summary>模型详情</summary><code data-lab-model-id>{model??'宿主未报告模型'}</code></details>
    </PopoverContent>
  </Popover></HeaderSeat>;
}
function Option({option}:{option:AddOption}){
  const label=option.name;
  return <label className="composer-lab-setting" data-switch={option.kind==='switch'||undefined}><span>{label}</span>{option.kind==='switch'
    ?<Switch on={option.on} label={label} onToggle={option.onToggle}/>
    :<select aria-label={label} data-lab-option={option.id} value={option.current} onChange={e=>option.onPick(e.target.value)}>
      {option.values.map(v=><option key={v.value} value={v.value}>{v.heading ? v.heading + " · " : ""}{v.name}</option>)}
    </select>}{option.note&&<small>{option.note}</small>}</label>;
}
export function AcpHeader({who,mode,options,protocol,onControl,disabled=false}:{who:Who;mode:ReactNode;options:AddOption[];protocol?:AgentProtocol|null;onControl?:(operation:string,payload?:Record<string,unknown>)=>unknown;disabled?:boolean}){
  const [open,setOpen]=useState(false),[deleting,setDeleting]=useState(false);
  const m=isValidElement(mode)?mode.props as ComponentProps<typeof ModeSeat>:null;
  const automatic=m&&['workspace','auto','acceptEdits','bypassPermissions','full-access'].includes(m.current);
  return <><Preferences model={who.model} agent={who.name??'智能体'} mode={automatic?m.word:undefined} open={open} onOpen={setOpen}>
    <fieldset disabled={disabled} className="contents">{options.map(option=><Option key={option.id} option={option}/>)}</fieldset>
    {disabled&&options.length>0&&<p className="composer-lab-description">本轮结束后可修改模型设置</p>}
    {m&&<label className="composer-lab-setting"><span>执行权限</span>{m.choices&&m.onPick
      ?<select aria-label="执行权限" data-lab-permission value={m.current} onChange={e=>m.onPick?.(e.target.value)}>{m.choices.map(v=><option key={v.id} disabled={v.fixed} value={v.id}>{v.name}</option>)}</select>
      :<span>{m.word}</span>}
      <small>{m.choices?.find(v=>v.id===m.current)?.note}</small></label>}
    {who.usage&&<div className="composer-lab-fact"><span>上下文</span><span>{Math.round(who.usage.used/who.usage.size*100)}%</span></div>}
    {protocol?.authMethods?.filter(method=>!method.type||method.type==='agent').map(method=><Button key={method.id} disabled={disabled} onClick={()=>onControl?.('authenticate',{methodId:method.id})}>{method.name}</Button>)}
    {protocol?.capabilities?.auth?.logout!=null&&<Button disabled={disabled} onClick={()=>onControl?.('logout')}>退出登录</Button>}
    {protocol?.capabilities?.sessionCapabilities?.close!=null&&<Button disabled={disabled} onClick={()=>onControl?.('close')}>结束会话</Button>}
    {protocol?.capabilities?.sessionCapabilities?.delete!=null&&<Button disabled={disabled} onClick={()=>{setOpen(false);setDeleting(true);}}>删除会话记录…</Button>}
    {!options.length&&!m&&<p className="composer-lab-description">此会话由宿主管理模型与执行权限</p>}
  </Preferences><Dialog open={deleting} onOpenChange={setDeleting}><DialogContent purpose="confirmation"><DialogTitle>删除会话记录？</DialogTitle><DialogBody>由智能体删除这段会话的记录。删除后无法从这里恢复。</DialogBody><DialogFooter><Button onClick={()=>setDeleting(false)}>取消</Button><Button variant="destructive" onClick={()=>{setDeleting(false);onControl?.('delete');}}>删除记录</Button></DialogFooter></DialogContent></Dialog></>;
}
export function KhorHeader({view,onChanged,disabled,updates=[]}:{view:KhorAgentView|null;onChanged:(view:KhorAgentView)=>void;disabled:boolean;updates?:readonly string[]}){
  const [open,setOpen]=useState(false),[saving,setSaving]=useState(false),[error,setError]=useState(''),[options,setOptions]=useState<SessionConfigOption[]>([]);
  const request=useRef<AbortController|null>(null);
  const account=view?.accounts.find(a=>a.id===view.config?.account),model=view?.config?.model??'';
  useEffect(()=>{if(!open||!account)return;const abort=new AbortController();request.current=abort;setError('');
    void piJob(account.home,{operation:'session_settings',account:account.id},abort.signal).then(result=>{if(!abort.signal.aborted)setOptions(result.result?.configOptions??[]);},e=>{if(!abort.signal.aborted)setError(String(e));});
    return()=>abort.abort();
  },[open,account?.id,account?.home,model]);
  useEffect(()=>()=>request.current?.abort(),[]);
  const pick=async(configId:string,value:string|boolean)=>{
    if(!account||saving)return;request.current?.abort();const abort=new AbortController();request.current=abort;setSaving(true);setError('');
    try{const result=await piJob(account.home,{operation:'session_set_option',account:account.id,configId,value},abort.signal);
      if(!abort.signal.aborted){setOptions(result.result?.configOptions??[]);onChanged(await khorAgent('read',account.home));}
    }catch(e){if(!abort.signal.aborted)setError(String(e));}finally{if(!abort.signal.aborted)setSaving(false);}
  };
  const ui=options.flatMap((o):AddOption[]=>o.type==='boolean'?[{kind:'switch',id:o.id,name:o.name,note:o.description??null,on:o.currentValue===true,onToggle:()=>void pick(o.id,o.currentValue!==true)}]:o.type==='select'?[{kind:'values',id:o.id,name:o.name,note:o.description??null,current:String(o.currentValue??''),values:(o.options??[]).flatMap(v=>'group' in v?v.options.map(x=>({...x,heading:v.name,note:x.description??null})):[{...v,heading:"",note:v.description??null}]),onPick:v=>void pick(o.id,v)}]:[]);
  const usage=updates.reduce<{used:number;size:number}|null>((last,encoded)=>{try{const u=JSON.parse(encoded);if(u.sessionUpdate==='compaction_update')return null;return u.sessionUpdate==='usage_update'&&Number.isFinite(u.used)&&u.size>0?u:last;}catch{return last;}},null);
  return <Preferences model={model} agent="khor" open={open} onOpen={value=>{setOpen(value);setSaving(false);}}>
    <fieldset disabled={saving} className="contents">{ui.map(option=><Option key={option.id} option={option}/>)}</fieldset>
    {!options.length&&account&&!error&&<p className="composer-lab-description" role="status">读取会话设置…</p>}
    {usage&&<div className="composer-lab-fact"><span>上下文</span><span>{Math.round(usage.used/usage.size*100)}% · {usage.used.toLocaleString()} / {usage.size.toLocaleString()}</span></div>}
    <div className="composer-lab-fact"><span>账户</span><span>{account?.name??'未配置'}</span></div>
    <div className="composer-lab-fact"><span>运行于</span><span>{account?.home_name??'—'}</span></div>
    {disabled&&<p className="composer-lab-description">设置从下一轮生效</p>}
    {error&&<p role="alert" className="composer-lab-description">{error}</p>}
  </Preferences>;
}
export function LabAdd({destination,...props}:ComponentProps<typeof AddSeat>&{destination?:ReactNode}){
  const [open,setOpen]=useState(false);
  if(!labUnified)return <AddSeat {...props}/>;
  if(!props.onAttach&&!props.onAttachMachine)return null;
  return <Popover open={open} onOpenChange={setOpen}>
    <PopoverTrigger asChild><PlusSeat data-lab-add aria-label="添加附件"/></PopoverTrigger>
    <PopoverContent side="top" align="start" sideOffset={12} data-lab-attachments className="composer-lab-attachments">
      <strong>添加附件</strong>
      {props.onAttach&&<button type="button" data-lab-attach-device onClick={()=>{setOpen(false);props.onAttach?.();}}>从这台设备添加…</button>}
      {props.onAttachMachine&&<button type="button" onClick={()=>{setOpen(false);props.onAttachMachine?.pick();}}>{props.onAttachMachine.label}</button>}
      {destination&&<div className="composer-lab-upload-setting">{destination}</div>}
    </PopoverContent>
  </Popover>;
}
export function KhorSend({record,onStopped,ready,sending,onSend}:{record:CommandRecord|null;onStopped:()=>void;ready:boolean;sending:boolean;onSend:()=>void}){
  if(record)return <StopSeat tip={false} onStop={()=>{void khorAgent('stop',record.owner,{run:record.id}).then(onStopped);}}/>;
  return ready?<SendSeat onSend={onSend} disabled={sending} keepFocus/>:null;
}
