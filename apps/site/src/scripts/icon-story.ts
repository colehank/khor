export type IconPose = {presence:number; agents:number; mind:number; focus:number; form:number; color:number};
const resting:IconPose = {presence:0,agents:0,mind:0,focus:0,form:0,color:0};
const poses:IconPose[] = [
  {...resting,presence:1},
  {...resting,presence:1,agents:1},
  {...resting,presence:1,mind:1,focus:1},
  {...resting,presence:1,mind:1,focus:1,form:1},
  {...resting,presence:1,mind:1,focus:1,form:1,color:1},
];
const duration = [840,700,900,1600,1100];

/** A chapter has one target pose and one uninterrupted transition into it. */
export function createIconMotion(host:HTMLElement) {
  const paint=animateIcon(host);
  let current={...resting},from={...resting};
  let scene=-1,started=0,settled=false;
  return (requested:number,now:number,reduced:boolean)=>{
    // A direct jump to Meet still finishes the grey form before revealing colour.
    const next=reduced?4:requested===4&&current.form<1?3:requested;
    if(next!==scene){from={...current};scene=next;started=now;settled=false;}
    const time=reduced?1:clamp((now-started)/duration[scene]);
    const t=time===1?1:1-Math.pow(1-time,3);
    for(const key of Object.keys(current) as (keyof IconPose)[])current[key]=from[key]+(poses[scene][key]-from[key])*t;
    if(time===1){current={...poses[scene]};settled=true;}
    paint(current);
    host.dataset.iconScene=['devices','agents','mind','together','meet'][requested];
    host.dataset.iconMotion=settled&&scene===requested?'settled':'running';
    host.dataset.iconColor=String(current.color);
    return {settled:settled&&scene===requested,color:current.color};
  };
}

const namespace = 'http://www.w3.org/2000/svg';
const clamp = (x:number) => Math.max(0,Math.min(1,x));
const ease = (x:number,a:number,b:number) => {const t=clamp((x-a)/(b-a));return t*t*(3-2*t);};
const make = (name:string,attrs:Record<string,string>) => {
  const node=document.createElementNS(namespace,name);
  for(const [key,value]of Object.entries(attrs))node.setAttribute(key,value);
  return node;
};

