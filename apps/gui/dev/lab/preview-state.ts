export type PreviewState = {
  platform: 'desktop'|'ios'|'android'; theme: 'light'|'dark';
  material: 'solid'|'clear'|'frost'; opacity: number;
  keyboard: number; safeTop: number; safeBottom: number;
  background: 'wallpaper'|'grid'|'light'|'dark';
};
export const defaultPreview: PreviewState = {platform:'desktop',theme:'dark',material:'frost',opacity:20,keyboard:0,safeTop:0,safeBottom:0,background:'wallpaper'};
export function readPreview(value: unknown): PreviewState {
  if(!value || typeof value!=='object')throw new Error('Invalid Lab environment');
  const row=value as Record<string,unknown>;
  const member=<T extends string>(key:string,values:readonly T[]):T=>{
    const field=row[key];if(typeof field!=='string'||!values.includes(field as T))throw new Error('Invalid Lab '+key);return field as T;
  };
  const number=(key:string,max:number)=>{const n=row[key];if(typeof n!=='number'||!Number.isFinite(n)||n<0||n>max)throw new Error('Invalid Lab '+key);return n;};
  return {platform:member('platform',['desktop','ios','android']),theme:member('theme',['light','dark']),material:member('material',['solid','clear','frost']),
    opacity:number('opacity',100),keyboard:number('keyboard',480),safeTop:number('safeTop',100),safeBottom:number('safeBottom',100),background:member('background',['wallpaper','grid','light','dark'])};
}
