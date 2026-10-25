import {useEffect,useRef,useState} from 'react';
import {managePi} from '@/lib/pi-accounts';
import type {AgentProtocol} from '@/lib/acp-session';
import {AgentInteraction,type InteractionRequest} from '@/components/AgentInteraction';
import {Button} from '@/components/ui/button';
import {Textarea} from '@/components/ui/textarea';

export type SessionEnvironment={mcpServers:unknown[];additionalDirectories:string[]};
export const EMPTY_ENVIRONMENT:SessionEnvironment={mcpServers:[],additionalDirectories:[]};
/** ACP login and session context are executed on the selected computer. */
export function AcpAgentSetup({agent,machine,environment,onEnvironment,onValid}:{agent:string;machine:string;environment:SessionEnvironment;onEnvironment:(value:SessionEnvironment)=>void;onValid:(valid:boolean)=>void}){
  const [facts,setFacts]=useState<AgentProtocol|null>(null),[busy,setBusy]=useState(false),[error,setError]=useState(''),[job,setJob]=useState<{id:string;ask:number;request:InteractionRequest}|null>(null),[mcp,setMcp]=useState('[]');
  const active=useRef<AbortController|null>(null);
  useEffect(()=>{setFacts(null);setError('');setJob(null);setBusy(false);return()=>active.current?.abort();},[agent,machine]);
  const run=async(methodId?:string)=>{
    active.current?.abort();const abort=new AbortController();active.current=abort;setBusy(true);setError('');let id:string|undefined;let done=false;
    try{
      const start=await managePi(machine,{operation:methodId?'acp_authenticate':'acp_initialize',agent,...(methodId?{methodId}:{})});id=start.job;if(!id)throw new Error(start.error??'无法启动智能体设置');
      const cancel=()=>{void managePi(machine,{operation:'acp_cancel',job:id}).catch(()=>{});};abort.signal.addEventListener('abort',cancel,{once:true});
      if(abort.signal.aborted){cancel();return;}
      try{for(;;){
        abort.signal.throwIfAborted();
        const state=await managePi(machine,{operation:'acp_poll',job:id}) as unknown as {status:string;error?:string;ask?:number;request?:InteractionRequest;result?:AgentProtocol};
        if(abort.signal.aborted)return;
        if(state.status==='done'){done=true;setFacts(state.result??null);setJob(null);break;}
        if(state.status==='failed'||state.status==='cancelled'){done=true;throw new Error(state.error??state.status);}
        setJob(state.status==='waiting'&&state.ask!==undefined&&state.request?{id,ask:state.ask,request:state.request}:null);
        if(state.error)setError(state.error);
        await new Promise<void>(resolve=>setTimeout(resolve,250));
      }}finally{abort.signal.removeEventListener('abort',cancel);if(!done)cancel();}
    }catch(e){if(!abort.signal.aborted)setError(String(e));}finally{if(!abort.signal.aborted)setBusy(false);}
  };
  return <details data-acp-setup className="text-callout"><summary className="min-h-touch py-in">智能体登录与会话工具</summary><div className="flex flex-col gap-row py-row">
    <div className="flex flex-wrap gap-in"><Button disabled={busy} onClick={()=>void run()}>读取智能体能力</Button>{busy&&<Button onClick={()=>{active.current?.abort();setBusy(false);setJob(null);}}>取消</Button>}</div>
    {facts&&<div className="flex flex-col gap-in">{facts.authMethods?.map(method=>(!method.type||method.type==='agent')?<Button key={method.id} disabled={busy} onClick={()=>void run(method.id)}>{method.name}</Button>:<p key={method.id} className="text-muted-foreground">{method.name} · {method.description??'请在执行机器完成此登录方式'}</p>)}{!facts.authMethods?.length&&<p className="text-muted-foreground">智能体未提供登录入口</p>}</div>}
    {job&&<AgentInteraction key={`${job.id}:${job.ask}`} request={job.request} onAnswer={response=>managePi(machine,{operation:'acp_reply',job:job.id,ask:job.ask,response})}/>}
    <label className="flex flex-col gap-in"><span>附加工作目录（每行一个，位于执行机器）</span><Textarea value={environment.additionalDirectories.join('\n')} onChange={e=>onEnvironment({...environment,additionalDirectories:e.target.value.split('\n').filter(Boolean)})}/></label>
    <label className="flex flex-col gap-in"><span>此会话的 MCP servers</span><Textarea aria-label="MCP servers" className="font-mono" value={mcp} onChange={e=>{setMcp(e.target.value);try{const values=JSON.parse(e.target.value);if(!Array.isArray(values))throw new Error('MCP servers 应为数组');onEnvironment({...environment,mcpServers:values});onValid(true);setError('');}catch(e){onValid(false);setError(String(e));}}}/></label>
    {error&&<p role="alert" className="text-destructive">{error}</p>}
  </div></details>;
}
