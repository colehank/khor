type Point = {x:number;y:number};
export const networkNodeRadius=34;
export const networkViewBox=(compact:boolean,externalUser=false)=>externalUser?(compact?'0 -100 500 690':'0 -100 1000 750'):(compact?'0 35 500 625':'0 0 1000 650');
const mix=(a:Point,b:Point,t:number):Point=>({x:a.x+(b.x-a.x)*t,y:a.y+(b.y-a.y)*t});
const smooth=(t:number)=>{const x=Math.max(0,Math.min(1,t));return x*x*(3-2*x);};
const pair=(p:Point)=>`${p.x.toFixed(6)} ${p.y.toFixed(6)}`;

type Sample=(angle:number,inset?:number)=>{point:Point;tangent:Point};

// Boundary, label and icon share the same continuous curve and local tangent.
function outline(sample:Sample,markAngle:number,labelDirection=1,adaptive=false) {
  const curve=(start:number,sweep:number,count:number,inset=0)=>{
    const step=sweep/count;
    const samples=Array.from({length:count+1},(_,i)=>sample(start+i*step,inset));
    let d=`M${pair(samples[0].point)}`;
    const append=(a:ReturnType<Sample>,b:ReturnType<Sample>,from:number,to:number,depth=0)=>{
      const span=to-from;
      const c1={x:a.point.x+a.tangent.x*span/3,y:a.point.y+a.tangent.y*span/3};
      const c2={x:b.point.x-b.tangent.x*span/3,y:b.point.y-b.tangent.y*span/3};
      if(adaptive&&depth<6&&[.25,.5,.75].some(t=>{
        const s=1-t,exact=sample(from+span*t,inset);
        const p={x:s**3*a.point.x+3*s*s*t*c1.x+3*s*t*t*c2.x+t**3*b.point.x,y:s**3*a.point.y+3*s*s*t*c1.y+3*s*t*t*c2.y+t**3*b.point.y};
        const v={x:3*s*s*(c1.x-a.point.x)+6*s*t*(c2.x-c1.x)+3*t*t*(b.point.x-c2.x),y:3*s*s*(c1.y-a.point.y)+6*s*t*(c2.y-c1.y)+3*t*t*(b.point.y-c2.y)};
        return Math.hypot(p.x-exact.point.x,p.y-exact.point.y)>.02||(v.x*exact.tangent.x+v.y*exact.tangent.y)<.95*Math.hypot(v.x,v.y)*Math.hypot(exact.tangent.x,exact.tangent.y);
      })){
        const middle=(from+to)/2,mid=sample(middle,inset);
        append(a,mid,from,middle,depth+1);append(mid,b,middle,to,depth+1);return;
      }
      d+=` C${pair(c1)} ${pair(c2)} ${pair(b.point)}`;
    };
    for(let i=0;i<count;i++)append(samples[i],samples[i+1],start+i*step,start+(i+1)*step);
    return d;
  };
  const mark=sample(markAngle);
  return {
    d:`${curve(-Math.PI/2,Math.PI*2,48)} Z`,
    // An icon's horizontal axis follows the label baseline's local tangent.
    mark:{...mark.point,rotation:Math.atan2(mark.tangent.y*labelDirection,mark.tangent.x*labelDirection)*180/Math.PI},
    labelPath:curve(markAngle,Math.PI*.8*labelDirection,24),
    // Each baseline needs its own arc length; a large text dy compresses glyphs.
    subtitlePath:curve(markAngle,Math.PI*.8*labelDirection,24,38),
  };
}

