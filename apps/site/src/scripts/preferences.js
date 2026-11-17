// Inlined in <head>: choose the language before any translated content paints.
(()=>{
  const root=document.documentElement;
  const scheme=matchMedia('(prefers-color-scheme: dark)');
  const keys={theme:'khor.site.theme',locale:'khor.site.locale'};
  const options={theme:['system','light','dark'],locale:['system','en','zh']};
  const read=(kind)=>{try{const value=localStorage.getItem(keys[kind]);return options[kind].includes(value)?value:'system';}catch{return 'system';}};
  const preference={theme:read('theme'),locale:read('locale')};
  const attributes=()=>{
    const index=root.dataset.locale==='zh'?1:0;
    for(const node of document.querySelectorAll('[data-localized-attrs]')){
      const pairs=JSON.parse(node.getAttribute('data-localized-attrs'));
      for(const [name,pair] of Object.entries(pairs)){
        if(name==='textContent')node.textContent=pair[index];else node.setAttribute(name,pair[index]);
      }
    }
  };
  const language=()=>{
    const preferred=(navigator.languages?.length?navigator.languages:[navigator.language]).find(value=>/^(zh|en)(-|$)/i.test(value));
    const locale=preference.locale==='system'?(/^zh/i.test(preferred??'')?'zh':'en'):preference.locale;
    const changed=root.dataset.locale!==locale;
    root.dataset.localePreference=preference.locale;
    root.dataset.locale=locale;root.lang=locale==='zh'?'zh-CN':'en';attributes();
    if(changed)window.dispatchEvent(new CustomEvent('khor:localechange',{detail:{locale}}));
    window.dispatchEvent(new Event('khor:preferencechange'));
  };
  const sources=()=>{for(const source of document.querySelectorAll('[data-theme-source]'))source.media=root.dataset.theme==='dark'?'all':'not all';};
  const theme=()=>{
    const value=preference.theme==='system'?(scheme.matches?'dark':'light'):preference.theme,changed=root.dataset.theme!==value;
    root.dataset.themePreference=preference.theme;root.dataset.theme=value;sources();
    if(changed)window.dispatchEvent(new Event('khor:themechange'));
    window.dispatchEvent(new Event('khor:preferencechange'));
  };
  language();theme();
  document.addEventListener('DOMContentLoaded',()=>{
    attributes();sources();
    if(root.dataset.locale==='zh'&&['#desktop','#cli'].includes(location.hash))document.getElementById(location.hash.slice(1)+'-zh')?.scrollIntoView();
  },{once:true});
  window.addEventListener('languagechange',language);
  scheme.addEventListener('change',theme);
  window.addEventListener('khor:preference',event=>{
    const {kind,value}=event.detail??{};
    if(!options[kind]?.includes(value))return;
    preference[kind]=value;try{localStorage.setItem(keys[kind],value);}catch{}
    if(kind==='theme')theme();else language();
  });
  window.addEventListener('storage',event=>{
    for(const kind of ['theme','locale'])if(event.key===keys[kind]||event.key===null){preference[kind]=read(kind);if(kind==='theme')theme();else language();}
  });
})();
