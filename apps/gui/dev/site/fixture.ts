import {createLabFixture} from '../lab/fixture';
import {scenario} from './scenario';
import {GiB,machines,projects,tasks} from './world';
import {release} from '../../../site/src/releases';
import avatars from './data/avatars.json';
import {siteLoad,siteUsage} from './telemetry';
import type {Avatar} from '../../src/gen/bindings/Avatar';
import type {DeviceRow} from '../../src/gen/bindings/DeviceRow';
import type {SystemKind} from '../../src/gen/bindings/SystemKind';
import type {ModelKind} from '../../src/gen/bindings/ModelKind';
import type {FileText} from '../../src/gen/bindings/FileText';
import type {DirRow} from '../../src/gen/bindings/DirRow';
import type {SessionRow} from '../../src/gen/bindings/SessionRow';
import type {WorkspaceRow} from '../../src/gen/bindings/WorkspaceRow';
import type {AccountRing} from '../../src/gen/bindings/AccountRing';
import type {ChatMsgRow} from '../../src/gen/bindings/ChatMsgRow';
import type {KhorAgentView} from '../../src/gen/bindings/KhorAgentView';

export function createSiteFixture(origin:string) {
  const base=createLabFixture(true,{...scenario,reply:`Let's try Khor, [download it here.](${origin}/download/)`,origin,settings:true,items:3,machines:machines.length});
  const at=Date.now(),version=release.version.replace(/^v/,'');
  const original=base.request('devices') as DeviceRow[];
  const devices:DeviceRow[]=machines.map((profile,index)=>{
    const disk={used:profile.disk[0]*GiB,total:profile.disk[1]*GiB};
    const mount=profile.system==='windows'?'C:/':profile.system==='android'?'/storage/emulated/0':'/';
    const gpuMemory=profile.system==='linux'?{used:1.4*GiB,total:12*GiB}:null;
    return {...original[index],face:avatars[index] as Avatar,name:profile.name,shown:profile.name,paused:false,pinned:index===1,hosts:index<4,
      hop:index===0?null:'direct',last_hop:index===0?null:'direct',rtt_ms:index===0?null:12+index*4,last_rtt_ms:index===0?null:12+index*4,
      down_ms:null,heard_ms:index===0?null:800,since_ms:index===0?null:7200000,relay:null,via:null,sponsor:null,
      roads:[],watch:[],watched:{rows:[],age_ms:0},
      vitals:{age_ms:500,stale:[],mem_strain:null,disk_strain:null,vram_strain:null,volume_strains:[],vitals:{
        system:profile.system as SystemKind,model:{kind:profile.kind as ModelKind,name:profile.model},version,
        cpu_pct:profile.cpu,cores:profile.cores,mem:{used:profile.mem[0]*GiB,total:profile.mem[1]*GiB},disk,
        gpu:profile.gpu===null?null:{util_pct:profile.gpu,cards:1,mem:gpuMemory},
        gpus:profile.gpu===null?[]:[{name:profile.system==='linux'?'NVIDIA RTX 4070 Ti':profile.system==='windows'?'Intel Iris Xe':'Apple GPU',util_pct:profile.gpu,mem:gpuMemory,temp_c:profile.system==='linux'?46:null}],
        volumes:[{mount,fill:disk}],uptime_s:profile.uptime,
      }},
    };
  });
  // The first session is the App's own Khor conversation (MachineChat),
  // backed by durable command receipts, rather than an ACP/Claude chat.
  const model='gpt-5.4';
  const agentView=(data:unknown=null):KhorAgentView=>({
    config:{home:devices[0].id,account:'site-openai',model,options:{thought_level:'medium',auto_compaction:true}},
    accounts:[{id:'site-openai',name:'OpenAI API',home:devices[0].id,home_name:devices[0].name,provider:'openai',default_model:model,configured:true,base_url:null,auth_type:'api_key'}],
    run:null,data,
  });
  const messages:ChatMsgRow[]=[];
  const receipt=(line:string,time:number):ChatMsgRow=>{
    const id='site-khor-'+(messages.length+1);
    return {id,at_ms:time,from:devices[0].name,from_me:true,text:null,files:null,unknown:null,retracted:false,edited:false,
      step:null,moved:0,landed:[],why:null,got:null,of:null,via:null,to:[],note:null,pushed:false,
      command:{id,line,owner:devices[0].id,owner_name:devices[0].name,status:'done',started_at_ms:time,ended_at_ms:time+1200,
        stdout:'',stderr:'',data_json:null,session:null,error:null,agent:{model,approval:null,updated_at_ms:time+1200,updates:[],updates_truncated:false}}};
  };
  for(const frame of scenario.samples[0]){
    const update=(frame as {update?:{update?:{sessionUpdate:string;content?:{text?:string}}}}).update?.update;
    if(!update)continue;
    if(update.sessionUpdate==='user_message_chunk')messages.push(receipt(update.content?.text??'',at-360000+messages.length*180000));
    else messages.at(-1)?.command?.agent?.updates.push(JSON.stringify(update));
  }
  const initialMessages=messages.length;
  const usage=siteUsage(at);
  const owner=(name:unknown)=>devices.find(row=>row.name===name||row.id===name)??devices[0];
  const pathOf=(body:Record<string,any>)=>String(body.path??body.dir??'').replace(/\\/g,'/').replace(/\/$/,'');
  const sourceProject=(body:Record<string,any>)=>{
    const path=pathOf(body);
    return projects.find(project=>path===project.root||path.startsWith(project.root+'/'))??projects.find(project=>devices[project.machine].id===owner(body.machine).id)??projects[0];
  };
  const files=new Map<string,FileText>();
  const originals=new Map<string,string>();
  let revision=1;
  for(const project of projects)for(const file of project.files){
    const path=project.root+'/'+file.path;
    files.set(path,{text:file.text,total:new TextEncoder().encode(file.text).length,digest:'site-1',at_ms:at-(tasks.find(task=>task.root===project.root)?.minutes??3)*60000,cut:false,binary:false});
    originals.set(path,file.before??file.text);
  }
  const fileAt=(body:Record<string,any>)=>{
    const file=files.get(pathOf(body));
    if(!file)throw new Error('此示例中没有这个文件');
    return file;
  };
  const changed=(root:string)=>[...files].filter(([path,file])=>path.startsWith(root+'/')&&file.text!==originals.get(path));
  const commits=(project:typeof projects[number])=>[
    {id:'a'.repeat(39)+'1',subject:project.name==='atlas-web'?'增加订单导出入口':project.name==='atlas-api'?'添加支付回调处理':'增加报销表单基础校验',author:'Lin',when_ms:at-86400000,cut:false},
    {id:'b'.repeat(39)+'2',subject:'补充项目说明',author:'Zhou',when_ms:at-172800000,cut:false},
    {id:'c'.repeat(39)+'3',subject:'初始化项目结构',author:'Lin',when_ms:at-604800000,cut:false},
  ];
  const workspaces=():WorkspaceRow[]=>projects.map(project=>{
    const rows=changed(project.root),head=commits(project)[0];
    return {device:devices[project.machine].id,machine:devices[project.machine].name,path:project.root,name:project.name,
      pinned:project.machine===0,branch:project.branch,changed:rows.length,changed_capped:false,changed_ms:at-180000,
      git:project.branch?{branch:project.branch,head:head.id,subject:head.subject,ahead:0,behind:0,stashes:0,conflicted:0,staged:0,unstaged:rows.length,untracked:0,capped:false,since_ms:at-7200000,operation:null}:null,
      newest:project.branch?null:{path:'远航项目周会.md',at_ms:at-2760000,added:false}};
  });
  const quota=(base.request('quota_board') as AccountRing[]).map((account,index)=>{
    const home=devices[[0,1,0,2,3,0][index]],names=['Claude Max','ChatGPT Pro','DeepSeek','OpenRouter','团队 API','OpenAI API'];
    const short=index===0?36:48,long=index===0?28:54,remaining=[0,0,128.64,56.78,72.5,35.4][index];
    const limit=index===3?200:index===4?300:null,spent=limit===null?null:limit-remaining;
    return {...account,home_name:home.name,mine:home.me,age_ms:0,strain:null,pinned:index===0,budget:limit,
      reading:{...account.reading,name:names[index],home:home.id,fill:index<2?Math.max(short,long)/100:limit===null?null:spent!/limit,
        trouble:null,at_ms:at,remaining:index<2?null:remaining,limit,spent,
        windows:account.reading.windows.map((window,i)=>({...window,used_pct:i===0?short:long,severity:null}))}};
  });
  return {dispose:()=>base.dispose(),request(command:string,body:Record<string,any>={}):unknown {
    switch(command){
      case 'devices':return structuredClone(devices);
      case 'sessions':{
        const rows=base.request(command,body) as SessionRow[];
        return tasks.map((task,index)=>{
          const row=rows.find(row=>row.id===task.id)!,home=devices[task.machine];
          return {...row,title:task.title,category:task.category,pinned:index===0,last:task.id===tasks[0].id&&messages.length>initialMessages?"Let's try Khor, download it here.":task.preview,home:home.name,execution_home:home.id,observation_owner:home.id,face:home.face,place:task.root,
            at_ms:at-task.minutes*60000,unread:0,source:null,stale:false,group:'',can_control:task.category!=='shell'};
        });
      }
      case 'machine_log':return {messages:structuredClone(messages),broken:0};
      case 'khor_agent':{
        if(body.action==='prompt'){
          const message=receipt(String(body.payload?.text??''),Date.now());
          message.command!.agent!.updates.push(JSON.stringify({sessionUpdate:'agent_message_chunk',content:{type:'text',text:`Let's try Khor, [download it here.](${origin}/download/)`}}));
          messages.push(message);
        }
        if(body.action==='manage'){
          if(body.payload?.operation==='session_settings')return agentView({job:'site-settings',status:'running'});
          if(body.payload?.operation==='poll')return agentView({status:'done',result:{model,configOptions:[{id:'model',name:'模型',category:'model',type:'select',currentValue:model,options:[{value:model,name:'GPT-5.4'}]}]}});
        }
        return agentView();
      }
      case 'visit':return null;
      case 'load':return siteLoad(devices.indexOf(owner(body.machine)),at,body.by,body.desc,body.top);
      case 'usage':return structuredClone(usage);
      case 'journal':return [{at:at-7200000,when:new Date(at-7200000).toLocaleTimeString('zh-CN',{hour12:false}),machine:owner(body.machine).name,text:'点对点连接已建立',trouble:false}];
      case 'about':return {version,root:'/Users/lin/.khor',paused:false,store_bytes:18*1024**2,faces:[{key:'gui',name:'桌面',up:true,said:'运行中'},{key:'web',name:'网页',up:true,said:'运行中'}]};
      case 'quota_board':return structuredClone(quota);
      case 'quota':return {quota:{windows:quota[0].reading.windows,as_of:at,plan:'Max 20x',credits:null,fill:.36,source:'claude',limit_reached:false},trouble:null};
      case 'workspaces':return workspaces();
      case 'ls':{
        const path=pathOf(body)||sourceProject(body).root,entries=new Map<string,DirRow>();
        for(const [name,file] of files){
          const project=projects.find(project=>name.startsWith(project.root+'/'))!;
          if(devices[project.machine].id!==owner(body.machine).id)continue;
          if(!name.startsWith(path+'/'))continue;
          const relative=name.slice(path.length+1),part=relative.split('/')[0],dir=relative.includes('/');
          entries.set(part,{name:part,dir,size:dir?0:file.total,at_ms:file.at_ms,from:null,received_ms:null});
        }
        return {path,entries:[...entries.values()].sort((a,b)=>Number(b.dir)-Number(a.dir)||a.name.localeCompare(b.name)),truncated:false};
      }
      case 'read_text':case 'stat_text':return {...fileAt(body)};
      case 'write_text':{
        const file=fileAt(body);
        if(body.was!==file.digest)throw new Error('文件已更新，请重新读取');
        file.text=String(body.text);file.total=new TextEncoder().encode(file.text).length;file.digest='site-'+ ++revision;file.at_ms=Date.now();
        return {digest:file.digest,at_ms:file.at_ms};
      }
      case 'preview':return {kind:'text',total:fileAt(body).total,data:'',media:'',html:'',cut:false,empty:!fileAt(body).text};
      case 'touched':return files.get(pathOf(body))?.at_ms??at;
      case 'checkout':return !!sourceProject(body).branch;
      case 'git_status':{
        const project=sourceProject(body);
        return {branch:project.branch??'',rows:changed(project.root).map(([path])=>({path:path.slice(project.root.length+1),mark:'M'})),more:false,ahead:0,behind:0,fetched_ms:at};
      }
      case 'git_diff':{
        const path=pathOf(body),text=files.get(path)?.text,was=originals.get(path);
        return {hunks:text&&was!==undefined&&text!==was?[{at:1,lines:text.trimEnd().split('\n').length,was:was.trimEnd().split('\n')}]:[],more:false};
      }
      case 'git_history':{
        const history=commits(sourceProject(body));return {head:history[0].id,commits:history,next:null};
      }
      case 'git_log':return {commits:commits(sourceProject(body)).map(commit=>({...commit,body:commit.subject})),more:false};
      case 'git_blame':return {commits:[],runs:[]};
      case 'git_show':return {text:originals.get(pathOf(body))??fileAt(body).text,cut:false,diff:{hunks:[],more:false}};
      case 'git_commit':{
        const project=sourceProject(body),commit=commits(project).find(item=>item.id===body.rev)??commits(project)[0];
        return {commit:{...commit,body:commit.subject},total:project.files.length,more:false,files:project.files.map(file=>({path:file.path,from:'',status:'A',added:(file.before??file.text).trimEnd().split('\n').length,removed:0,binary:false,big:false,capped:false,before:'',after:file.before??file.text}))};
      }
      case 'code_here':return {version:'',ready:false};
      case 'code_windows':case 'git_windows':case 'dir_pins':case 'web_pins':return [];
      case 'trash_here':return false;
      default:return base.request(command,body);
    }
  }};
}
