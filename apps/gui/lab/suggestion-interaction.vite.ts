import {mergeConfig} from 'vite';
import base from '../vite.config';
export default mergeConfig(base,{plugins:[{name:'suggestion-fixture',configureServer(server){server.middlewares.use((req,res,next)=>{const path=req.url?.split('?')[0];if(!path?.startsWith('/api/'))return next();let result:unknown=null;
if(path==='/api/recents')result=Array.from({length:12},(_,i)=>({device:'fixture',machine:'fixture',kind:'page',place:`https://example.org/page-${i+1}`,at_ms:Date.now()-i*60000}));
else if(path==='/api/ls')result={path:'/home/fixture/',entries:Array.from({length:30},(_,i)=>({name:'project-'+(i+1),dir:true})),truncated:false};
res.setHeader('Content-Type','application/json');res.end(JSON.stringify(result));});}}],optimizeDeps:{entries:['lab/suggestion-interaction.html']}});
