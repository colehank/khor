type Point = {x:number;y:number};
export const networkNodeRadius=40;

const mix=(a:Point,b:Point,t:number):Point=>({x:a.x+(b.x-a.x)*t,y:a.y+(b.y-a.y)*t});
const smooth=(t:number)=>{const x=Math.max(0,Math.min(1,t));return x*x*(3-2*x);};
const pair=(p:Point)=>`${p.x.toFixed(3)} ${p.y.toFixed(3)}`;

// One closed hyperedge surrounds its members, with equal circular clearance
// for a single node, a pair or a trio. No filters or external renderer needed.
function envelope(points:Point[],radius:number):string {
  const sorted=points.filter((p,i)=>points.findIndex(q=>q.x===p.x&&q.y===p.y)===i).sort((a,b)=>a.x-b.x||a.y-b.y);
  if(sorted.length===1){
    const {x,y}=sorted[0];
    return `M${x+radius} ${y} A${radius} ${radius} 0 1 1 ${x-radius} ${y} A${radius} ${radius} 0 1 1 ${x+radius} ${y} Z`;
  }
  const cross=(a:Point,b:Point,c:Point)=>(b.x-a.x)*(c.y-a.y)-(b.y-a.y)*(c.x-a.x);
  const half=(list:Point[])=>{
    const hull:Point[]=[];
    for(const p of list){while(hull.length>1&&cross(hull[hull.length-2],hull[hull.length-1],p)<=0)hull.pop();hull.push(p);}
    return hull.slice(0,-1);
  };
  const hull=[...half(sorted),...half([...sorted].reverse())];
  const normals=hull.map((p,i)=>{
    const next=hull[(i+1)%hull.length],dx=next.x-p.x,dy=next.y-p.y,length=Math.hypot(dx,dy);
    return {x:dy/length*radius,y:-dx/length*radius};
  });
  const offset=(p:Point,n:Point):Point=>({x:p.x+n.x,y:p.y+n.y});
  let path=`M${pair(offset(hull[0],normals[0]))}`;
  for(let i=0;i<hull.length;i++){
    const next=(i+1)%hull.length;
    path+=` L${pair(offset(hull[next],normals[i]))} A${radius} ${radius} 0 0 1 ${pair(offset(hull[next],normals[next]))}`;
  }
  return `${path} Z`;
}

export function networkGeometry(compact:boolean,position:number) {
  const fixed=compact?{
    linux:{x:66,y:265},mac:{x:169,y:265},windows:{x:407,y:265},relay:{x:275,y:65},
  }:{
    linux:{x:120,y:240},mac:{x:310,y:240},windows:{x:820,y:240},relay:{x:530,y:55},
  };
  const phoneY=compact?475:380;
  const stops=[fixed.mac.x,fixed.relay.x,fixed.windows.x];
  const segment=Math.min(1,Math.floor(position));
  const phone={x:stops[segment]+(stops[segment+1]-stops[segment])*(position-segment),y:phoneY};
  const a=1-smooth(position/.7),b=smooth((position-1.3)/.7);
  const padding=compact?58:64;
  const cellular={x:fixed.relay.x,y:phoneY};
  const areas={
    a:{d:envelope([fixed.linux,fixed.mac,mix(fixed.mac,{x:fixed.mac.x,y:phoneY},a)],padding),label:{x:(fixed.linux.x+fixed.mac.x)/2,y:fixed.mac.y-padding-30},opacity:1},
    b:{d:envelope([fixed.windows,mix(fixed.windows,{x:fixed.windows.x,y:phoneY},b)],padding),label:{x:fixed.windows.x,y:fixed.windows.y-padding-30},opacity:1},
    cellular:{d:envelope([cellular],padding),label:{x:cellular.x,y:cellular.y+padding+32},opacity:1-a-b},
  };
  const start=fixed.mac.x+networkNodeRadius,y=fixed.mac.y,top=phoneY-networkNodeRadius;
  const approach=Math.min(1,position),directEnd=phone.x+networkNodeRadius*(1-2*approach);
  const routes={
    direct:`M${start} ${y} C${start+(compact?6:30)} ${y} ${directEnd-50*approach} ${phoneY} ${directEnd} ${phoneY}`,
    'relay-in':`M${start} ${y} C${start+64} ${y} ${fixed.relay.x-96} ${fixed.relay.y} ${fixed.relay.x-25} ${fixed.relay.y}`,
    'relay-out':`M${fixed.relay.x} ${fixed.relay.y+25} C${fixed.relay.x} ${fixed.relay.y+130} ${phone.x} ${top-46} ${phone.x} ${top}`,
  };
  return {box:compact?'0 0 500 600':'0 0 1000 500',nodes:{...fixed,phone},areas,routes};
}
