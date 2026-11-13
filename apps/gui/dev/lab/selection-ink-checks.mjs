import assert from 'node:assert/strict';
import path from 'node:path';
const linear=c=>{c/=255;return c<=.04045?c/12.92:((c+.055)/1.055)**2.4;};
const luminance=rgb=>rgb.slice(0,3).map(linear).reduce((sum,c,i)=>sum+c*[.2126,.7152,.0722][i],0);
const contrast=(a,b)=>{const [hi,lo]=[luminance(a),luminance(b)].sort((a,b)=>b-a);return (hi+.05)/(lo+.05);};
function chroma(rgb){
 const [R,G,B]=rgb.map(linear),l=Math.cbrt(.4122214708*R+.5363325363*G+.0514459929*B),m=Math.cbrt(.2119034982*R+.6806995451*G+.1073969566*B),s=Math.cbrt(.0883024619*R+.2817188376*G+.6299787005*B);
 return Math.hypot(1.9779984951*l-2.428592205*m+.4505937099*s,.0259040371*l+.7827717662*m-.808675766*s);
}
async function mean(page,png){
 return page.evaluate(async data=>{
  const img=new Image();img.src='data:image/png;base64,'+data;await img.decode();
  const canvas=document.createElement('canvas');canvas.width=img.width;canvas.height=img.height;const ctx=canvas.getContext('2d');ctx.drawImage(img,0,0);
  const pixels=ctx.getImageData(0,0,img.width,img.height).data,result=[0,0,0];for(let i=0;i<pixels.length;i+=4)for(let k=0;k<3;k++)result[k]+=pixels[i+k]/(pixels.length/4);return result;
 },png.toString('base64'));
}
async function background(page,button){
 const box=await button.boundingBox();assert(box);
 // Hide only foreground ink temporarily to sample the actual composited
 // material, including pseudo-elements. Restore before visual evidence.
 const prior=await button.locator('svg').evaluateAll(nodes=>nodes.map(node=>{const value=node.style.visibility;node.style.visibility='hidden';return value;}));
 try{return await mean(page,await page.screenshot({clip:{x:Math.floor(box.x+box.width/2)-2,y:Math.floor(box.y+box.height/2)-2,width:4,height:4},animations:'disabled'}));}
 finally{await button.locator('svg').evaluateAll((nodes,values)=>nodes.forEach((node,i)=>node.style.visibility=values[i]),prior);}
}
export async function checkSelectionInk(browser,{port,report,engine}){
 const name='selection-ink-'+engine,errors=[],measurements=[];
 for(const theme of ['light','dark'])for(const width of [1280,390])for(const material of ['solid','frost']){
  const label=`${theme}-${width}-${material}`,context=await browser.newContext({viewport:{width,height:900},isMobile:width<500,hasTouch:width<500}),page=await context.newPage();
  page.setDefaultTimeout(20000);page.on('pageerror',error=>errors.push(label+': '+String(error)));
  try{
   const state={platform:width<500?'ios':'desktop',theme,material,opacity:20,keyboard:0,safeTop:width<500?47:0,safeBottom:width<500?34:0,background:theme};
   const url=new URL('/app/',`http://127.0.0.1:${port}`);url.searchParams.set('scene','list');url.searchParams.set('machines','3');url.searchParams.set('environment',JSON.stringify(state));
   await page.goto(url.href);await page.locator(`html[data-theme=${theme}]`).waitFor();
   const base=width<500?'[data-float-nav] [data-nav-item]':'[data-landing-strip] [data-rail-item]',chosen=page.locator(base+'[data-on=true]').first(),rest=page.locator(base+'[data-on=false]').first();
   await chosen.waitFor();await rest.waitFor();await page.waitForLoadState('networkidle');
   const paint=async locator=>locator.evaluate(el=>{
    const cs=getComputedStyle(el),body=el.querySelector('svg [data-body]');if(!body)throw new Error('Production glyph has no solid body');
    const canvas=document.createElement('canvas');canvas.width=canvas.height=1;const ctx=canvas.getContext('2d');
    const rgba=color=>{ctx.clearRect(0,0,1,1);ctx.fillStyle=color;ctx.fillRect(0,0,1,1);return [...ctx.getImageData(0,0,1,1).data];};
    return {ink:rgba(cs.color),opacity:cs.opacity,bodyOpacity:Number(getComputedStyle(body).fillOpacity),canvas:rgba(getComputedStyle(document.documentElement).getPropertyValue('--canvas-solid'))};
   });
   const selected=await paint(chosen),unselected=await paint(rest),selectedGround=await background(page,chosen),restGround=await background(page,rest);
   const selectedContrast=contrast(selected.ink,selectedGround),textContrast=contrast(selected.ink,selected.canvas),restLine=unselected.ink.slice(0,3).map((c,i)=>c*unselected.ink[3]/255+restGround[i]*(1-unselected.ink[3]/255)),a=unselected.ink[3]/255*unselected.bodyOpacity,restBody=unselected.ink.slice(0,3).map((c,i)=>c*a+restGround[i]*(1-a));
   const row={theme,width,material,selected,unselected,selectedGround,selectedContrast,textContrast,chroma:chroma(selected.ink.slice(0,3)),restLineContrast:contrast(restLine,restGround),restBodyContrast:contrast(restBody,restGround)};measurements.push(row);
   assert.equal(selected.opacity,'1');assert.equal(selected.ink[3],255);assert.equal(selected.bodyOpacity,1);
   assert(row.chroma>=.08,'Selected ink must read as a color');assert(chroma(unselected.ink.slice(0,3))<.03,'Resting navigation remains neutral');
   assert(selectedContrast>=3,'Selected icon must remain readable against its actual material');assert(textContrast>=4.5,'Shared selected ink must retain text contrast on the standard theme ground');
   if(material==='solid'){assert(row.restLineContrast>=(theme==='light'?4.7:5.5));assert(row.restBodyContrast>=3);}
   await page.screenshot({path:path.join(report,`${name}-${label}.png`)});
  }catch(error){errors.push(label+': '+String(error));await page.screenshot({path:path.join(report,`${name}-${label}-failure.png`)}).catch(()=>{});}
  finally{await context.close();}
 }
 // The review link explicitly selects light; verify the real Storybook
 // manager passes that choice to its production App iframe.
 const context=await browser.newContext({viewport:{width:1600,height:1200}}),page=await context.newPage();
 page.on('pageerror',error=>errors.push('workbench: '+String(error)));page.setDefaultTimeout(20000);
 try{
  await page.goto(`http://127.0.0.1:${port}/workbench/?path=/story/khor-app--sessions&globals=theme:light`);
  const canvas=page.frameLocator('#storybook-preview-iframe'),frame=canvas.frameLocator('[data-lab-app-frame]');
  await canvas.locator('[data-lab-app-frame][data-lab-app-ready]').waitFor();await frame.locator('html[data-theme=light]').waitFor();
  await frame.locator('[data-landing-strip] [data-on=true]').waitFor();await page.screenshot({path:path.join(report,name+'-workbench-light.png')});
 }catch(error){errors.push('workbench: '+String(error));}finally{await context.close();}
 return {name,measurements,errors};
}
