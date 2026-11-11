import type {Load} from '../../src/gen/bindings/Load';
import type {Usage} from '../../src/gen/bindings/Usage';
import type {Tokens} from '../../src/gen/bindings/Tokens';
import {GiB,machines} from './world';

// Fixed samples, not live host measurements. Every view uses these same
// readings, including sums, the final trend bucket and the process table.
const round=(n:number)=>Math.round(n*10)/10;
const pulse=(x:number,center:number,width:number)=>Math.exp(-.5*((x-center)/width)**2);
const noise=(i:number,salt=0)=>{const x=Math.sin((i+1)*127.1+salt*311.7)*43758.5453;return x-Math.floor(x);};
const zero=():Tokens=>({input:0,output:0,cached_input:0,cache_write:0});

export function siteUsage(at:number):Usage {
  const today=new Intl.DateTimeFormat('sv-SE',{timeZone:'Asia/Shanghai'}).format(at);
  const midnight=Date.parse(today+'T12:00:00+08:00');
  const specs=[
    {model:'claude-sonnet-4-6',category:'claude',days:30,base:132000,cache:3.8,output:.24,start:0,end:29},
    {model:'claude-opus-4-6',category:'claude',days:22,base:178000,cache:2.4,output:.38,start:5,end:29},
    {model:'claude-haiku-4-5',category:'claude',days:26,base:48000,cache:1.7,output:.18,start:0,end:29},
    {model:'claude-sonnet-4-5',category:'claude',days:18,base:86000,cache:2.8,output:.26,start:0,end:22},
    {model:'claude-opus-4-5',category:'claude',days:11,base:120000,cache:2.1,output:.42,start:0,end:17},
    {model:'claude-3-5-haiku',category:'claude',days:7,base:39000,cache:1.2,output:.14,start:0,end:13},
    {model:'gpt-5.4',category:'codex',days:25,base:156000,cache:3.1,output:.32,start:0,end:29},
    {model:'gpt-5.4-mini',category:'codex',days:16,base:54000,cache:1.9,output:.22,start:2,end:29},
  ];
  const models:Usage['models']=[];
  specs.forEach((spec,m)=>{
    const active=new Set(Array.from({length:spec.end-spec.start+1},(_,i)=>i+spec.start)
      .sort((a,b)=>noise(b,m+3)-noise(a,m+3)).slice(0,spec.days));
    for(let d=0;d<30;d++){
      if(!active.has(d))continue;
      const date=new Date(midnight-(29-d)*86400000),weekday=date.getUTCDay();
      const workday=weekday===0||weekday===6 ? .34 : 1;
      const launches=1+.7*pulse(d,7,1.4)+1.15*pulse(d,17,1.9)+.9*pulse(d,26,1.2);
      const input=Math.round(spec.base*workday*launches*(.65+.6*noise(d,m+9)));
      const tokens={input,output:Math.round(input*spec.output*(.8+.4*noise(d,m+11))),
        cached_input:Math.round(input*spec.cache*(.7+.6*noise(d,m+17))),cache_write:Math.round(input*(.13+.12*noise(d,m+19)))};
      models.push({day:new Intl.DateTimeFormat('sv-SE',{timeZone:'Asia/Shanghai'}).format(date),category:spec.category,model:spec.model,tokens});
    }
  });
  models.sort((a,b)=>a.day.localeCompare(b.day)||a.category.localeCompare(b.category)||a.model!.localeCompare(b.model!));
  const summed=new Map<string,Usage['days'][number]>();
  for(const row of models){const key=row.day+'/'+row.category;const day=summed.get(key)??{day:row.day,category:row.category,tokens:zero()};
    for(const key of Object.keys(day.tokens) as (keyof Tokens)[])day.tokens[key]+=row.tokens[key];summed.set(key,day);}
  return {today,unreadable:0,models,days:[...summed.values()]};
}

