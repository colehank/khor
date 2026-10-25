import {useState} from 'react';
import {Input} from '@/components/ui/input';
import {Button} from '@/components/ui/button';
import {openLink} from '@/api';

type Property={type?:string;title?:string;description?:string;default?:unknown;enum?:string[];oneOf?:{const:string;title?:string}[];minimum?:number;maximum?:number;minLength?:number;maxLength?:number;pattern?:string;format?:string;items?:{enum?:string[];anyOf?:{const:string;title?:string}[]};minItems?:number;maxItems?:number};
export type InteractionRequest={mode?:string;message?:string;sessionId?:string;url?:string;requestedSchema?:{properties?:Record<string,Property>;required?:string[]}};
/** Ephemeral interaction: answers never enter the transcript or browser storage. */
export function AgentInteraction({request,onAnswer}:{request:InteractionRequest;onAnswer:(response:unknown)=>Promise<unknown>}){
  const [values,setValues]=useState<Record<string,unknown>>(()=>Object.fromEntries(Object.entries(request.requestedSchema?.properties??{}).filter(([,p])=>p.default!==undefined||p.type==='boolean'||p.type==='array').map(([k,p])=>[k,p.default??(p.type==='boolean'?false:[])]))),[busy,setBusy]=useState(false),[error,setError]=useState(''),[visited,setVisited]=useState(false);
  const props=request.requestedSchema?.properties??{},required=request.requestedSchema?.required??[];
  const supported=request.mode==='form'&&Object.values(props).every(p=>['string','number','integer','boolean','array'].includes(p.type??''))||request.mode==='url';
  const answer=async(action:string)=>{if(busy)return;setBusy(true);setError('');try{await onAnswer({action,...(action==='accept'&&request.mode==='form'?{content:values}:{})});}catch(e){setError(String(e));}finally{setBusy(false);}};
  let target:URL|null=null;try{const parsed=new URL(request.url??'');if(['https:','http:'].includes(parsed.protocol)&&!parsed.username&&!parsed.password)target=parsed;}catch{/* unknown link stays unavailable */}
  return <form data-agent-interaction className="flex flex-col gap-row rounded-md bg-fill p-row text-callout" onSubmit={e=>{e.preventDefault();void answer('accept');}}>
    <strong>{request.message??'智能体需要补充信息'}</strong>
    {request.mode==='form'&&Object.entries(props).map(([key,p])=>{
      const choices=p.oneOf?.map(v=>({value:v.const,label:v.title??v.const}))??p.enum?.map(value=>({value,label:value}));
      const multi=p.items?.anyOf?.map(v=>({value:v.const,label:v.title??v.const}))??p.items?.enum?.map(value=>({value,label:value}));
      const set=(value:unknown)=>setValues(now=>({...now,[key]:value}));
      return <label key={key} className="flex flex-col gap-in"><span>{p.title??key}{required.includes(key)?' *':''}</span>
        {p.type==='boolean'?<input type="checkbox" checked={values[key]===true} onChange={e=>set(e.target.checked)}/>:choices?<select className="min-h-touch rounded-sm bg-fill px-in" required={required.includes(key)} value={String(values[key]??'')} onChange={e=>set(e.target.value)}><option value="" disabled>请选择</option>{choices.map(v=><option key={v.value} value={v.value}>{v.label}</option>)}</select>:p.type==='array'?<select multiple aria-label={p.title??key} className="min-h-touch rounded-sm bg-fill px-in" value={Array.isArray(values[key])?values[key] as string[]:[]} onChange={e=>set([...e.target.selectedOptions].map(o=>o.value))}>{multi?.map(v=><option key={v.value} value={v.value}>{v.label}</option>)}</select>:<Input aria-label={p.title??key} required={required.includes(key)} type={p.type==='integer'||p.type==='number'?'number':p.format==='email'?'email':p.format==='uri'?'url':p.format==='date'?'date':'text'} min={p.minimum} max={p.maximum} step={p.type==='integer'?1:'any'} minLength={p.minLength} maxLength={p.maxLength} pattern={p.pattern} value={String(values[key]??'')} onChange={e=>{if(e.target.value==='')setValues(now=>{const next={...now};delete next[key];return next;});else set(p.type==='integer'||p.type==='number'?Number(e.target.value):e.target.value);}}/>}
        {p.description&&<small className="text-caption text-muted-foreground">{p.description}</small>}
      </label>;
    })}
    {request.mode==='url'&&<div className="flex flex-col gap-in"><span className="break-all">{target?.host??'无法打开此地址'}</span>{target&&<Button type="button" disabled={busy} onClick={()=>{void openLink(target!.href).then(()=>setVisited(true),e=>setError(String(e)));}}>打开 {target.host}</Button>}</div>}
    {!supported&&<p>此补问格式暂不支持，请取消后让智能体换一种方式。</p>}
    {error&&<p role="alert" className="text-destructive">{error}</p>}
    <div className="flex flex-wrap gap-in"><Button type="submit" disabled={busy||!supported||(request.mode==='url'&&!visited)}>确认</Button><Button type="button" disabled={busy} onClick={()=>void answer('decline')}>拒绝</Button><Button type="button" disabled={busy} onClick={()=>void answer('cancel')}>取消</Button></div>
  </form>;
}
