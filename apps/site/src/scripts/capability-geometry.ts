import type {CapabilityTarget} from '../components/concepts/capability-data';
type Point={x:number;y:number};
export type CapabilityNodeId=CapabilityTarget|'mac'|'linux'|'source';
const round=(n:number)=>Number(n.toFixed(2));
const targets=[['codex','linux','Codex'],['claude','linux','Claude'],['cursor','linux','Cursor'],['files','mac','Files'],['web','mac','Web'],['git','mac','Git'],['terminal','mac','Terminal']] as const;
export function capabilityGeometry(compact:boolean,isAgent:boolean){
  const points:Record<CapabilityNodeId,Point>=compact?{
    linux:{x:155,y:220},mac:{x:355,y:455},source:{x:140,y:610},
    codex:{x:90,y:80},claude:{x:265,y:105},cursor:{x:65,y:340},
    files:{x:400,y:260},web:{x:410,y:620},git:{x:275,y:720},terminal:{x:225,y:405},
  }:{
    linux:{x:270,y:220},mac:{x:720,y:260},source:{x:465,y:515},
    codex:{x:145,y:85},claude:{x:360,y:80},cursor:{x:105,y:330},
    files:{x:650,y:70},web:{x:900,y:190},git:{x:885,y:405},terminal:{x:675,y:450},
  };
  const labels:Partial<Record<CapabilityNodeId,{x:number;y:number;anchor:'start'|'middle'|'end'}>>=compact?{
    linux:{x:210,y:227,anchor:'start'},mac:{x:409,y:462,anchor:'start'},
  }:{linux:{x:320,y:226,anchor:'start'},mac:{x:668,y:267,anchor:'end'}};
  const above=new Set(compact?['codex','claude','files','terminal']:['codex','claude','files','web']);
  const nodes=[
    ...targets.map(([id,host,label])=>({id,host,icon:['codex','claude','cursor'].includes(id)?label:id,label,kind:'tool',radius:25})),
    {id:'mac',host:'mac',icon:'macOS',label:'macOS',kind:'device',radius:34},
    {id:'linux',host:'linux',icon:'Linux',label:'Linux',kind:'device',radius:34},
    {id:'source',host:'source',icon:isAgent?'Khor':'iOS',label:isAgent?'Khor Agent':'You',kind:'source',radius:38},
  ].map(node=>{
    const id=node.id as CapabilityNodeId,p=points[id];
    return {...node,id,...p,labelPosition:labels[id]??{x:p.x,y:p.y+(above.has(id)?-42:node.radius+28),anchor:'middle' as const}};
  });
  const byId=Object.fromEntries(nodes.map(node=>[node.id,node]));
  const curve=(from:CapabilityNodeId,to:CapabilityNodeId,bend:number)=>{
    const a=byId[from],b=byId[to],dx=b.x-a.x,dy=b.y-a.y,length=Math.hypot(dx,dy);
    const c={x:(a.x+b.x)/2-dy/length*bend,y:(a.y+b.y)/2+dx/length*bend};
    const port=(p:typeof a)=>{const distance=Math.hypot(c.x-p.x,c.y-p.y),r=p.radius+7;return {x:p.x+(c.x-p.x)/distance*r,y:p.y+(c.y-p.y)/distance*r}};
    const start=port(a),end=port(b);
    return `M ${round(start.x)} ${round(start.y)} Q ${round(c.x)} ${round(c.y)} ${round(end.x)} ${round(end.y)}`;
  };
  const routes=[
    {id:'linux',host:'linux',d:curve('source','linux',compact?-8:-40)},
    {id:'mac',host:'mac',d:curve('source','mac',compact?24:14)},
    ...targets.map(([id,host])=>({id,host,d:curve(host,id,id==='terminal'?-20:12)})),
  ];
  return {box:compact?'0 0 500 810':'0 0 1000 630',nodes,routes};
}
