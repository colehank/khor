export function mountPreferenceControls(){
  const controller=new AbortController(),on={signal:controller.signal};
  const toggle=document.querySelector<HTMLButtonElement>('[data-theme-toggle]');
  const language=document.querySelector<HTMLSelectElement>('[data-language-select]');
  const sync=()=>{
    toggle?.setAttribute('aria-pressed',String(document.documentElement.dataset.theme==='dark'));
    if(language)language.value=document.documentElement.dataset.localePreference??'system';
  };
  toggle?.addEventListener('click',()=>window.dispatchEvent(new CustomEvent('khor:preference',{detail:{kind:'theme',value:document.documentElement.dataset.theme==='dark'?'light':'dark'}})),on);
  language?.addEventListener('change',()=>window.dispatchEvent(new CustomEvent('khor:preference',{detail:{kind:'locale',value:language.value}})),on);
  window.addEventListener('khor:preferencechange',sync,on);sync();
  window.addEventListener('pagehide',event=>{if(!event.persisted)controller.abort();},on);
}
