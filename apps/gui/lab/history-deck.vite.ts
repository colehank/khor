import {mergeConfig} from 'vite';
import base from '../vite.config';
const commits=Array.from({length:35},(_,i)=>({id:(i+1).toString(16).padStart(40,'a'),author:'作者',when_ms:Date.now()-i*60000,subject:`第 ${i+1} 次提交：整理工作区文档和交互`,body:'提交说明随内容滚动\n\n保留 Diff 本身的横向滚动。'}));
const before=Array.from({length:35},(_,i)=>`第 ${i+1} 行：`+'已有的文档内容 '.repeat(30)).join('\n');
export default mergeConfig(base,{plugins:[{name:'history-fixture',configureServer(server){server.middlewares.use(async(req,res,next)=>{const path=req.url?.split('?')[0];if(!path?.startsWith('/api/'))return next();let text='';for await(const chunk of req)text+=chunk;const args=text?JSON.parse(text):{};let result:unknown=null;
if(path==='/api/git_log')result={commits,more:false};
else if(path==='/api/git_commit')result={commit:commits.find(c=>c.id===args.rev)||commits[0],total:3,more:false,files:['docs/notes.md','src/index.ts','assets/diagram.png'].map((path,i)=>({path,from:'',status:'M',added:1,removed:1,binary:i===2,big:false,capped:false,before:i===2?'':before,after:i===2?'':before.replace('第 18 行：','更新第 18 行：')}))};
res.setHeader('Content-Type','application/json');res.end(JSON.stringify(result));});}}],optimizeDeps:{entries:['lab/history-deck-frame.html']}});
