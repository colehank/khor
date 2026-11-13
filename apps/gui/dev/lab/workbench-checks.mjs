import assert from 'node:assert/strict';
import path from 'node:path';

export async function checkWorkbench(browser,{port,report,engine}){
 const name='workbench-'+engine,context=await browser.newContext({viewport:{width:1600,height:1200}}),page=await context.newPage(),errors=[];
 page.setDefaultTimeout(20000);page.on('pageerror',error=>errors.push(String(error)));
 try{
  const origin=`http://127.0.0.1:${port}`;
  await page.goto(origin+'/workbench/?path=/story/khor-app--gui');
  const canvas=page.frameLocator('#storybook-preview-iframe'),frame=canvas.frameLocator('[data-lab-app-frame]');
  await canvas.locator('[data-lab-app-frame][data-lab-app-ready]').waitFor();
  await frame.locator('[data-composer-text]').fill('保留真实 App 草稿');
  assert.equal(await frame.locator('[data-lab-model-header]').count(),0,'Wide App keeps its production tab navigation');
  const initial=await frame.locator('html').getAttribute('data-lab-environment');
  assert.equal(JSON.parse(initial).material,'frost');
  assert.equal(await frame.locator('html').evaluate(el=>el.style.getPropertyValue('--window-opacity')),'20%');
  assert.equal(await frame.locator('html').evaluate(()=>!!window.__TAURI_INTERNALS__),false,'Browser preview must not impersonate a native host');
  await frame.locator('html').evaluate(()=>{window.__workbenchDocument='same-App';});
  await page.getByRole('tab',{name:/Controls/}).click();
  const opacity=page.getByRole('row').filter({hasText:'Desktop 不透明度'}).locator('input[type=range]');
  await opacity.focus();await opacity.press('End');for(let i=0;i<7;i++)await opacity.press('ArrowLeft');
  await frame.locator('html').evaluate(()=>new Promise(resolve=>setTimeout(resolve,100)));
  assert.equal(await frame.locator('html').evaluate(el=>el.style.getPropertyValue('--window-opacity')),'65%');
  assert.equal(await frame.locator('[data-composer-text]').inputValue(),'保留真实 App 草稿');
  await page.screenshot({path:path.join(report,name+'-desktop-glass.png')});
  await page.getByRole('button',{name:/^Viewport size /}).click();
  await page.getByText('Mobile · 390 × 844',{exact:true}).click();
  await frame.locator('[data-lab-model-header]:visible').waitFor();
  await frame.locator('html').evaluate(()=>new Promise(resolve=>setTimeout(resolve,100)));
  assert.equal(await frame.locator('html').evaluate(()=>innerWidth),390,'Use an actual iframe viewport, not a scaled desktop screenshot');
  assert.equal(await frame.locator('html').evaluate(()=>innerHeight),844,'Storybook panels must not shrink the requested device height');
  assert.equal(await frame.locator('html').evaluate(()=>window.__workbenchDocument),'same-App','Resize does not remount the production App');
  assert.equal(await frame.locator('[data-composer-text]').inputValue(),'保留真实 App 草稿');
  assert.equal(await frame.locator('html').getAttribute('data-window-transparent'),null,'Mobile keeps the actual opaque app ground');
  const keyboard=page.getByRole('row').filter({hasText:'键盘占位高度'}).locator('input[type=range]');
  await keyboard.focus();await keyboard.press('End');
  await frame.locator('html[data-keyboard]').waitFor();
  assert.equal(await frame.locator('html').evaluate(el=>el.style.getPropertyValue('--kb')),'400px');
  assert.equal(await frame.locator('html').evaluate(el=>parseFloat(el.style.getPropertyValue('--app-h'))),444,'Actual viewport code reserves the simulated keyboard space');
  await canvas.locator('[data-lab-keyboard-placeholder]').waitFor();
  await page.screenshot({path:path.join(report,name+'-mobile-keyboard.png')});
  await keyboard.press('Home');await frame.locator('html:not([data-keyboard])').waitFor();
  assert.equal(await frame.locator('[data-composer-text]').inputValue(),'保留真实 App 草稿');
  assert.deepEqual(errors,[]);
 }catch(error){errors.push(String(error));await page.screenshot({path:path.join(report,name+'-failure.png')}).catch(()=>{});}
 finally{await context.close();}
 return {name,errors};
}
