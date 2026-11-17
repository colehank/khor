export type Locale='en'|'zh';
export const currentLocale=():Locale=>typeof document!=='undefined'&&document.documentElement.dataset.locale==='zh'?'zh':'en';
export const translate=(en:string,zh:string)=>currentLocale()==='zh'?zh:en;
/** Static English fallback plus paired attributes for the browser preference bootstrap. */
export function localizedAttrs(values:Record<string,readonly [string,string]>,deferMedia=false) {
  return {...Object.fromEntries(Object.entries(values).filter(([name])=>name!=='textContent'&&(!deferMedia||!['src','srcset'].includes(name))).map(([name,pair])=>[name,pair[0]])),
    'data-localized-attrs':JSON.stringify(values)};
}
