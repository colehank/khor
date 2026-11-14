import {networkGeometry,networkNodeRadius} from './network-geometry';
type Point={x:number;y:number};
const pair=(p:Point)=>`${p.x.toFixed(2)} ${p.y.toFixed(2)}`;
const port=(p:Point,toward:Point,r:number)=>{const d=Math.hypot(toward.x-p.x,toward.y-p.y);return {x:p.x+(toward.x-p.x)/d*r,y:p.y+(toward.y-p.y)/d*r}};
const curve=(from:Point,c1:Point,c2:Point,to:Point)=>`M ${pair(from)} C ${pair(c1)} ${pair(c2)} ${pair(to)}`;

// The same four device positions as the infrastructure scene. Relationships are
// permanent; responsive layout is the only time these coordinates are changed.
export function capabilityGeometry(compact:boolean,agent=false) {
  const network=networkGeometry(compact,1),khor=network.khorNet.mark;
  const nodes={...network.nodes,khor,you:{x:khor.x-(compact?115:145),y:khor.y-108}};
  const a=nodes.windows,b=nodes.phone,radius=64,dx=b.x-a.x,dy=b.y-a.y,d=Math.hypot(dx,dy);
  const n={x:-dy/d*radius,y:dx/d*radius},offset=(p:Point,sign:number)=>({x:p.x+n.x*sign,y:p.y+n.y*sign});
  const access={x:(a.x+b.x)/2,y:(a.y+b.y)/2};
  const accessPath=`M ${pair(offset(a,1))} L ${pair(offset(b,1))} A ${radius} ${radius} 0 0 0 ${pair(offset(b,-1))} L ${pair(offset(a,-1))} A ${radius} ${radius} 0 0 0 ${pair(offset(a,1))} Z`;
  const origin={x:access.x+radius*d/dy,y:access.y};
  const workRoutes=compact?{
    linux:curve(origin,{x:345,y:345},{x:370,y:220},{x:nodes.linux.x+53,y:nodes.linux.y}),
    mac:curve(origin,{x:242,y:365},{x:280,y:368},port(nodes.mac,{x:280,y:368},networkNodeRadius+19)),
  }:{
    linux:curve(origin,{x:570,y:360},{x:580,y:185},{x:nodes.linux.x+53,y:nodes.linux.y}),
    mac:curve(origin,{x:490,y:405},{x:650,y:360},port(nodes.mac,{x:650,y:360},networkNodeRadius+19)),
  };
  const routerStart={x:khor.x,y:khor.y+27};
  const controls=compact?{
    linux:[{x:khor.x,y:116},{x:215,y:120}],mac:[{x:khor.x,y:140},{x:430,y:105}],
  }:{
    linux:[{x:khor.x,y:110},{x:390,y:115}],mac:[{x:khor.x,y:115},{x:670,y:90}],
  };
  const routes=(['linux','mac'] as const).map(id=>{
    const [c1,c2]=controls[id];
    return {id,d:agent?curve(routerStart,c1,c2,port(nodes[id],c2,networkNodeRadius+19)):workRoutes[id]};
  });
  const requestC1={x:nodes.you.x+80,y:nodes.you.y-12},requestC2={x:khor.x+10,y:nodes.you.y+20};
  const resultC1={x:khor.x-90,y:khor.y+8},resultC2={x:nodes.you.x-8,y:nodes.you.y+65};
  const request=curve(port(nodes.you,requestC1,35),requestC1,requestC2,port(khor,requestC2,33));
  const result=curve(port(khor,resultC1,33),resultC1,resultC2,port(nodes.you,resultC2,35));
  const taskLabel={x:(nodes.you.x+khor.x)/2+14,y:nodes.you.y-20};
  const resultLabel={x:(nodes.you.x+khor.x)/2-25,y:khor.y+(compact?36:18)};
  return {nodes,access,accessPath,routes,request,result,taskLabel,resultLabel,terminal:{x:compact?22:0,y:100}};
}
