/** Lab-only, viewer-local tabs. No production/node persistence is changed. */
import type {Draft} from '../src/views/FileView';
export const NEW_CONTENT = '新会话标签页';
export type Place = {source:'workspace'|'browser';scope:{device:string;root:string}|null;query:string;address:string;exit:string|null};
export type Child = {id:string;kind:'new'|'file'|'page';path:string;title:string;place:Place;draft?:Draft;editors:Record<string,{address?:string;editing?:boolean;exitPrompt?:number}>};
export type Root = {id:string;title:string;kind:'session'|'new';agent:'codex'|'claude'|'terminal';tabs:Child[];active:string|null;expanded:boolean};
let serial=0;
export const uid=()=>`lab-${Date.now().toString(36)}-${++serial}`;
export const place=():Place=>({source:'workspace',scope:{device:'mac',root:'/Users/zgh/Projects/khor'},query:'',address:'',exit:'mac'});
export function rootTab(title='khor-codex',agent:Root['agent']='codex'):Root{return {id:uid(),title,kind:'session',agent,tabs:[],active:null,expanded:false};}
export function addChild(root:Root):Root {
 const previous=root.tabs.find(t=>t.id===root.active);
 const tab:Child={id:uid(),kind:'new',title:NEW_CONTENT,path:'',place:{...(previous?.place||place()),query:'',address:''},editors:{}};
 return {...root,tabs:[...root.tabs,tab],active:tab.id,expanded:true};
}
export function changeChild(root:Root,id:string,patch:Partial<Child>):Root{return {...root,tabs:root.tabs.map(t=>t.id===id?{...t,...patch}:t)};}
export function pointChild(root:Root,id:string,path:string,kind:'file'|'page'):Root {
 return changeChild(root,id,{kind,path,title:kind==='file'?path.split('/').pop()||path:path.replace(/^https?:\/\//,'').replace(/\/$/,'')});
}
export function closeChild(root:Root,id:string):Root {
 const at=root.tabs.findIndex(t=>t.id===id),tabs=root.tabs.filter(t=>t.id!==id);
 return {...root,tabs,active:root.active===id?tabs[Math.min(at,tabs.length-1)]?.id||null:root.active,expanded:tabs.length>0&&root.expanded};
}
export function reorder<T extends {id:string}>(rows:T[],ids:string[]):T[]{const rank=new Map(ids.map((id,i)=>[id,i]));return [...rows].sort((a,b)=>(rank.get(a.id)??rows.indexOf(a))-(rank.get(b.id)??rows.indexOf(b)));}
