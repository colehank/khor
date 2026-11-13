// One fictional project and device network, shared by all three acts.
export const devices = {
  mac: {name:'MacBook Pro',system:'macOS'},
  linux: {name:'build-01',system:'Linux'},
  phone: {name:'iPhone',system:'iOS'},
} as const;
export const sessions = [
  {id:'terminal',name:'atlas-web',type:'Terminal',mark:'Shell',host:'mac'},
  {id:'codex',name:'Export review',type:'Codex',mark:'Codex',host:'linux'},
  {id:'claude',name:'Project notes',type:'Claude',mark:'Claude',host:'mac'},
] as const;
export type SessionId = typeof sessions[number]['id'];
