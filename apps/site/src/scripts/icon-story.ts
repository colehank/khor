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
  return (p:number)=>{
    const agents=ease(p,.17,.25)*(1-ease(p,.35,.42));
    const dim=ease(p,.37,.44)*(1-ease(p,.70,.84));
    const mind=ease(p,.42,.52);
    const ring=ease(p,.55,.69);
    const glass=ease(p,.62,.80);
    const green=ease(p,.70,.85);
    const rim=ease(p,.81,.96);
    const back=ease(p,.64,.96);
    ground.forEach(node=>style(node,{opacity:back*opacity.get(node)!}));
    const centre=pearlGroups[0];
    style(centre.group,{opacity:mind,transform:mind===1?'none':`translate(512px,512px) scale(${.86+.14*mind}) translate(-512px,-512px)`});
    pearlGroups.slice(1).forEach((pearl,i)=>{
      const scale=3.05-2.05*ease(p,.65,.88);
      const initial=[1.06,.90,.64,.78][i];
      const brightness=(initial+(1-initial)*green)*(1-.83*dim);
      style(pearl.group,{filter:green>=1?'none':`grayscale(${1-green}) brightness(${brightness})`,transform:scale===1?'none':`translate(${pearl.x}px,${pearl.y}px) scale(${scale}) translate(${-pearl.x}px,${-pearl.y}px)`});
      style(overlays[i],{opacity:agents});
    });
    satelliteHalos.forEach(node=>style(node,{opacity:green*opacity.get(node)!}));
    reveal(inner,'story-inner-reveal',ring);innerReveal.setAttribute('stroke-dashoffset',String(1-ring));
    reveal(diamond,'story-diamond-reveal',glass);diamondReveal.setAttribute('r',String(glass*750));
    reveal(outer,'story-outer-reveal',rim);outerReveal.setAttribute('stroke-dashoffset',String(1-rim));
  };
}
