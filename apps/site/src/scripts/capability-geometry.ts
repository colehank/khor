import {networkGeometry,networkNodeRadius} from './network-geometry';
type Point={x:number;y:number};
const pair=(p:Point)=>`${p.x.toFixed(2)} ${p.y.toFixed(2)}`;
const port=(p:Point,toward:Point,r:number)=>{const d=Math.hypot(toward.x-p.x,toward.y-p.y);return {x:p.x+(toward.x-p.x)/d*r,y:p.y+(toward.y-p.y)/d*r}};
const curve=(from:Point,c1:Point,c2:Point,to:Point)=>`M ${pair(from)} C ${pair(c1)} ${pair(c2)} ${pair(to)}`;

// The same four device positions as the infrastructure scene. Relationships are
// permanent; responsive layout is the only time these coordinates are changed.
export function capabilityGeometry(compact:boolean,agent=false) {
  const network=networkGeometry(compact,1),khor=network.khorNet.mark;
  const nodes={...network.nodes,khor};
  const a=nodes.windows,b=nodes.phone,radius=64,dx=b.x-a.x,dy=b.y-a.y,d=Math.hypot(dx,dy);
  const n={x:-dy/d*radius,y:dx/d*radius},offset=(p:Point,sign:number)=>({x:p.x+n.x*sign,y:p.y+n.y*sign});
  const access={x:(a.x+b.x)/2,y:(a.y+b.y)/2};
  const accessPath=`M ${pair(offset(a,1))} L ${pair(offset(b,1))} A ${radius} ${radius} 0 0 0 ${pair(offset(b,-1))} L ${pair(offset(a,-1))} A ${radius} ${radius} 0 0 0 ${pair(offset(a,1))} Z`;
  const origin={x:access.x+radius*d/dy,y:access.y};
  const workEnds={
    linux:{x:nodes.linux.x+networkNodeRadius+5,y:nodes.linux.y},
    mac:port(nodes.mac,compact?{x:280,y:368}:{x:650,y:360},networkNodeRadius+5),
  };
  const workRoutes=compact?{
    linux:curve(origin,{x:345,y:345},{x:370,y:220},workEnds.linux),
    mac:curve(origin,{x:242,y:365},{x:280,y:368},workEnds.mac),
  }:{
    linux:curve(origin,{x:570,y:360},{x:580,y:185},workEnds.linux),
    mac:curve(origin,{x:490,y:405},{x:650,y:360},workEnds.mac),
  };
  const routerStart={x:khor.x,y:khor.y+27};
  const controls=compact?{
    linux:[{x:khor.x,y:116},{x:215,y:120}],mac:[{x:khor.x,y:140},{x:430,y:105}],
  }:{
    linux:[{x:khor.x,y:110},{x:390,y:115}],mac:[{x:khor.x,y:115},{x:670,y:90}],
  };
  const routes=(['linux','mac'] as const).map(id=>{
    const [c1,c2]=controls[id];
    // Meet the device above its face, clear of the two corner badges.
    const end=agent?{x:nodes[id].x,y:nodes[id].y-networkNodeRadius-5}:workEnds[id];
    return {id,end,d:agent?curve(routerStart,c1,c2,end):workRoutes[id]};
  });
  // The same access group delegates through its upper edge; no separate user.
  const requestC1=compact?{x:105,y:163}:{x:200,y:140};
  const requestC2=compact?{x:135,y:112}:{x:khor.x-95,y:khor.y+30};
  const resultC1=compact?{x:195,y:118}:{x:khor.x-35,y:140};
  const resultC2=compact?{x:175,y:159}:{x:270,y:180};
  const requestStart=port(a,requestC1,radius),resultEnd=port(a,resultC2,radius);
  const request=curve(requestStart,requestC1,requestC2,port(khor,requestC2,33));
  const result=curve(port(khor,resultC1,33),resultC1,resultC2,resultEnd);
  const taskLabel=compact?{x:134,y:139}:{x:230,y:133};
  const resultLabel=compact?{x:157,y:186}:{x:292,y:172};
  return {nodes,access,accessPath,origin:agent?resultEnd:origin,routes,request,result,taskLabel,resultLabel};
}
