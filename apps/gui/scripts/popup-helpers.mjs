export async function popupChoices(trigger) {
  await trigger.click();const menu=trigger.page().locator('[data-popup-menu]:visible').last();await menu.waitFor();
  const choices=await menu.locator('[data-popup-option]').evaluateAll(items=>items.map(item=>({value:item.getAttribute('data-popup-option'),label:item.textContent,disabled:item.hasAttribute('data-disabled'),selected:item.getAttribute('data-on')==='true'})));
  await trigger.page().keyboard.press('Escape');await menu.waitFor({state:'detached'});return choices;
}
export async function pickPopup(trigger,value) {
  // Native fields in external fixtures remain native; app-owned menus use real clicks.
  if(await trigger.evaluate(element=>element.tagName==='SELECT'))return trigger.selectOption(value);
  await trigger.click();const menu=trigger.page().locator('[data-popup-menu]:visible').last();await menu.waitFor();
  const options=menu.locator('[data-popup-option]');let chosen;
  for(let index=0;index<await options.count();index++){const option=options.nth(index);if(await option.getAttribute('data-popup-option')===value){chosen=option;break;}}
  if(!chosen)throw Error('Missing Khor choice '+value);
  // Wait for the finite opening transform before the real pointer action.
  // click already scrolls; a separate scroll action can race the next menu's
  // opening frame in WebKit. Do not wait for infinite content animations.
  await menu.evaluate(async node=>{await Promise.all(node.getAnimations().filter(animation=>animation.effect?.getComputedTiming().iterations!==Infinity).map(animation=>animation.finished.catch(()=>{})));});
  try{await chosen.click();}catch(error){
    console.error('Khor popup action failed',JSON.stringify(await menu.evaluate(node=>({rect:node.getBoundingClientRect().toJSON(),active:document.activeElement?.outerHTML,options:[...node.querySelectorAll('[data-popup-option]')].map(item=>({value:item.dataset.popupOption,rect:item.getBoundingClientRect().toJSON()})),animations:node.getAnimations().map(animation=>({state:animation.playState,timing:animation.effect?.getComputedTiming()}))}))));throw error;
  }
  await menu.waitFor({state:'detached'});
}
