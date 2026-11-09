import type {Avatar} from '../../src/gen/bindings/Avatar';
import type {DeviceRow} from '../../src/gen/bindings/DeviceRow';
import type {AccountRing} from '../../src/gen/bindings/AccountRing';
import type {WorkspaceRow} from '../../src/gen/bindings/WorkspaceRow';
import type {Usage} from '../../src/gen/bindings/Usage';
import type {GitCommitSummary} from '../../src/gen/bindings/GitCommitSummary';
import type {DirRow} from '../../src/gen/bindings/DirRow';
import type {SystemKind} from '../../src/gen/bindings/SystemKind';
import type {ModelKind} from '../../src/gen/bindings/ModelKind';
import type {VitalsReading} from '../../src/gen/bindings/VitalsReading';
import type {About} from '../../src/gen/bindings/About';
import type {RelayChoices} from '../../src/gen/bindings/RelayChoices';
import type {DoctorRow} from '../../src/gen/bindings/DoctorRow';
import type {RoadShare} from '../../src/gen/bindings/RoadShare';
import type {Ticket} from '../../src/gen/bindings/Ticket';
import type {ConfigRow} from '../../src/gen/bindings/ConfigRow';
import generated from './data/world.generated.json';

const native = generated as {machine_ids:string[];avatars:Record<string,Avatar>;directories:Record<string,{entries:DirRow[];truncated:boolean}>;documents:Record<string,string>;pairing_ticket:Ticket;settings:Record<'zh'|'en',ConfigRow[]>};
// Explicit synthetic readings: the Lab never diagnoses the actual host.
export const labAbout:About = {version:'0.0.0-lab',root:'/preview/.khor',paused:false,store_bytes:0,faces:[]};
export const labRelays:RelayChoices = {mine:[],keyed:[],in_use:[],only_mine:false,preferred:null};
export const labRoadShare:RoadShare = {direct:0,settled:0,unread:'no-serve'};
export const labDoctor:DoctorRow[] = [{key:'lab',name:'预览环境',state:'unmeasured',said:'此预览不检查真实机器',group:'lab',group_label:'预览环境',fix:null}];
export const labSettings = native.settings;
export const labPairingTicket = native.pairing_ticket;

export function labDirectory(path: string) {
  const directory=native.directories[path];
  if(!directory)throw new Error('No generated fixture directory: '+path);
  return directory;
}

export function labText(path: string) {
  const text=native.documents[path];
  if(text===undefined)throw new Error('No generated fixture file: '+path);
  return text;
}

export function labRoot(device: DeviceRow|undefined) {
  if(!device||device.me)return '/preview';
  switch(device.vitals?.vitals.system){
    case 'windows':return 'C:/Users/lab/projects';
    case 'macos':return '/Users/lab/projects';
    case 'ios':return '/var/mobile/Documents/Lab';
    case 'android':return '/storage/emulated/0/Documents/Lab';
    default:return '/home/lab/projects';
  }
}

export function labModels(provider: string) {
  const models = provider==='deepseek'
    ? [['deepseek-chat','DeepSeek Chat',false,false],['deepseek-reasoner','DeepSeek Reasoner',true,false]]
    : provider==='openrouter'
      ? [['openai/gpt-5.4','GPT-5.4',true,true],['anthropic/claude-sonnet-4-6','Claude Sonnet 4.6',true,true]]
      : [['claude-sonnet-4-6','Claude Sonnet 4.6',true,true],['claude-opus-4-6','Claude Opus 4.6',true,true]];
  return models.map(([id,name,reasoning,image])=>({id:String(id),name:String(name),reasoning:!!reasoning,image:!!image,contextWindow:provider==='deepseek' ? 128000 : 200000}));
}

export function labDevices(base: DeviceRow, count: number): DeviceRow[] {
  const names = ['验收环境','HPC-Linux','Mac mini 构建机','Windows 笔记本','iPhone','Android 平板'];
  const systems:SystemKind[] = ['macos','linux','macos','windows','ios','android'];
  const kinds:ModelKind[] = ['macbook','server','mac-mini','unknown','unknown','unknown'];
  const readings = (index:number):VitalsReading => ({vitals:{system:systems[index],model:{kind:kinds[index],name:names[index]},
    cpu_pct:8+index*11,cores:index===1?64:8,mem:{used:(4+index)*1024**3,total:(index===1?128:16)*1024**3},
    disk:{used:(80+index*20)*1024**3,total:512*1024**3},gpu:null,gpus:[],volumes:[],uptime_s:86400*(index+1),version:null},
    age_ms:index===3?3_600_000:1000,mem_strain:null,disk_strain:null,vram_strain:null,volume_strains:[],stale:index===3?['cpu','mem','disk']:[]});
  return Array.from({length:count},(_,index)=>({...base,
    id:native.machine_ids[index],
    name:names[index],shown:names[index],me:index===0,face:native.avatars[native.machine_ids[index]],vitals:readings(index),
    pinned:index===1,paused:index===5,hosts:index<4,
    hop:index===0 ? null : index===1 ? 'direct' : index===2 ? 'relay' : 'quiet',
    heard_ms:index===0 ? null : index===3 ? 3_600_000 : 1000,
    rtt_ms:index===1 ? 12 : index===2 ? 146 : null,
    down_ms:index===3 ? 3_600_000 : null,last_hop:index===0 ? null : index===2 ? 'relay' : 'direct',
    last_rtt_ms:index===0 ? null : index===2 ? 146 : 12,since_ms:index===0 ? null : 1_800_000,
    relay:index===2 ? 'relay.example.test:443' : null,seat:{left:20+index*11,top:30+(index%3)*18},
  }));
}

