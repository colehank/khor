import {mergeConfig} from 'vite';import base from '../vite.config';
let active='image';const tabs=[{id:'image',device:'fixture',url:'/photo.svg',title:'',view:'file',session:'switch-fixture',pinned:false,touched:2},{id:'markdown',device:'fixture',url:'/README.md',title:'',view:'file',session:'switch-fixture',pinned:false,touched:1}];
const svg=Buffer.from('<svg xmlns="http://www.w3.org/2000/svg" width="500" height="700"><rect width="500" height="700" fill="navy"/><text x="30" y="90" fill="white" font-size="42">IMAGE</text></svg>').toString('base64');
export default mergeConfig(base,{plugins:[{name:'switch-fixture',configureServer(server){server.middlewares.use(async(req,res,next)=>{const path=req.url?.split('?')[0];if(!path?.startsWith('/api/'))return next();let body='';for await(const chunk of req)body+=chunk;const args=body?JSON.parse(body):{};let result:unknown=null;
if(path==='/api/tabs_list')result={tabs,active};else if(path==='/api/tabs_focus'){active=args.id;await new Promise(r=>setTimeout(r,180));result={tabs,active};}
else if(path==='/api/preview')result={kind:'image',media:'image/svg+xml',data:svg,total:500,cut:false};
else if(path==='/api/read_text')result={text:'# README\n\n这是 Markdown 源码，图片必须退场。',total:100,at_ms:1,cut:false,binary:false,digest:'0'.repeat(64)};
else if(path==='/api/file_stat')result={total:100,at_ms:1,digest:'0'.repeat(64)};
else if(path==='/api/ls')result={path:'/',entries:[{name:'photo.svg',dir:false},{name:'README.md',dir:false}],truncated:false};
else if(path==='/api/page_icons')result={};else if(path==='/api/reveal_here')result=false;
res.setHeader('Content-Type','application/json');res.end(JSON.stringify(result));});}}],optimizeDeps:{entries:['lab/content-switch.html']}});
