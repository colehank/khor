/** Shared session controls. The placement experiment remains on the Lab branch. */
import {useEffect,useLayoutEffect,useRef,useState,isValidElement,cloneElement,Fragment,type ReactNode,type ReactElement,type ComponentProps} from 'react';
import {createPortal} from 'react-dom';
import {BarSeat,useInBar} from '@/components/SessionBar';
import {DropdownMenu,DropdownMenuTrigger,DropdownMenuContent,DropdownMenuItem,DropdownMenuLabel,DropdownMenuSeparator,DropdownMenuCheckboxItem,DropdownMenuRadioGroup,DropdownMenuRadioItem} from '@/components/ui/dropdown-menu';
import {IconBack,IconChevronDown,IconChevronRight,IconFiles,IconUpload,IconMore,IconFilter} from '@/components/icons';
import {Button} from '@/components/ui/button';
import {Dialog,DialogContent,DialogTitle,DialogBody,DialogFooter} from '@/components/ui/dialog';
import {SendInto,type SendIntoControl} from '@/views/chat-files';
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
/** Session actions share App's 更多 menu. Choices replace its rows, so
 * phone users never have to chase a submenu beside the screen. */
function Preferences({model,agent,mode,options,disabled=false,children,danger,open,onOpen}:{model:string|null;agent:string;mode?:string;options:AddOption[];disabled?:boolean;children:ReactNode;danger?:ReactNode;open:boolean;onOpen:(open:boolean)=>void}){
  const inBar=useInBar(),docked=inBar&&labUnified&&new URLSearchParams(location.search).get('deck')==='combined';
  const light=inBar&&labUnified&&new URLSearchParams(location.search).get('deck')==='light';
  const modelBar=inBar&&labUnified&&new URLSearchParams(location.search).get('deck')==='compose';
  const thought=options.find(o=>o.kind==='values'&&o.category==='thought_level');
  const levels:Record<string,string>={off:'关闭',minimal:'最低',low:'低',medium:'中',high:'高',xhigh:'极高',standard:'标准',deep:'深入',default:'默认',auto:'自动'};
  const effort=thought?.kind==='values'?(levels[thought.current]??thought.values.find(v=>v.value===thought.current)?.name??thought.current):null;
  const [page,setPage]=useState<string|null>(null),panel=useRef<HTMLDivElement>(null);
  const choice=page?.startsWith('option:')?options.find(o=>o.id===page.slice(7)):undefined,values=choice?.kind==='values'?choice:null;
  const back=()=>{setPage(null);requestAnimationFrame(()=>panel.current?.querySelector<HTMLElement>(page?.startsWith('option:')?`[data-lab-option="${CSS.escape(page.slice(7))}"]`:'[data-lab-option]')?.focus());};
  useLayoutEffect(()=>{
    if(!open||!page)return;
    const frame=requestAnimationFrame(()=>(panel.current?.querySelector<HTMLElement>('[role="menuitemradio"][data-state="checked"]')??panel.current?.querySelector<HTMLElement>('[data-session-menu-back]'))?.focus());
    return()=>cancelAnimationFrame(frame);
  },[page,open]);
  return <HeaderSeat><DropdownMenu open={open} onOpenChange={value=>{if(value)setPage(null);onOpen(value);}}>
    <DropdownMenuTrigger asChild><button type="button" data-lab-session-settings data-lab-dock-settings={docked||undefined} data-lab-config-trigger={light||undefined} data-lab-model-trigger={modelBar||undefined} title={light?'会话配置':undefined} aria-label={`会话配置：${shortModel(model)}${modelBar&&effort?' · '+effort:''}`} className={`${light||modelBar?'':'glass '}composer-lab-trigger data-[state=open]:glass-sel`}>
      {modelBar?<><span className="lab-model-summary"><span>{shortModel(model)}</span>{effort&&<><span aria-hidden>·</span><span data-lab-effort>{effort}</span></>}</span><IconChevronDown/></>:light?<IconFilter/>:docked?<IconMore/>:<><span className="composer-lab-trigger-words"><span>{shortModel(model)}</span>{mode&&<small data-lab-execution-badge>{mode}</small>}</span><IconChevronDown/></>}
    </button></DropdownMenuTrigger>
    <DropdownMenuContent ref={panel} side={docked?'top':'bottom'} align="end" sideOffset={8} data-lab-preferences className="min-w-nav-menu composer-lab-menu" onKeyDown={e=>{if(e.key==='ArrowLeft'&&page){e.preventDefault();back();}}}>
      {page?<>
        <DropdownMenuItem data-session-menu-back onSelect={e=>{e.preventDefault();back();}}><IconBack className="size-affordance"/>返回</DropdownMenuItem>
        <DropdownMenuSeparator/>
        {values?<>
          <DropdownMenuLabel>{values.name}</DropdownMenuLabel>
          {values.note&&<DropdownMenuLabel className="composer-lab-menu-note">{values.note}</DropdownMenuLabel>}
          <DropdownMenuRadioGroup aria-label={values.name} value={values.current} onValueChange={value=>{if(!disabled)values.onPick(value);}}>
            {values.values.map((v,i)=><Fragment key={v.value}>
              {v.heading&&v.heading!==values.values[i-1]?.heading&&<DropdownMenuLabel>{v.heading}</DropdownMenuLabel>}
              <DropdownMenuRadioItem value={v.value} disabled={disabled} data-option-value={v.value} className="composer-lab-choice"><span>{v.name}</span>{v.note&&<small>{v.note}</small>}</DropdownMenuRadioItem>
            </Fragment>)}
          </DropdownMenuRadioGroup>
        </>:page==='model-details'?<><DropdownMenuLabel>模型详情 · {agent}</DropdownMenuLabel><DropdownMenuLabel><code data-lab-model-id className="composer-lab-model-id">{model??'宿主未报告模型'}</code></DropdownMenuLabel></>:<DropdownMenuLabel>此设置已由智能体移除</DropdownMenuLabel>}
      </>:<>
        <DropdownMenuLabel>{agent} · 会话设置</DropdownMenuLabel>
        {options.map(option=>option.kind==='switch'
          ?<DropdownMenuCheckboxItem key={option.id} disabled={disabled} checked={option.on} data-lab-option={option.id} onCheckedChange={()=>option.onToggle()} onSelect={e=>e.preventDefault()}>{option.name}</DropdownMenuCheckboxItem>
          :<DropdownMenuItem key={option.id} disabled={disabled} data-lab-option={option.id} onSelect={e=>{e.preventDefault();setPage(`option:${option.id}`);}}><span className="min-w-0 flex-1">{option.name}</span><span className="composer-lab-menu-value">{option.values.find(v=>v.value===option.current)?.name??option.current}</span><IconChevronRight className="size-affordance text-muted-foreground"/></DropdownMenuItem>)}
        {children}
        <DropdownMenuSeparator/>
        <DropdownMenuItem onSelect={e=>{e.preventDefault();setPage('model-details');}}>模型详情<IconChevronRight className="ml-auto size-affordance text-muted-foreground"/></DropdownMenuItem>
        {danger&&<><DropdownMenuSeparator/>{danger}</>}
      </>}
    </DropdownMenuContent>
  </DropdownMenu></HeaderSeat>;
}
function Fact({name,children}:{name:string;children:ReactNode}){
  return <DropdownMenuLabel className="composer-lab-fact"><span>{name}</span><span>{children}</span></DropdownMenuLabel>;
}
export function AcpHeader({who,mode,options,protocol,onControl,disabled=false}:{who:Who;mode:ReactNode;options:AddOption[];protocol?:AgentProtocol|null;onControl?:(operation:string,payload?:Record<string,unknown>)=>unknown;disabled?:boolean}){
  const [open,setOpen]=useState(false),[deleting,setDeleting]=useState(false);
  const m=isValidElement(mode)?mode.props as ComponentProps<typeof ModeSeat>:null;
  const automatic=m&&['workspace','auto','acceptEdits','bypassPermissions','full-access'].includes(m.current);
  let permissionId='execution-permission';while(options.some(o=>o.id===permissionId))permissionId+='-';
  const permissions:AddOption[]=m?.choices?.some(v=>!v.fixed)&&m.onPick?[{kind:'values',id:permissionId,name:'执行权限',note:null,current:m.current,values:m.choices.filter(v=>!v.fixed).map(v=>({value:v.id,name:v.name,note:v.note??null,heading:null})),onPick:value=>m.onPick?.(value)}]:[];
  return <><Preferences model={who.model} agent={who.name??'智能体'} mode={automatic?m.word:undefined} options={[...options,...permissions]} disabled={disabled} open={open} onOpen={setOpen}
    danger={protocol?.capabilities?.sessionCapabilities?.delete!=null&&<DropdownMenuItem className="text-destructive" disabled={disabled} onSelect={()=>{setOpen(false);setDeleting(true);}}>删除会话记录</DropdownMenuItem>}>
    {disabled&&options.length>0&&<DropdownMenuLabel className="composer-lab-menu-note">本轮结束后可修改模型设置</DropdownMenuLabel>}
    {m&&!permissions.length&&<Fact name="执行权限">{m.word}</Fact>}
    {who.usage&&<Fact name="上下文">{Math.round(who.usage.used/who.usage.size*100)}%</Fact>}
    {protocol?.authMethods?.filter(method=>!method.type||method.type==='agent').map(method=><DropdownMenuItem key={method.id} disabled={disabled} onSelect={()=>onControl?.('authenticate',{methodId:method.id})}>{method.name}</DropdownMenuItem>)}
    {protocol?.capabilities?.auth?.logout!=null&&<DropdownMenuItem disabled={disabled} onSelect={()=>onControl?.('logout')}>退出登录</DropdownMenuItem>}
    {protocol?.capabilities?.sessionCapabilities?.close!=null&&<DropdownMenuItem disabled={disabled} onSelect={()=>onControl?.('close')}>结束会话</DropdownMenuItem>}
    {!options.length&&!m&&<DropdownMenuLabel className="composer-lab-menu-note">此会话由宿主管理模型与执行权限</DropdownMenuLabel>}
  </Preferences><Dialog open={deleting} onOpenChange={setDeleting}><DialogContent purpose="confirmation"><DialogTitle>删除会话记录？</DialogTitle><DialogBody>由智能体删除这段会话的记录。删除后无法从这里恢复。</DialogBody><DialogFooter><Button onClick={()=>setDeleting(false)}>取消</Button><Button variant="destructive" onClick={()=>{setDeleting(false);onControl?.('delete');}}>删除记录</Button></DialogFooter></DialogContent></Dialog></>;
}
export function KhorHeader({view,onChanged,disabled,updates=[]}:{view:KhorAgentView|null;onChanged:(view:KhorAgentView)=>void;disabled:boolean;updates?:readonly string[]}){
  const [open,setOpen]=useState(false),[saving,setSaving]=useState(false),[error,setError]=useState(''),[options,setOptions]=useState<SessionConfigOption[]>([]);
  const request=useRef<AbortController|null>(null);
  const account=view?.accounts.find(a=>a.id===view.config?.account),model=view?.config?.model??'';
  useEffect(()=>{if(!account||saving)return;const abort=new AbortController();setError('');
    void piJob(account.home,{operation:'session_settings',account:account.id},abort.signal).then(result=>{if(!abort.signal.aborted)setOptions(result.result?.configOptions??[]);},e=>{if(!abort.signal.aborted)setError(String(e));});
    return()=>abort.abort();
  },[open,account?.id,account?.home,model,saving]);
  useEffect(()=>()=>request.current?.abort(),[]);
  const pick=async(configId:string,value:string|boolean)=>{
    if(!account||saving)return;request.current?.abort();const abort=new AbortController();request.current=abort;setSaving(true);setError('');
    try{const result=await piJob(account.home,{operation:'session_set_option',account:account.id,configId,value},abort.signal);
      if(!abort.signal.aborted){setOptions(result.result?.configOptions??[]);onChanged(await khorAgent('read',account.home));}
    }catch(e){if(!abort.signal.aborted)setError(String(e));}finally{if(!abort.signal.aborted)setSaving(false);}
  };
  const ui=options.flatMap((o):AddOption[]=>o.type==='boolean'?[{kind:'switch',id:o.id,name:o.name,note:o.description??null,on:o.currentValue===true,onToggle:()=>void pick(o.id,o.currentValue!==true)}]:o.type==='select'?[{kind:'values',id:o.id,name:o.name,note:o.description??null,current:String(o.currentValue??''),values:(o.options??[]).flatMap(v=>'group' in v?v.options.map(x=>({...x,heading:v.name,note:x.description??null})):[{...v,heading:"",note:v.description??null}]),onPick:v=>void pick(o.id,v)}]:[]).map(o=>({...o,category:options.find(source=>source.id===o.id)?.category}));
  const usage=updates.reduce<{used:number;size:number}|null>((last,encoded)=>{try{const u=JSON.parse(encoded);if(u.sessionUpdate==='compaction_update')return null;return u.sessionUpdate==='usage_update'&&Number.isFinite(u.used)&&u.size>0?u:last;}catch{return last;}},null);
  return <Preferences model={model} agent="khor" options={ui} disabled={saving} open={open} onOpen={setOpen}>
    {!options.length&&account&&!error&&<DropdownMenuLabel role="status">读取会话设置…</DropdownMenuLabel>}
    <DropdownMenuSeparator/>
    <Fact name="执行权限"><span data-khor-permission>逐项确认</span></Fact>
    <DropdownMenuLabel className="composer-lab-menu-note">读取直接执行，修改前逐项确认</DropdownMenuLabel>
    {usage&&<Fact name="上下文">{Math.round(usage.used/usage.size*100)}% · {usage.used.toLocaleString()} / {usage.size.toLocaleString()}</Fact>}
    <Fact name="账户">{account?.name??'未配置'}</Fact>
    <Fact name="运行于">{account?.home_name??'—'}</Fact>
    {disabled&&<DropdownMenuLabel className="composer-lab-menu-note">设置从下一轮生效</DropdownMenuLabel>}
    {error&&<DropdownMenuLabel role="alert" className="composer-lab-menu-note text-state-failed">{error}</DropdownMenuLabel>}
  </Preferences>;
}
export function LabAdd({destination,...props}:ComponentProps<typeof AddSeat>&{destination?:ReactElement<ComponentProps<typeof SendInto>>|null}){
  if(!labUnified)return <AddSeat {...props}/>;
  if(!props.onAttach&&!props.onAttachMachine)return null;
  const menu=(target?:SendIntoControl)=><DropdownMenu>
    <DropdownMenuTrigger asChild><PlusSeat data-lab-add aria-label="添加附件"/></DropdownMenuTrigger>
    <DropdownMenuContent side="top" align="start" sideOffset={16} alignOffset={-6} data-lab-attachments className="min-w-nav-menu composer-lab-menu" onCloseAutoFocus={e=>e.preventDefault()}>
      {props.onAttach&&<DropdownMenuItem data-lab-attach-device onSelect={props.onAttach}><IconUpload className="size-nav-glyph"/>从这台设备添加</DropdownMenuItem>}
      {props.onAttachMachine&&<DropdownMenuItem onSelect={props.onAttachMachine.pick}><IconFiles className="size-nav-glyph"/>{props.onAttachMachine.label}</DropdownMenuItem>}
      {target&&<><DropdownMenuSeparator/><DropdownMenuItem data-send-into-change onSelect={target.pick}><span>上传位置</span><span data-send-into-path className="composer-lab-menu-value ml-auto">{target.path}</span><IconChevronRight className="size-affordance text-muted-foreground"/></DropdownMenuItem>
        {target.reset&&<DropdownMenuItem data-send-into-reset onSelect={target.reset}>恢复默认位置</DropdownMenuItem>}
      </>}
    </DropdownMenuContent>
  </DropdownMenu>;
  // DirPicker must survive closing the menu that opened it.
  return destination?cloneElement(destination,{render:menu}):menu();
}
export function KhorSend({record,onStopped,ready,sending,onSend}:{record:CommandRecord|null;onStopped:()=>void;ready:boolean;sending:boolean;onSend:()=>void}){
  if(record)return <StopSeat tip={false} onStop={()=>{void khorAgent('stop',record.owner,{run:record.id}).then(onStopped);}}/>;
  return ready?<SendSeat onSend={onSend} disabled={sending} keepFocus/>:null;
}
