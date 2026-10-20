/** Isolated Lab preferences. No production khor setting is read or written. */
import {DEFAULT_DECK_TUNING,type DeckTuning as ProductionTuning} from '../src/lib/deck-scrub';
export const tuningFields = [
  {key:'readMs',label:'自动启动等待',unit:'ms',value:1200,min:300,max:3000,step:50,hint:'初次自动与反向纠正后共用'},
  {key:'restMs',label:'停靠触发延迟',unit:'ms',value:200,min:40,max:500,step:20,hint:'静止多久后开始就近停靠'},
  {key:'longPressMs',label:'长按唤出',unit:'ms',value:320,min:180,max:700,step:20,hint:'按住多久进入探索'},
  {key:'turnSlop',label:'反向判定距离',unit:'pt',value:8,min:4,max:24,step:1,hint:'调小更灵敏，调大更不容易误反向'},
  {key:'stillSlop',label:'静止容差',unit:'pt',value:8,min:4,max:24,step:1,hint:'小于这个累计位移，不重新计时'},
  {key:'neutralScale',label:'暂停区大小',unit:'倍',value:1,min:.6,max:2,step:.1,hint:'以初始触点为中心，调大更容易暂停'},
  {key:'followGain',label:'跟手灵敏度',unit:'倍',value:1,min:.5,max:2,step:.1,hint:'同样滑动距离，调大能浏览更多标签'},
  {key:'dwellSlowMs',label:'慢速逐项停留',unit:'ms',value:900,min:300,max:2200,step:20,hint:'自动以慢速运行时，每一项完整停留的时间'},
  {key:'dwellFastMs',label:'最快逐项停留',unit:'ms',value:480,min:200,max:1800,step:20,hint:'加快自动后仍保留的最短逐项停留'},
  {key:'animationScale',label:'动效时长',unit:'倍',value:1,min:.5,max:2,step:.1,hint:'调大更舒缓，影响翻选与停靠动画'},
] as const;
export type TuningKey = typeof tuningFields[number]['key'];
export type DeckTuning = ProductionTuning;
export const DEFAULT_TUNING:DeckTuning = {...DEFAULT_DECK_TUNING};
export const TUNING_STORAGE_KEY='khor.lab.deck-tuning.v1';
export function normalizeTuning(value:unknown):DeckTuning {
  const obj=value&&typeof value==='object'?value as Record<string,unknown>:{};
  const result={...DEFAULT_TUNING};
  for(const f of tuningFields){const n=obj[f.key];if(typeof n==='number'&&Number.isFinite(n))result[f.key]=Math.max(f.min,Math.min(f.max,n));}
  result.dwellFastMs=Math.min(result.dwellFastMs,result.dwellSlowMs);
  return result;
}
function restore(){try{return normalizeTuning(JSON.parse(window.localStorage.getItem(TUNING_STORAGE_KEY)??'null'));}catch{return {...DEFAULT_TUNING};}}
let current=restore();
export const getTuning=():DeckTuning=>({...current});
export function setTuning(patch:Partial<DeckTuning>,persist=true):DeckTuning {
  current=normalizeTuning({...current,...patch});
  try{if(persist)window.localStorage.setItem(TUNING_STORAGE_KEY,JSON.stringify(current));}catch{/* Still usable for this page if storage is unavailable. */}
  return getTuning();
}