// A smooth support function encloses the Wi-Fi members as Mobile roams.
function envelope(fixed:Point[],mobile:Point,presence:number,padding:number,softness:number,markAngle:number) {
  const sample=(angle:number,inset:number)=>{
    const n={x:Math.cos(angle),y:Math.sin(angle)},t={x:-n.y,y:n.x};
    const projections=fixed.map(p=>p.x*n.x+p.y*n.y);
    const tangents=fixed.map(p=>p.x*t.x+p.y*t.y);
    const max=Math.max(...projections);
    const weights=projections.map(p=>Math.exp((p-max)/softness));
    const total=weights.reduce((a,b)=>a+b,0);
    const average=(values:number[])=>values.reduce((sum,v,i)=>sum+v*weights[i]/total,0);
    let support=max+softness*Math.log(total),slope=average(tangents);
    let bend=-average(projections)+(average(tangents.map(v=>v*v))-slope*slope)/softness;
    const mobileProjection=mobile.x*n.x+mobile.y*n.y;
    const mobileSupport=mobileProjection-softness*(1-presence);
    const mobileSlope=mobile.x*t.x+mobile.y*t.y;
    const difference=(support-mobileSupport)/softness;
    if(difference<=-1){support=mobileSupport;slope=mobileSlope;bend=-mobileProjection;}
    else if(difference<1){
      // A C2 smooth maximum with finite influence. A member below the upper
      // arc cannot perturb that arc; no fixed/live curve splice is needed.
      const q=difference,q2=q*q;
      const blend=(3+6*q2-q2*q2)/8,first=q*(3-q2)/2,second=1.5*(1-q2);
      bend=(bend-mobileProjection)/2+first*(bend+mobileProjection)/2+second*(slope-mobileSlope)**2/(2*softness);
      slope=(slope+mobileSlope)/2+first*(slope-mobileSlope)/2;
      support=(support+mobileSupport)/2+softness*blend/2;
    }
    const curvature=padding-inset+support+bend;
    return {point:{x:(support+padding-inset)*n.x+slope*t.x,y:(support+padding-inset)*n.y+slope*t.y},tangent:{x:curvature*t.x,y:curvature*t.y}};
  };
  return outline((angle,inset=0)=>sample(angle,inset),markAngle,1,true);
}

function ellipse(center:Point,rx:number,ry:number,tilt:number,markAngle:number,labelDirection=1) {
  const rotate=(p:Point)=>({x:p.x*Math.cos(tilt)-p.y*Math.sin(tilt),y:p.x*Math.sin(tilt)+p.y*Math.cos(tilt)});
  return outline((angle,inset=0)=>{
    const p=rotate({x:(rx-inset)*Math.cos(angle),y:(ry-inset)*Math.sin(angle)});
    return {point:{x:center.x+p.x,y:center.y+p.y},tangent:rotate({x:-(rx-inset)*Math.sin(angle),y:(ry-inset)*Math.cos(angle)})};
  },markAngle,labelDirection);
}

export function networkGeometry(compact:boolean,position:number,wiredAngle=compact?Math.PI*.7:-Math.PI*.3) {
  const fixed=compact?{
    mac:{x:355,y:435},linux:{x:355,y:255},windows:{x:130,y:265},
  }:{
    mac:{x:720,y:445},linux:{x:720,y:235},windows:{x:240,y:270},
  };
  const cellular=compact?{x:215,y:525}:{x:460,y:500};
  const wifi=compact?{x:130,y:435}:{x:240,y:450};
  const phone=mix(cellular,wifi,position);
  phone.y+=Math.sin(position*Math.PI)*12;
  const joined=smooth(position);
  const padding=76,softness=compact?22:44;
  const areas={
    // Wired owns the right-hand enclosure. Use its free lower arc on phones,
    // reversing the baseline so text stays upright, away from the Wi-Fi edge.
    a:{...ellipse(compact?{x:355,y:345}:{x:720,y:340},compact?100:140,compact?169:185,-.04,wiredAngle,compact?-1:1),opacity:1},
    b:{...envelope([fixed.linux,fixed.windows],mix(fixed.windows,wifi,joined),joined,padding,softness,-Math.PI*.62),opacity:1},
    c:{...ellipse(cellular,compact?88:144,compact?58:72,-.04,-Math.PI*.68),opacity:1-joined},
  };
  // KhorNet stays stable while the native network boundaries and Mobile change.
  const khorNet=ellipse(compact?{x:250,y:340}:{x:500,y:330},compact?225:420,compact?270:285,-.05,-Math.PI/2+.05);
  return {box:networkViewBox(compact),nodes:{...fixed,phone},areas,khorNet};
}
