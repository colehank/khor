// Shared dialog regression: overflow, compact import, and menu pan ownership.
// --serve exposes the same disposable fixture for a CUA / native device walk.
import assert from 'node:assert/strict';
import {createServer} from 'vite';
import {fileURLToPath} from 'node:url';
import {tmpdir} from 'node:os';
import path from 'node:path';
const root=fileURLToPath(new URL('..',import.meta.url));
const port=Number(process.env.DIALOGS_PORT||23604);
let presets=Array.from({length:15},(_,i)=>({id:'preset-'+i,name:'工作流 '+(i+1),command:'codex --profile project-'+(i+1)}));
const catalog=Array.from({length:40},(_,i)=>({id:'agent-'+i,name:['OpenCode','Poolside','Mistral Vibe'][i%3]+' '+(i+1),about:'The open source coding agent',argv:i%3===0?null:'agent-'+i}));
const candidates=Array.from({length:30},(_,i)=>({id:'candidate-'+i,form:'shell',machine:'fixture',device:'fixture',title:'终端 '+(i+1),place:'~/project-'+i,at_ms:Date.now()-i*60000,category:null,agent:null,command:'zsh'}));
const html=`<!doctype html><html data-theme="dark"><meta charset="utf-8"><meta name="viewport" content="width=device-width,initial-scale=1"><body><div id="root"></div><script type="module">import RefreshRuntime from '/@react-refresh';RefreshRuntime.injectIntoGlobalHook(window);window.$RefreshReg$=()=>{};window.$RefreshSig$=()=>type=>type;window.__vite_plugin_react_preamble_installed__=true;</script><script type="module" src="/scripts/fixtures/dialog-bounds.tsx"></script></body></html>`;
const wrapper=`<!doctype html><meta charset="utf-8"><style>body{margin:0;background:#111;color:#ddd;font:14px system-ui}header{height:42px;display:flex;gap:12px;align-items:center;padding:0 20px}button{padding:4px 12px}iframe{display:block;border:1px solid #555;width:1080px;height:680px;margin:auto}</style><header>正式弹窗组件验收 <button onclick="size(1080,680)">桌面</button><button onclick="size(900,440)">低窗口</button><button onclick="size(390,760)">手机</button><button onclick="size(390,400)">手机短视口</button><button onclick="let d=document.querySelector('iframe').contentDocument.documentElement;d.dataset.theme=d.dataset.theme==='dark'?'light':'dark'">浅深主题</button></header><iframe src="/dialog-frame?bridge=${port}"></iframe><script>function size(w,h){let f=document.querySelector('iframe');f.style.width=w+'px';f.style.height=h+'px'}</script>`;
const server=await createServer({root,configFile:path.join(root,'vite.config.ts'),cacheDir:path.join(tmpdir(),'khor-dialogs-vite'),server:{port,host:'127.0.0.1',strictPort:true},optimizeDeps:{entries:['scripts/fixtures/dialog-bounds.tsx']},plugins:[{name:'disposable-dialog-fixture',configureServer(s){s.middlewares.use(async(req,res,next)=>{
  if(req.url.startsWith('/dialog-frame')){res.setHeader('Content-Type','text/html');return res.end(html);}
  if(req.url==='/dialog-proof'){res.setHeader('Content-Type','text/html');return res.end(wrapper);}
  if(req.method!=='POST')return next();
  let body='';for await(const chunk of req)body+=chunk;
  const args=body?JSON.parse(body):{},cmd=req.url.slice(1);
  let result;
  if(cmd==='devices')result=[];
  else if(cmd==='agents')result=[{name:'claude-acp'},{name:'codex-acp'}];
  else if(cmd==='agent_seen')result={said:'unasked',rows:[]};
  else if(cmd==='launch_presets')result=presets;
  else if(cmd==='launch_preset_save'){presets=presets.filter(p=>p.id!==args.preset.id).concat(args.preset);result=presets;}
  else if(cmd==='launch_preset_delete'){presets=presets.filter(p=>p.id!==args.id);result=presets;}
  else if(cmd==='agents_known')result=catalog;
  else if(cmd==='agents_known_taken')result=['2026-08-30',40,40];
  else if(cmd==='import_candidates')result=candidates;
  else if(cmd==='ls')result={path:'/home/fixture/',entries:Array.from({length:40},(_,i)=>({name:'project-'+(i+1),dir:true})),truncated:false};
  else {res.statusCode=500;return res.end(JSON.stringify('Unexpected fixture command: '+cmd));}
  res.setHeader('Content-Type','application/json');res.end(JSON.stringify(result));
});}}]});
await server.listen();
if(process.argv.includes('--serve')){
  console.log(`Dialog fixture: http://127.0.0.1:${port}/dialog-proof`);
  for(const signal of ['SIGINT','SIGTERM'])process.on(signal,async()=>{await server.close();process.exit(0)});
}else{
  let browser;
  try{
    const {chromium}=await import('playwright-core');
    browser=await chromium.launch({channel:'chrome',headless:true});
    const page=await browser.newPage({viewport:{width:1100,height:820}});
    await page.goto(`http://127.0.0.1:${port}/dialog-proof`);
    const frame=page.frameLocator('iframe');
    const bounded=async()=>{
      const b=await frame.getByRole('dialog').evaluate(el=>{
        const box=el.getBoundingClientRect(),footer=el.querySelector('[data-slot="dialog-footer"]')?.getBoundingClientRect();
        return {top:box.top,bottom:box.bottom,right:box.right,height:innerHeight,width:innerWidth,footer:footer?.bottom};
      });
      assert(b.top>=-1&&b.bottom<=b.height+1&&b.right<=b.width+1,'dialog escaped viewport: '+JSON.stringify(b));
      if(b.footer!==undefined)assert(b.footer<=b.bottom+1,'actions escaped dialog');
    };
    for(const size of ['桌面','低窗口','手机','手机短视口']){
      await page.getByRole('button',{name:size,exact:true}).click();
      await frame.getByRole('combobox',{name:'启动方式',exact:true}).click();
      await frame.getByRole('option',{name:'管理启动方式',exact:true}).click();
      await bounded();
      await frame.getByRole('button',{name:'接入智能体',exact:true}).click();
      await frame.getByRole('textbox',{name:'搜索智能体'}).fill('');
      await frame.getByText('OpenCode 40',{exact:true}).click();
      await bounded();
      await frame.getByRole('textbox',{name:'搜索智能体'}).fill('Poolside 38');
      assert.equal(await frame.locator('[data-launch-catalog-rows] > div').count(),1);
      await frame.getByRole('button',{name:'返回',exact:true}).click();
      await frame.getByRole('button',{name:'完成',exact:true}).click();
      await frame.getByRole('button',{name:'导入会话',exact:true}).click();
      await frame.locator('[data-import-row]').last().waitFor();
      await bounded();
      if(size.startsWith('手机')){
        const ratio=await frame.getByRole('dialog').evaluate(el=>el.getBoundingClientRect().height/innerHeight);
        assert(ratio<=.71,'import unexpectedly full-screen');
        assert.equal(await frame.locator('[data-import-field]').evaluate(el=>el===document.activeElement),false);
      }
      const fitted=await frame.getByRole('dialog').evaluate(el=>el.getBoundingClientRect().height);
      await frame.getByRole('searchbox',{name:'搜索可导入的会话'}).fill('终端 30');
      assert(Math.abs(await frame.getByRole('dialog').evaluate(el=>el.getBoundingClientRect().height)-fitted)<1,'filter moved the import sheet');
      await frame.getByRole('button',{name:'取消',exact:true}).click();
    }
    await page.getByRole('button',{name:'手机短视口',exact:true}).click();
    await frame.getByRole('button',{name:'目录选择',exact:true}).click();
    const directory=frame.getByRole('combobox',{name:'目录',exact:true});
    await directory.fill('~/project-4');
    await frame.locator('[data-path-suggestions] button').waitFor();
    await directory.fill('~/');
    await frame.locator('[data-path-suggestions] button').last().waitFor();
    const menu=await frame.getByRole('dialog').evaluate(el=>{
      const body=el.querySelector('[data-slot="dialog-body"]').getBoundingClientRect();
      const menu=el.querySelector('[data-menu-viewport]').getBoundingClientRect();
      return {bottom:menu.bottom,limit:body.bottom,height:menu.height};
    });
    assert(menu.bottom<=menu.limit+1&&menu.height>100,'menu clipped or failed to regrow');
    await frame.getByRole('button',{name:'取消',exact:true}).click();
    await page.getByRole('button',{name:'手机',exact:true}).click();
    await frame.getByRole('button',{name:'手势检查',exact:true}).click();
    await frame.getByRole('button',{name:'上滑候选菜单',exact:true}).click();
    assert.deepEqual(JSON.parse(await frame.locator('[data-pan-result]').textContent()),{prevented:false,dragged:false,closed:false});
    await frame.getByRole('button',{name:'上滑主内容',exact:true}).click();
    assert.equal(JSON.parse(await frame.locator('[data-pan-result]').textContent()).dragged,true);
    await frame.getByRole('button',{name:'路径表单',exact:true}).click();
    await frame.getByRole('combobox').fill('~/');
    await frame.locator('[data-path-suggestions]').waitFor();
    await frame.getByRole('button',{name:'下滑推荐菜单',exact:true}).click();
    const held=JSON.parse(await frame.locator('[data-form-pan-result]').textContent());
    assert.equal(held.prevented,false);assert.equal(held.pull,'');
    await frame.getByRole('button',{name:'取消',exact:true}).click();
    await frame.locator('[data-pan-form]').waitFor({state:'hidden'});
    console.log('PASS: dialog viewport bounds, fixed actions, compact import, menu pan isolation and main-content handoff');
  }finally{await browser?.close();await server.close();}
}
