import {networkGeometry,networkNodeRadius} from './network-geometry';
type Point={x:number;y:number};
const pair=(p:Point)=>`${p.x.toFixed(2)} ${p.y.toFixed(2)}`;
const port=(p:Point,toward:Point,r:number)=>{const d=Math.hypot(toward.x-p.x,toward.y-p.y);return {x:p.x+(toward.x-p.x)/d*r,y:p.y+(toward.y-p.y)/d*r}};
const curve=(from:Point,c1:Point,c2:Point,to:Point)=>`M ${pair(from)} C ${pair(c1)} ${pair(c2)} ${pair(to)}`;
const onCurve=(a:Point,b:Point,c:Point,d:Point,t:number)=>{
  const s=1-t;return {x:s*s*s*a.x+3*s*s*t*b.x+3*s*t*t*c.x+t*t*t*d.x,y:s*s*s*a.y+3*s*s*t*b.y+3*s*t*t*c.y+t*t*t*d.y};
};

// Three scenes share two balanced device groups. Every route has its own lane.
export function capabilityGeometry(compact:boolean,agent=false,markScale=1) {
  const network=networkGeometry(compact,1),khor=network.khorNet.mark;
  const nodes={...network.nodes,khor};
  const a=nodes.windows,b=nodes.phone,radius=46*markScale,nodeRadius=networkNodeRadius*Math.sqrt(markScale);
  const access={x:a.x,y:(a.y+b.y)/2};
  const accessPath=`M ${pair({x:a.x-radius,y:a.y})} A ${radius} ${radius} 0 0 1 ${pair({x:a.x+radius,y:a.y})} L ${pair({x:b.x+radius,y:b.y})} A ${radius} ${radius} 0 0 1 ${pair({x:b.x-radius,y:b.y})} Z`;
  const origin={x:access.x+radius,y:access.y};
  const routes=(['linux','mac'] as const).map(id=>{
    if(!agent){
      const end={x:nodes[id].x-nodeRadius-5*markScale,y:nodes[id].y};
      const c1={x:origin.x+(end.x-origin.x)*.38,y:origin.y};
      const c2={x:origin.x+(end.x-origin.x)*.62,y:end.y};
      return {id,end,d:curve(origin,c1,c2,end)};
    }
    const end=id==='linux'?{x:nodes[id].x,y:nodes[id].y-nodeRadius-5*markScale}:{x:nodes[id].x-nodeRadius-5*markScale,y:nodes[id].y};
    const lane=(access.x+nodes.mac.x)/2+(compact?10:30);
    const start=port(khor,id==='linux'?{x:khor.x+85,y:khor.y+100}:{x:lane,y:khor.y+130},29*markScale);
    const c1=id==='linux'?{x:khor.x+85,y:khor.y+100}:{x:lane,y:khor.y+155};
    const c2=id==='linux'?{x:end.x,y:end.y-62}:{x:lane,y:end.y};
    return {id,end,d:curve(start,c1,c2,end)};
  });
  // Parallel request / result arcs join the access group's upper edge to Khor.
  const c1=compact?{x:134,y:157}:{x:275,y:137};
  const c2={x:khor.x-82,y:khor.y+46};
  const requestStart=port(a,c1,radius),requestEnd=port(khor,c2,32*markScale);
  const request=curve(requestStart,c1,c2,requestEnd);
  const r1={x:khor.x-12,y:khor.y+117};
  const r2=compact?{x:234,y:186}:{x:440,y:179};
  const resultStart=port(khor,r1,32*markScale),resultEnd=port(a,r2,radius);
  const result=curve(resultStart,r1,r2,resultEnd);
  const taskLabel=onCurve(requestStart,c1,c2,requestEnd,.43);
  const resultLabel=onCurve(resultStart,r1,r2,resultEnd,.56);
  return {nodes,access,accessPath,origin:agent?resultEnd:origin,routes,request,result,taskLabel,resultLabel};
}