/** Wrap contiguous original paint layers; preserve every path, gradient and paint order. */
export function animateIcon(host:HTMLElement) {
  const svg=host.querySelector<SVGSVGElement>('.canonical-icon > svg');
  const art=svg?.querySelector<SVGGElement>('g[transform="translate(2.6 .88) scale(.9922)"]');
  if(!svg||!art)throw new Error('The canonical khor icon layer contract has changed');
  const children=[...art.children];
  const diamondStart=children.findIndex(el=>el.tagName==='path');
  const haloStart=children.findIndex(el=>el.getAttribute('fill')==='url(#artwork-ring-halo)');
  const pearlBodies=Array.from({length:5},(_,i)=>children.find(el=>el.getAttribute('fill')===`url(#artwork-pearl-${i})`));
  if(diamondStart<0||haloStart<0||pearlBodies.some(el=>!el))throw new Error('Missing canonical khor icon geometry');
  const wrap=(nodes:Element[])=>{
    const group=make('g',{}) as SVGGElement;
    art.insertBefore(group,nodes[0]);
    nodes.forEach(node=>group.append(node));
    return group;
  };
  const outer=wrap(children.slice(0,diamondStart));
  const diamond=wrap(children.slice(diamondStart,haloStart));
  const innerNodes=children.slice(haloStart,children.indexOf(pearlBodies[0]!)-1);
  // Top and bottom satellites also share the centre x coordinate.
  const satelliteHalos=innerNodes.filter(el=>el.getAttribute('r')==='70.3');
  const inner=wrap(innerNodes);
  const pearlGroups=pearlBodies.map(body=>{
    const index=children.indexOf(body!);
    const group=wrap(children.slice(index-1,index+3));
    return {group,x:Number(body!.getAttribute('cx')),y:Number(body!.getAttribute('cy'))};
  });
  const faces=[...host.querySelectorAll<SVGSVGElement>('[data-session-marks] > svg')];
  const overlays=pearlGroups.slice(1).map((pearl,i)=>{
    const face=faces[i].cloneNode(true) as SVGSVGElement;
    face.setAttribute('x',String(pearl.x-12));face.setAttribute('y',String(pearl.y-12));
    face.setAttribute('width','24');face.setAttribute('height','24');
    face.classList.add('session-face');pearl.group.append(face);return face;
  });
  const defs=svg.querySelector('defs')!;
  const mask=(id:string,r:number,width:number)=>{
    const node=make('mask',{id,maskUnits:'userSpaceOnUse',x:'0',y:'0',width:'1024',height:'1024'});
    const circle=make('circle',{cx:'512',cy:'512',r:String(r),fill:'none',stroke:'white','stroke-width':String(width),pathLength:'1','stroke-dasharray':'1',transform:'rotate(-90 512 512)'});
    node.append(circle);defs.append(node);return circle;
  };
  const outerReveal=mask('story-outer-reveal',368,120);
  const innerReveal=mask('story-inner-reveal',172,260);
  const diamondMask=make('mask',{id:'story-diamond-reveal',maskUnits:'userSpaceOnUse',x:'0',y:'0',width:'1024',height:'1024'});
  const diamondReveal=make('circle',{cx:'512',cy:'512',r:'0',fill:'white'});
  diamondMask.append(diamondReveal);defs.append(diamondMask);
  const ground=[...svg.children].filter(el=>el!==defs&&el!==art);
  const opacity=new Map([...ground,...satelliteHalos].map(node=>[node,Number(node.getAttribute('opacity')??1)]));
  const style=(node:Element,values:Record<string,string|number>)=>{for(const [key,value]of Object.entries(values))(node as SVGElement).style.setProperty(key,String(value));};
  const reveal=(node:Element,maskId:string,amount:number)=>{
    if(amount>=1)node.removeAttribute('mask');else node.setAttribute('mask',`url(#${maskId})`);
    style(node,{opacity:amount});
  };
  return (pose:IconPose)=>{
    // Every layer follows the same form transition; the curves overlap throughout.
    const ring=ease(pose.form,0,.84);
    const glass=ease(pose.form,.02,.92);
    const rim=ease(pose.form,.05,1);
    const back=ease(pose.form,.12,1);
    svg.style.filter=pose.color===1?'none':`grayscale(${1-pose.color})`;
    ground.forEach(node=>style(node,{opacity:back*opacity.get(node)!}));
    const centre=pearlGroups[0];
    style(centre.group,{opacity:pose.mind,transform:pose.mind===1?'none':`translate(512px,512px) scale(${.92+.08*pose.mind}) translate(-512px,-512px)`});
    pearlGroups.slice(1).forEach((pearl,i)=>{
      const scale=(2.25-1.25*pose.form)*(.96+.04*pose.presence);
      const initial=[.68,.56,.45,.61][i];
      const grey=initial+(.68-initial)*pose.form;
      const brightness=(grey+(1-grey)*pose.color)*(1-.78*pose.focus*(1-pose.form)*(1-pose.color));
      style(pearl.group,{opacity:pose.presence,filter:pose.color===1?'none':`brightness(${brightness})`,transform:scale===1?'none':`translate(${pearl.x}px,${pearl.y}px) scale(${scale}) translate(${-pearl.x}px,${-pearl.y}px)`});
      style(overlays[i],{opacity:pose.agents});
    });
    satelliteHalos.forEach(node=>style(node,{opacity:pose.form*opacity.get(node)!}));
    reveal(inner,'story-inner-reveal',ring);innerReveal.setAttribute('stroke-dashoffset',String(1-ring));
    reveal(diamond,'story-diamond-reveal',glass);diamondReveal.setAttribute('r',String(glass*750));
    reveal(outer,'story-outer-reveal',rim);outerReveal.setAttribute('stroke-dashoffset',String(1-rim));
  };
}
