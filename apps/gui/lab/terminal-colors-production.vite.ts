import {mergeConfig,type Plugin} from 'vite';
import {fileURLToPath} from 'node:url';
import base from '../vite.config';
const counts:Record<string,number>={};
const text=Array.from({length:140},(_,i)=>`row ${i} · 中文 terminal history\r\n`).join('')+'\x1b[32mSuccess\x1b[0m · \x1b[93mWarning\x1b[0m\r\n\x1b[48;2;31;31;31m > Ask Codex to do anything                        \x1b[0m\r\n\x1b[38;2;128;160;200mRGB foreground\x1b[0m\r\n';
const fixture:Plugin={name:'terminal-colour-proof',configureServer(s){s.middlewares.use(async(req,res,next)=>{
 if(req.url==='/colour-proof-counts'){res.setHeader('Content-Type','application/json');return res.end(JSON.stringify(counts));}
 if(req.method!=='POST')return next();
 const cmd=req.url?.slice(1).split('?')[0]||'';counts[cmd]=(counts[cmd]||0)+1;
 for await(const _chunk of req){};
 let value:unknown=null;
 if(cmd==='term_attach')value={bytes:Buffer.from(text).toString('base64'),seq:1};
 else if(['term_wait','term_stream'].includes(cmd)){await new Promise(r=>setTimeout(r,500));value={bytes:'',seq:1,gone:false,reset:false,stall:null};}
 else if(cmd==='term_ended')value={ended:{exit:0},recent:[],screen:{cols:80,rows:24,at_ms:1,bytes:Buffer.from(text).toString('base64')}};
 res.setHeader('Content-Type','application/json');res.end(JSON.stringify(value));
 });}};
export default mergeConfig(base,{cacheDir:fileURLToPath(new URL('../../../target/vite-terminal-colours-proof/',import.meta.url)),server:{host:'127.0.0.1',port:23624,strictPort:true},optimizeDeps:{entries:['lab/terminal-colors-production.html']},plugins:[fixture]});
