import {networkGeometry,networkNodeRadius} from './network-geometry';
import type {TaskPhase} from '../components/concepts/capability-data';
type Point={x:number;y:number};
const number=(n:number)=>Number(n.toFixed(2));
// Exact node and hyperedge coordinates from the first scene, not a second graph.
export function capabilityGeometry(compact:boolean,route?:TaskPhase['route']) {
  const network=networkGeometry(compact,1),nodes=network.nodes;
  // Keep operation paths inside the network, clear of the fixed edge labels
  // and the status beneath Server, in both reference layouts.
  const controls={transfer:compact?{x:320,y:215}:{x:680,y:190},watch:compact?{x:400,y:300}:{x:570,y:330},request:compact?{x:300,y:440}:{x:570,y:495}};
  const paths={
    upload:[nodes.mac,nodes.linux,controls.transfer],download:[nodes.linux,nodes.mac,controls.transfer],
    watch:[nodes.linux,nodes.phone,controls.watch],preview:[nodes.linux,nodes.phone,controls.watch],
    request:[nodes.phone,nodes.mac,controls.request],deliver:[nodes.mac,nodes.phone,controls.request],
  } as const;
  let path='';
  if(route){
    const [a,b,c]=paths[route];
    const port=(p:Point)=>{const distance=Math.hypot(c.x-p.x,c.y-p.y),r=networkNodeRadius+19;return {x:p.x+(c.x-p.x)/distance*r,y:p.y+(c.y-p.y)/distance*r}};
    const start=port(a),end=port(b);
    path=`M ${number(start.x)} ${number(start.y)} Q ${number(c.x)} ${number(c.y)} ${number(end.x)} ${number(end.y)}`;
  }
  return {nodes,path};
}
