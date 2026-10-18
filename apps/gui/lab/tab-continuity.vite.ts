import {mergeConfig,type Plugin} from 'vite';
import {fileURLToPath} from 'node:url';
import base from '../vite.config';
import fixtureData from './tab-continuity-fixture.json';
const port=23619;
const notes='# Session tabs\n\nEach new tab keeps its own directory, address and draft.\n\n## Motion\n\n- Selection responds immediately\n- A repeated press stays still\n- Closing chooses the adjacent tab\n';
const paths:Record<string,string[]>={'/Users/zgh/Projects/khor':['apps/','crates/','docs/','scripts/','AGENTS.md','Cargo.toml','README.md'],'/Users/zgh/Projects/khor/docs':['design-system/','SHIP.md','ARCHITECTURE.md'],'/Users/zgh/Projects/khor/apps':['gui/'],'/Users/zgh/Projects/khor/apps/gui':['src/','package.json'],'/Users/zgh/Projects/khor/apps/gui/src':['App.tsx','main.tsx','styles/']};
const stamp=Date.now();let sequence=0;
const fixture:Plugin={name:'tab-continuity-fixture',configureServer(s){s.middlewares.use(async(req,res,next)=>{
 if(req.url==='/tab-lab'){
 res.setHeader('Content-Type','text/html');return res.end(`<!doctype html><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><title>Khor · 标签连续性 lab</title><style>body{margin:0;background:#121212;color:#bbb;font:13px system-ui}header{display:flex;align-items:center;justify-content:center;gap:12px;padding:14px;flex-wrap:wrap}button{font:inherit;color:inherit;background:#222;border:1px solid #444;border-radius:7px;padding:5px 10px}iframe{display:block;width:min(1280px,calc(100vw - 24px));height:800px;border:1px solid #444;border-radius:16px;margin:auto}small{display:block;text-align:center;margin:12px}</style><header><b>标签连续性</b><button onclick="size(1280,800)">桌面</button><button onclick="size(390,800)">窄屏</button><button onclick="scenario('empty')">空白起步</button><button onclick="scenario('crowded')">拥挤标签</button><button onclick="scenario('default')">重置</button><button onclick="scenario('halo')">头像状态环</button><button onclick="document.querySelector('iframe').contentWindow.postMessage('theme','*')">浅深主题</button></header><iframe title="Khor 标签 lab" src="/lab/tab-continuity.html?bridge=${port}"></iframe><small>隔离数据 · 正式组件 · 不连接现有会话</small><small id="motion" data-motion-runs="0">切换已有标签可查看动效；重复点击、＋和 × 不跳转</small><script>addEventListener('message',e=>{if(e.source!==document.querySelector('iframe').contentWindow||e.data?.type!=='lab-motion')return;let out=document.getElementById('motion');out.dataset.motionRuns=e.data.runs;out.textContent=e.data.layer+'切换 · '+e.data.ms+' ms · 第 '+e.data.runs+' 次'});function size(w,h){let f=document.querySelector('iframe');f.style.width='min('+w+'px,calc(100vw - 24px))';f.style.height=h+'px'}function scenario(v){document.querySelector('iframe').src='/lab/tab-continuity.html?bridge=${port}&scenario='+v+'&reset='+Date.now()}</script>`);
 }
 if(req.url?.startsWith('/lab-site')){res.setHeader('Content-Type','text/html');return res.end(`<!doctype html><meta charset="utf-8"><style>body{font:15px system-ui;background:#f5f5f1;color:#242721;max-width:680px;padding:40px;margin:auto;line-height:1.8}a{color:#46661b}h1{font-size:30px}</style><p>Khor / Design</p><h1>留在同一个标签页</h1><p>新建、浏览目录、打开内容，是同一枚标签连续发生的变化。</p><p>这个网页是 lab 的本地内容夹具，地址栏、工作区、文件阅读器和标签控件复用现有前端。</p>${Array.from({length:15},(_,i)=>`<h2>章节 ${i+1}</h2><p>切换其他标签后回来，页面仍停留在你刚才阅读的位置。每个标签保存自己的浏览位置和输入。</p>`).join('')}`);}
 if(req.method!=='POST')return next();
 let body='';for await(const chunk of req)body+=chunk;let a:Record<string,any>={};try{a=JSON.parse(body||'{}')}catch{}
 const cmd=req.url?.replace(/^\/(api\/)?/,'').split('?')[0];let value:unknown;
 if(cmd==='devices')value=fixtureData.devices;
 else if(cmd==='ls'){const path=String(a.path||'/Users/zgh/Projects/khor').replace(/\/$/,'');value={path,entries:(paths[path]||['README.md','notes.md']).map(name=>({name:name.replace(/\/$/,''),dir:name.endsWith('/'),size:notes.length,at_ms:stamp,from:null,received_ms:null})),truncated:false};}
 else if(cmd==='checkout')value=true;
 else if(cmd==='git_status')value={branch:'main',rows:[],more:false,ahead:0,behind:0,fetched_ms:0};
 else if(cmd==='git_log')value={commits:[],more:false};
 else if(cmd==='git_diff')value={rows:[],hunks:[],text:'',base:'',head:''};
 else if(cmd==='git_blame')value={lines:[],commits:{}};
 else if(cmd==='read_text'||cmd==='stat_text')value={text:String(a.path).endsWith('.tsx')?'export function App() {\n  return <main>khor</main>;\n}\n':notes,total:notes.length,at_ms:stamp,digest:'lab-digest',cut:false,binary:false};
 else if(cmd==='dirs_wait'){await new Promise(r=>setTimeout(r,1500));value={changed:[],since:0};}
 else if(cmd==='recents')value=a.kind==='page'?[{device:'mac',name:'MacLab',place:'https://khor.app/design',at_ms:stamp,session:null},{device:'linux',name:'turing',place:'https://example.org/research',at_ms:stamp-60000,session:null}]:[];
 else if(cmd==='search_in')value={names:['/Users/zgh/Projects/khor/README.md'],lines:[],more_names:false,more_lines:false};
 else if(cmd==='agents')value=[];
 else if(cmd==='launch_presets')value=[];
 else if(cmd==='agent_seen')value={said:'unasked',rows:[]};
 else if(cmd==='agents_known')value=[];
 else if(cmd==='agents_known_taken')value=['fixture',0,0];
 else if(cmd==='open_shell'||cmd==='open_session')value='shell/lab-'+(++sequence);
 else if(['forget_recent_by_device','recent_visit','page_step','write_text','merge_text','page_deck'].includes(cmd||''))value=null;
 else {res.statusCode=400;res.setHeader('Content-Type','application/json');return res.end(JSON.stringify('Lab fixture has no '+cmd));}
 res.setHeader('Content-Type','application/json');res.end(JSON.stringify(value));
 });}};
export default mergeConfig(base,{cacheDir:fileURLToPath(new URL('../../../target/vite-tab-continuity/',import.meta.url)),server:{host:'127.0.0.1',port,strictPort:true},optimizeDeps:{entries:['lab/tab-continuity.html']},plugins:[fixture]});
