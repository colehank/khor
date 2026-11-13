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

export function networkGeometry(compact:boolean,position:number,relayAmount=0) {
  const fixed=compact?{
    linux:{x:80,y:220},mac:{x:205,y:170},windows:{x:430,y:330},relay:{x:330,y:65},
  }:{
    linux:{x:180,y:270},mac:{x:350,y:190},windows:{x:800,y:350},relay:{x:610,y:110},
  };
  const stops=compact?[{x:155,y:365},{x:260,y:505},{x:405,y:520}]:[{x:260,y:415},{x:575,y:445},{x:830,y:495}];
  const segment=Math.min(1,Math.floor(position)),progress=position-segment;
  const phone=mix(stops[segment],stops[segment+1],progress);
  // An unhurried arc between networks, rather than a horizontal track.
  phone.y+=Math.sin(progress*Math.PI)*(segment===0?32:-40);
  const a=1-smooth(position/.7),b=smooth((position-1.3)/.7);
  const padding=compact?58:70;
  const areas={
    a:{d:envelope([fixed.linux,fixed.mac,mix(fixed.mac,stops[0],a)],padding),label:compact?{x:22,y:110,anchor:'start'}:{x:145,y:130,anchor:'start'},opacity:1,strength:a},
    b:{d:envelope([fixed.windows,mix(fixed.windows,stops[2],b)],padding),label:compact?{x:488,y:245,anchor:'end'}:{x:820,y:250,anchor:'middle'},opacity:1,strength:0},
    cellular:{d:envelope([stops[1]],padding),label:{x:stops[1].x,y:stops[1].y+padding+32,anchor:'middle'},opacity:1-a-b,strength:0},
  };
  // Connections are closed sets too. The relay joins the active set and then
  // retreats into the direct set when it is no longer needed.
  const connectionPadding=compact?54:62;
  const relayVertex=mix(mix(fixed.mac,phone,.5),fixed.relay,relayAmount);
  const connections={
    active:{d:envelope([fixed.mac,phone,relayVertex],connectionPadding),opacity:1-a},
    candidate:{d:envelope([fixed.mac,phone],connectionPadding-10),opacity:relayAmount*.45},
  };
  return {box:compact?'0 0 500 650':'0 0 1000 600',nodes:{...fixed,phone},areas,connections};
}
