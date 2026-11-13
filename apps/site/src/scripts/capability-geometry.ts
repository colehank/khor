import {networkGeometry,networkNodeRadius} from './network-geometry';
import type {TaskFlow,TaskEndpoint} from '../components/concepts/capability-data';
type Point={x:number;y:number};
const number=(n:number)=>Number(n.toFixed(2));
const pair=(p:Point)=>`${number(p.x)} ${number(p.y)}`;
const port=(p:Point,toward:Point,r:number)=>{const distance=Math.hypot(toward.x-p.x,toward.y-p.y);return {x:p.x+(toward.x-p.x)/distance*r,y:p.y+(toward.y-p.y)/distance*r}};

// Devices and the boundary router keep the exact first-scene coordinates.
export function capabilityGeometry(compact:boolean,flow?:TaskFlow) {
  const network=networkGeometry(compact,1),khor=network.khorNet.mark;
  const nodes={...network.nodes,khor,you:{x:khor.x-(compact?115:145),y:khor.y-108}};
  const route=(from:TaskEndpoint,to:TaskEndpoint):string=>{
    if(from==='you'||to==='you'){
      const control={x:khor.x-80,y:khor.y-105};
      const userPort=port(nodes.you,control,35),routerPort=port(khor,control,27);
      return `M ${pair(from==='you'?userPort:routerPort)} Q ${pair(control)} ${pair(from==='you'?routerPort:userPort)}`;
    }
    if(from==='khor'||to==='khor'){
      const device=from==='khor'?to:from;
      const controls={
        mac:compact?[{x:khor.x,y:140},{x:430,y:105}]:[{x:khor.x,y:115},{x:670,y:90}],
        linux:compact?[{x:khor.x,y:116},{x:215,y:120}]:[{x:khor.x,y:110},{x:390,y:115}],
        windows:compact?[{x:khor.x,y:145},{x:95,y:135}]:[{x:khor.x,y:135},{x:190,y:110}],
        phone:compact?[{x:400,y:140},{x:400,y:390}]:[{x:600,y:130},{x:580,y:440}],
      } as const;
      const [c1,c2]=controls[device as keyof typeof controls];
      const start={x:khor.x,y:khor.y+27},end=port(nodes[device],c2,networkNodeRadius+19);
      return from==='khor'?`M ${pair(start)} C ${pair(c1)} ${pair(c2)} ${pair(end)}`:`M ${pair(end)} C ${pair(c2)} ${pair(c1)} ${pair(start)}`;
    }
    const key=[from,to].sort().join('-');
    if(compact&&key==='linux-phone'){
      // Leave Server horizontally, then turn below its status.
      const start={x:nodes.linux.x+networkNodeRadius+19,y:nodes.linux.y};
      const c1={x:375,y:185},c2={x:295,y:375},end=port(nodes.phone,c2,networkNodeRadius+19);
      return from==='linux'?`M ${pair(start)} C ${pair(c1)} ${pair(c2)} ${pair(end)}`:`M ${pair(end)} C ${pair(c2)} ${pair(c1)} ${pair(start)}`;
    }
    const controls:Record<string,Point>={
      'linux-mac':compact?{x:320,y:215}:{x:680,y:190},
      'linux-windows':compact?{x:125,y:170}:{x:235,y:130},
      'linux-phone':compact?{x:392,y:300}:{x:570,y:330},
    };
    const control=controls[key];
    const start=port(nodes[from],control,networkNodeRadius+19),end=port(nodes[to],control,networkNodeRadius+19);
    return `M ${pair(start)} Q ${pair(control)} ${pair(end)}`;
  };
  return {nodes,path:flow?route(flow.from,flow.to):'',userPath:route('you','khor')};
}
