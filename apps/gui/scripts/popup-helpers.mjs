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
  await chosen.scrollIntoViewIfNeeded();await chosen.click();await menu.waitFor({state:'detached'});
}
