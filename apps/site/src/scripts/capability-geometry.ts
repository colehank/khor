import {taskSteps,type CapabilityTarget} from '../components/concepts/capability-data';
type Point={x:number;y:number};
export type CapabilityNodeId=CapabilityTarget|'operator'|'you';
const round=(n:number)=>Number(n.toFixed(2));
export function capabilityGeometry(compact:boolean,isAgent:boolean){
  const points:Record<CapabilityNodeId,Point>=compact?{
    review:{x:95,y:225},edit:{x:390,y:185},test:{x:415,y:430},
    preview:{x:330,y:640},changes:{x:90,y:520},operator:{x:245,y:355},you:{x:250,y:60},
  }:{
    review:{x:185,y:190},edit:{x:405,y:90},test:{x:695,y:110},
    preview:{x:855,y:310},changes:{x:720,y:500},operator:{x:435,y:365},you:{x:160,y:520},
  };
  const above=new Set(compact?['review','edit','test']:['review','edit','test','preview']);
  const nodes=[
    ...taskSteps.map(step=>({...step,kind:'task',radius:28,detail:step.host})),
    {id:'operator',icon:isAgent?'Khor':'macOS',label:isAgent?'Khor Agent':'You',detail:isAgent?'':'From your Mac',kind:'operator',radius:38},
    ...(isAgent?[{id:'you',icon:'iOS',label:'You',detail:'Your task',kind:'person',radius:32}]:[]),
  ].map(node=>{
    const id=node.id as CapabilityNodeId,p=points[id];
    const labelPosition=compact&&id==='operator'?{x:p.x-57,y:p.y+4,anchor:'end' as const}:compact&&id==='you'?{x:p.x+57,y:p.y+4,anchor:'start' as const}:{x:p.x,y:p.y+(above.has(id)?-64:node.radius+29),anchor:'middle' as const};
    return {...node,id,...p,labelPosition};
  });
  const byId=Object.fromEntries(nodes.map(node=>[node.id,node]));
  const curve=(from:CapabilityNodeId,to:CapabilityNodeId,bend:number)=>{
    const a=byId[from],b=byId[to],dx=b.x-a.x,dy=b.y-a.y,length=Math.hypot(dx,dy);
    const c={x:(a.x+b.x)/2-dy/length*bend,y:(a.y+b.y)/2+dx/length*bend};
    const port=(p:typeof a)=>{const distance=Math.hypot(c.x-p.x,c.y-p.y),r=p.radius+8;return {x:p.x+(c.x-p.x)/distance*r,y:p.y+(c.y-p.y)/distance*r}};
    const start=port(a),end=port(b);
    return {d:`M ${round(start.x)} ${round(start.y)} Q ${round(c.x)} ${round(c.y)} ${round(end.x)} ${round(end.y)}`,reverseD:`M ${round(end.x)} ${round(end.y)} Q ${round(c.x)} ${round(c.y)} ${round(start.x)} ${round(start.y)}`};
  };
  const routes=[
    ...taskSteps.map(step=>({id:step.id,...curve('operator',step.id,step.id==='changes'?-12:18)})),
    ...(isAgent?[{id:'handoff',...curve('you','operator',compact?35:0)}]:[]),
  ];
  return {box:compact?`0 ${isAgent?0:90} 500 ${isAgent?810:720}`:'0 0 1000 650',nodes,routes};
}