export function labQuota(devices: DeviceRow[]): AccountRing[] {
  if (!devices.length) return [];
  const now = Date.now(), seconds = Math.floor(now/1000);
  const samples = [
    {id:'fixture',name:'Claude Max · 日常开发',provider:'claude',fill:.36,kind:'sub',plan:'Max 20x',remaining:null,trouble:null},
    {id:'lab-codex',name:'Codex · 快到限额',provider:'codex',fill:.94,kind:'sub',plan:'Pro',remaining:null,trouble:null},
    {id:'lab-deepseek',name:'DeepSeek · 预付余额',provider:'deepseek',fill:null,kind:'api',plan:null,remaining:128.64,trouble:null},
    {id:'lab-openrouter',name:'OpenRouter · 月度预算',provider:'openrouter',fill:.72,kind:'api',plan:null,remaining:56,trouble:null},
    {id:'lab-newapi',name:'团队网关 · 用量已满',provider:'newapi',fill:1,kind:'api',plan:null,remaining:0,trouble:null},
    {id:'lab-offline',name:'远程账户 · 暂无读数',provider:'openai',fill:null,kind:'api',plan:null,remaining:null,trouble:'unreachable'},
  ];
  return samples.map((sample,index)=>{
    const home = devices[index%devices.length], sub = sample.kind==='sub';
    const budgeted=sample.provider==='openrouter',gateway=sample.provider==='newapi';
    return {home_name:home.name,mine:home.me,age_ms:home.me ? 0 : 240_000,strain:sample.fill===1 ? 'critical' : sample.fill!==null&&sample.fill>.9 ? 'tight' : null,
      base_url:null,budget:sample.id==='lab-openrouter' ? 200 : null,unit:sample.provider==='deepseek' ? 'CNY' : 'USD',pinned:index<2,kind:sample.kind,
      reading:{account:sample.id,name:sample.name,provider:sample.provider,home:home.id,fill:sample.fill,
        spent:budgeted ? 144 : gateway ? 100 : null,limit:budgeted ? 200 : gateway ? 100 : null,
        currency:sample.provider==='deepseek' ? 'CNY' : 'USD',unlimited:false,resets_at:sub ? seconds+7200 : null,
        trouble:sample.trouble,at_ms:now-(home.me?0:240_000),remaining:sample.remaining,expires_at:null,
        models:gateway ? ['gpt-5.4','claude-sonnet-4-6'] : [],days:[],
        spent_this_month:budgeted ? 144 : null,spent_all_time:budgeted ? 286.74 : null,
        model_days:[],log_capped:null,windows:sub ? [
          {span:'five_hour',scope:null,used_pct:(sample.fill??0)*100,resets_at:seconds+7200,active:true,severity:null},
          {span:'seven_day',scope:null,used_pct:index===0 ? 62 : 79,resets_at:seconds+3*86400,active:true,severity:null},
        ] : [],plan:sample.plan,credits:null,model_tokens:[],zone:'Asia/Shanghai'},
    } satisfies AccountRing;
  });
}

export function labUsage(): Usage {
  const today = new Date().toISOString().slice(0,10);
  const days = Array.from({length:14},(_,index)=>({day:new Date(Date.now()-(13-index)*86400_000).toISOString().slice(0,10),category:index%2 ? 'codex' : 'claude',
    tokens:{input:12_000+index*1700,cached_input:4800+index*500,cache_write:800,output:3200+index*290}}));
  return {today,unreadable:0,days,models:days.map(day=>({...day,model:day.category==='codex' ? 'gpt-5.4' : 'claude-sonnet-4-6'}))};
}

export function labCommits(root: string): GitCommitSummary[] {
  const subjects = ['保留工作区阅读位置','修复移动导航的松手确认','完善中文输入与撤销历史','Update session connection states','整理模型账户与额度展示','同步文件树与提交标签页'];
  return Array.from({length:86},(_,index)=>({id:(index+1).toString(16).padStart(40,'a'),
    subject:subjects[index%subjects.length]+(root.endsWith('project') ? '' : ' · '+root.split('/').pop()),
    author:['Lin','Kai','Morgan'][index%3],when_ms:Date.now()-index*3_600_000,cut:false}));
}

export function labWorkspaces(devices: DeviceRow[]): WorkspaceRow[] {
  const now = Date.now();
  return devices.flatMap((device,machine)=>['project','research-notes','design-system','资料库'].map((name,index)=>{
    const path = labRoot(device)+'/'+name;
    const branch = index===3 ? null : ['main','feat/mobile-navigation','research/long-branch-name-for-layout'][index];
    return {device:device.id,machine:device.name,path,name,pinned:machine===0&&index===0,branch,
      changed:index===0 ? 4 : index===1 ? 12 : 0,changed_capped:false,changed_ms:now-(machine+index)*60_000,
      git:branch ? {branch,head:'a'.repeat(39)+'1',subject:'保留工作区阅读位置',ahead:index+1,behind:machine,stashes:index,
        conflicted:0,staged:1,unstaged:2,untracked:1,capped:false,since_ms:now-3_600_000,operation:null} : null,
      newest:branch ? null : {path:'会议记录.md',at_ms:now-90_000,added:false},
    } satisfies WorkspaceRow;
  }));
}
