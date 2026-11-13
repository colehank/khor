import assert from 'node:assert/strict';
import path from 'node:path';

// The real production navigation and tab strip, not replacement glass.
// Backdrop changes affect only the Lab's synthetic wallpaper plane.
export async function checkGlassSelection(browser,{port,report,engine}){
 const name='glass-selection-'+engine,context=await browser.newContext({viewport:{width:1280,height:900}}),page=await context.newPage(),errors=[],measurements=[];
 page.setDefaultTimeout(15000);page.on('pageerror',error=>errors.push(String(error)));
 const cases=[
  {theme:'light',material:'frost',opacity:0},
  {theme:'light',material:'frost',opacity:20},
  {theme:'light',material:'clear',opacity:20},
  {theme:'light',material:'frost',opacity:65},
  {theme:'light',material:'frost',opacity:100},
  {theme:'light',material:'solid',opacity:20},
  {theme:'dark',material:'frost',opacity:20},
 ];
 try{
  for(const choice of cases){
   const label=`${choice.theme}-${choice.material}-${choice.opacity}`;
   try{
    const environment={platform:'desktop',...choice,keyboard:0,safeTop:0,safeBottom:0,background:'light'};
    const url=new URL('/app/',`http://127.0.0.1:${port}`);url.searchParams.set('scene','gui');url.searchParams.set('machines','3');url.searchParams.set('environment',JSON.stringify(environment));
    await page.goto(url.href);await page.locator('[data-composer-text]').waitFor();await page.locator(`html[data-theme=${choice.theme}]`).waitFor();
    const selectors={navigation:'[data-landing-strip] [data-rail-item][data-on=true]',tab:'[data-tab-bar]:visible .glass-sel:visible'};
    const row={...choice,surfaces:{}};
    for(const [kind,selector] of Object.entries(selectors)){
     const pill=page.locator(selector).first();await pill.waitFor();
     row.surfaces[kind]=await pill.evaluate(el=>{
      const cs=getComputedStyle(el),canvas=document.createElement('canvas');canvas.width=canvas.height=1;
      const ctx=canvas.getContext('2d');ctx.fillStyle=cs.backgroundColor;ctx.fillRect(0,0,1,1);
      return {fill:cs.backgroundColor,alpha:ctx.getImageData(0,0,1,1).data[3]/255,opacity:cs.opacity};
     });
    }
    if(choice.theme==='light'&&choice.opacity===20&&choice.material!=='solid'){
     for(const background of ['light','dark']){
      await page.locator('[data-lab-environment-background]').evaluate((el,value)=>{el.dataset.background=value;},background);
      await page.evaluate(()=>new Promise(resolve=>requestAnimationFrame(()=>requestAnimationFrame(resolve))));
      for(const [kind,selector] of Object.entries(selectors)){
       const box=await page.locator(selector).first().boundingBox();assert(box,'A selected pill must have visible bounds');
       const clip={x:Math.ceil(box.x+6),y:Math.ceil(box.y+6),width:Math.floor(box.width-12),height:Math.floor(box.height-12)};
       const png=await page.screenshot({clip,animations:'disabled'});
       row.surfaces[kind][background]=await page.evaluate(async data=>{
        const image=new Image();image.src='data:image/png;base64,'+data;await image.decode();
        const canvas=document.createElement('canvas');canvas.width=image.width;canvas.height=image.height;
        const ctx=canvas.getContext('2d');ctx.drawImage(image,0,0);const bytes=ctx.getImageData(0,0,image.width,image.height).data;
        const mean=[0,0,0];for(let i=0;i<bytes.length;i+=4)for(let k=0;k<3;k++)mean[k]+=bytes[i+k]/(bytes.length/4);return mean;
       },png.toString('base64'));
      }
      await page.screenshot({path:path.join(report,`${name}-${label}-${background}.png`)});
     }
    }
    measurements.push(row);
    for(const [kind,surface] of Object.entries(row.surfaces)){
     assert.equal(surface.opacity,'1',`${kind}: only the fill becomes translucent, not text/icons`);
     const translucent=choice.theme==='light'&&choice.material!=='solid';
     if(translucent)assert.equal(surface.alpha,0,`${label} ${kind}: use the same transparent-ground mechanism as dark; a thick white tint is not sufficient (${surface.fill})`);
     else if(choice.theme==='light')assert.equal(surface.alpha,1,`${label} ${kind}: keep the original solid selection`);
     else assert.equal(surface.alpha,0,`${label} ${kind}: preserve the dark transparent selection`);
     if(surface.light){
      surface.backdropDelta=Math.max(...surface.light.map((v,k)=>Math.abs(v-surface.dark[k])));
      assert(surface.backdropDelta>40,`${label} ${kind}: actual selected pixels ignore the backdrop (${surface.backdropDelta})`);
     }
    }
   }catch(error){errors.push(`${label}: ${String(error)}`);}
  }
 }finally{await context.close();}
 return {name,measurements,errors};
}
