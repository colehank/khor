import {spawn} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const gui=fileURLToPath(new URL('..',import.meta.url));
const server=spawn(process.execPath,[fileURLToPath(new URL('../node_modules/vite/bin/vite.js',import.meta.url)),'--config','scripts/session-navigation-preview.vite.ts'],{cwd:gui,stdio:['ignore','pipe','pipe'],env:{...process.env,KHOR_FACE_GATE:'1'}});
let log='';for(const stream of [server.stdout,server.stderr])stream.on('data',chunk=>{log=(log+chunk).slice(-8000);});
try{
 const deadline=Date.now()+30000;let ready=false;
 while(Date.now()<deadline){if(server.exitCode!==null)throw new Error(log);try{const r=await fetch('http://localhost:23805/');if(r.ok){ready=true;break;}}catch{}await new Promise(r=>setTimeout(r,100));}
 if(!ready)throw new Error('Session fixture did not become ready: '+log);
 await import('./session-navigation/ui.mjs');
}finally{server.kill('SIGTERM');await new Promise(resolve=>{if(server.exitCode!==null)return resolve();server.once('exit',resolve);setTimeout(()=>{server.kill('SIGKILL');resolve();},3000).unref();});}