export function siteLoad(index:number,at:number,by:unknown,desc:unknown,limit:unknown):Load {
  const profile=machines[index];
  const background=profile.system==='linux'?['systemd','sshd','nginx','containerd','journald','cron','bash','tmux','systemd-resolved','python3']:
    profile.system==='windows'?['System','explorer.exe','MsMpEng.exe','dwm.exe','svchost.exe','OneDrive.exe','conhost.exe','RuntimeBroker.exe','SearchIndexer.exe','spoolsv.exe']:
    profile.system==='ios'?['SpringBoard','backboardd','mediaserverd','sharingd','cloudd','mobileassetd','secd','locationd','bluetoothd','notifyd']:
    profile.system==='android'?['system_server','surfaceflinger','zygote64','audioserver','media.codec','netd','vold','logd','keystore2','com.android.settings']:
    ['launchd','WindowServer','mds_stores','cfprefsd','cloudd','kernel_task','distnoted','trustd','nsurlsessiond','sharingd',
      'logd','notifyd','opendirectoryd','runningboardd','powerd','coreaudiod','bluetoothd','locationd','accountsd','securityd',
      'networkserviceproxy','syspolicyd','sandboxd','diskarbitrationd','fseventsd','tccd','mediaanalysisd','bird','rapportd','Finder','Dock','ControlCenter'];
  const foreground:[string,number,number][]=[...profile.processes.map(([name,cpu,mem]):[string,number,number]=>[name,cpu,mem])];
  if(index===0)foreground.push(['tsserver',12.4,.84],['Claude',8.2,.63],['Code Helper (Renderer)',5.6,.76],['com.apple.WebKit.WebContent',3.9,.45],['Electron Helper (GPU)',2.7,.32],['npm test',18.6,.52],['vitest worker',26.4,.41],['Spotlight',.7,.22]);
  const processes:[string,number,number][]=[...foreground,...Array.from({length:32},(_,i)=>[
    background[i%background.length],
    .025+Math.exp(noise(i,index+31)*3.5)*.12,.012+Math.exp(noise(i,index+47)*3.8)*.007,
  ] as [string,number,number])];
  const weight=processes.reduce((sum,row)=>sum+row[1],0);
  const top:Load['top']=processes.map(([name,cpu,mem],i)=>({pid:1800+index*100+i,name,
    user:i>=foreground.length?'root':'lin',cpu_pct:round(cpu/weight*profile.cpu*profile.cores),mem:Math.round(mem*GiB),
    gpu_pct:profile.gpu===null?null:round(i===2?profile.gpu*.72:i===8?profile.gpu*.23:.01),gpu_mem:null,
    run_secs:Math.round(60+Math.exp(noise(i,index+53)*7)*90),arg:name.includes('node')?(index===1?'src/server.ts':'vite'):null,
    ppid:i<foreground.length?1:1800+index*100,status:i<3?'run':'sleep',cpu_ms:Math.round(cpu*24000),io_bytes:Math.round((.2+noise(i,61)*noise(i,67)*1800)*1024**2)}));
  const field=by==='mem'?'mem':by==='pid'?'pid':'cpu_pct';
  top.sort((a,b)=>(desc===false?1:-1)*(a[field]-b[field]));
  const last=287;
  const history=(kind:'cpu'|'mem'|'gpu'|'disk',current:number)=>Array.from({length:288},(_,i)=>{
    if(i===last)return current;
    const hour=(new Date(at).getUTCHours()+8-24+i/12+24)%24;
    const work=pulse(hour,10.4,.75)+.85*pulse(hour,14.7,1.25)+.5*pulse(hour,20.2,.8);
    const compile=pulse(i,108+index*4,3.5)+.8*pulse(i,169-index*3,5)+.7*pulse(i,231,2.7);
    const irregular=noise(i,index+71)-.5;
    const value=kind==='cpu'?3.2+27*work+51*compile+irregular*(3+13*work):
      kind==='gpu'?1+12*work+62*pulse(i,144,6)+28*pulse(i,237,4)+irregular*3:
      kind==='mem'?current-7+4.2*work+6.7*pulse(i,155,39)+irregular*.7:
      current-.22+(i/last)*.2+irregular*.025;
    // The final quarter-hour settles into the current instrument reading.
    const settle=Math.max(0,(i-(last-12))/12);
    return round(Math.min(96,Math.max(.3,value*(1-settle)+current*settle)));
  });
  return {top:top.slice(0,Number(limit)||200),cpu_day:{bucket_ms:300000,now_ms:at,
    buckets:history('cpu',profile.cpu),mem:history('mem',profile.mem[0]/profile.mem[1]*100),
    gpu:profile.gpu===null?[]:history('gpu',profile.gpu),disk:history('disk',profile.disk[0]/profile.disk[1]*100),vram:[]}};
}
