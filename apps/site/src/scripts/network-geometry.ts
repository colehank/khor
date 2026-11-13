type Point = {x:number;y:number};
export const networkNodeRadius=34;
export const networkViewBox=(compact:boolean,externalUser=false)=>externalUser?(compact?'0 -100 500 690':'0 -100 1000 750'):(compact?'0 35 500 625':'0 0 1000 650');
const mix=(a:Point,b:Point,t:number):Point=>({x:a.x+(b.x-a.x)*t,y:a.y+(b.y-a.y)*t});
const smooth=(t:number)=>{const x=Math.max(0,Math.min(1,t));return x*x*(3-2*x);};
const pair=(p:Point)=>`${p.x.toFixed(3)} ${p.y.toFixed(3)}`;

// A smooth support function surrounds all member circles. Its curvature stays
// positive: broad continuous arcs, without per-node bumps or inward folds.
function envelope(points:Point[],padding:number,softness:number,markAngle:number) {
  const sample=(angle:number,inset=0)=>{
    const n={x:Math.cos(angle),y:Math.sin(angle)},t={x:-n.y,y:n.x};
    const projections=points.map(p=>p.x*n.x+p.y*n.y);
    const tangents=points.map(p=>p.x*t.x+p.y*t.y);
    const max=Math.max(...projections);
    const weights=projections.map(p=>Math.exp((p-max)/softness));
    const total=weights.reduce((a,b)=>a+b,0);
    const average=(values:number[])=>values.reduce((sum,v,i)=>sum+v*weights[i]/total,0);
    const support=max+softness*Math.log(total),slope=average(tangents);
    const curvature=padding-inset+support-average(projections)+(average(tangents.map(v=>v*v))-slope*slope)/softness;
    return {point:{x:(support+padding-inset)*n.x+slope*t.x,y:(support+padding-inset)*n.y+slope*t.y},tangent:{x:curvature*t.x,y:curvature*t.y}};
  };
  const curve=(start:number,sweep:number,count:number,inset=0)=>{
    const step=sweep/count;
    const samples=Array.from({length:count+1},(_,i)=>sample(start+i*step,inset));
    let d=`M${pair(samples[0].point)}`;
    for(let i=0;i<count;i++){
      const a=samples[i],b=samples[i+1];
      d+=` C${pair({x:a.point.x+a.tangent.x*step/3,y:a.point.y+a.tangent.y*step/3})} ${pair({x:b.point.x-b.tangent.x*step/3,y:b.point.y-b.tangent.y*step/3})} ${pair(b.point)}`;
    }
    return d;
  };
  return {
    d:`${curve(-Math.PI/2,Math.PI*2,48)} Z`,
    mark:sample(markAngle).point,
    labelPath:curve(markAngle,Math.PI*.8,24),
    // Each baseline needs its own arc length; a large text dy compresses glyphs.
    subtitlePath:curve(markAngle,Math.PI*.8,24,38),
  };
}

export function networkGeometry(compact:boolean,position:number) {
  const fixed=compact?{
    mac:{x:365,y:315},linux:{x:235,y:185},windows:{x:135,y:270},
  }:{
    mac:{x:780,y:300},linux:{x:430,y:185},windows:{x:200,y:270},
  };
  const cellular=compact?{x:285,y:535}:{x:630,y:470};
  const wifi=compact?{x:145,y:420}:{x:290,y:425};
  const phone=mix(cellular,wifi,position);
  phone.y+=Math.sin(position*Math.PI)*22;
  const joined=smooth(position);
  const padding=compact?49:58,softness=compact?21:30;
  const normal=(a:Point,b:Point)=>{const [from,to]=a.x<b.x?[a,b]:[b,a];return Math.atan2(-(to.x-from.x),to.y-from.y);};
  const areas={
    a:{...envelope([fixed.mac,fixed.linux],padding,softness,normal(fixed.mac,fixed.linux)),opacity:1},
    b:{...envelope([fixed.linux,fixed.windows,mix(fixed.windows,wifi,joined)],padding,softness,normal(fixed.linux,fixed.windows)),opacity:1},
    c:{...envelope([cellular],padding+10,softness,-Math.PI/2),opacity:1-joined},
  };
  // Leave room for both curved label lines between KhorNet and native networks.
  const khorNet=envelope([...Object.values(fixed),phone],116,compact?30:44,compact?-Math.PI*5/9:-Math.PI*.55);
  return {box:networkViewBox(compact),nodes:{...fixed,phone},areas,khorNet};
}
