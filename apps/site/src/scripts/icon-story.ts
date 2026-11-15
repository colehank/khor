export const iconStops = [0, .22, .47, .73, 1];
// Copy and geometry start together; only a brief landing follows each move.
export const iconSegments=iconStops.slice(0,-1).map((start,index)=>{
  const end=iconStops[index+1],span=end-start;
  return {start,copyEnd:start+span*.08,motionStart:start,motionEnd:start+span*.94,end};
});
export const clamp = (x:number) => Math.max(0, Math.min(1, x));
export const ease = (x:number, a:number, b:number) => {
  const t=clamp((x-a)/(b-a));
  return t*t*(3-2*t);
};
export type IconState = {phase:number; copyPhase:number; harness:number; focus:number; mind:number; connection:number; light:number};
export type MotionValues = Record<string,string|number>;
export type MotionTrack = {target:HTMLElement|SVGElement; sample:(state:IconState, progress:number)=>MotionValues};

/** Both the native timeline and the fallback resolve the same reversible poses. */
export function iconState(progress:number):IconState {
  const p=clamp(progress);
  const from=Math.min(3,iconStops.findLastIndex(stop=>p>=stop));
  const segment=iconSegments[from];
  const move=clamp((p-segment.motionStart)/(segment.motionEnd-segment.motionStart));
  return {
    phase:from+move,
    copyPhase:from+ease(p,segment.start,segment.copyEnd),
    harness:from===0?move:1,
    focus:from<1?0:from===1?ease(move,0,1):1,
    mind:from<1?0:from===1?ease(move,.25,.88):1,
    connection:from<2?0:from===2?1-Math.pow(1-move,2):1,
    light:from<3?0:move,
  };
}

const namespace='http://www.w3.org/2000/svg';
const make=(name:string, attrs:Record<string,string>)=>{
  const node=document.createElementNS(namespace,name);
  for(const [key,value] of Object.entries(attrs))node.setAttribute(key,value);
  return node;
};
const journeys=[
  {device:{x:350,y:245,scale:3.2},harness:{x:265,y:185,scale:3.1},bend:{x:32,y:-28},start:0,end:.60},
  {device:{x:785,y:380,scale:4.1},harness:{x:820,y:295,scale:3.6},bend:{x:38,y:18},start:.12,end:.75},
  {device:{x:640,y:795,scale:2.6},harness:{x:735,y:800,scale:2.9},bend:{x:-18,y:-32},start:.25,end:.90},
  {device:{x:245,y:635,scale:3.5},harness:{x:195,y:720,scale:3.35},bend:{x:-32,y:22},start:.40,end:1},
];

/** Static canonical artwork inside independently compositable HTML layers. */
export function createIconMotion(host:HTMLElement):MotionTrack[] {
  const canvas=host.querySelector<HTMLElement>('.canonical-icon')!;
  const svg=canvas.querySelector<SVGSVGElement>('svg')!;
  const art=svg?.querySelector<SVGGElement>('g[transform="translate(2.6 .88) scale(.9922)"]');
  if(!art)throw new Error('The canonical khor icon layer contract has changed');
  const children=[...art.children];
  const definitions=new Map([...svg.querySelectorAll('defs [id]')].map(node=>[node.id,node]));
  const tracks:MotionTrack[]=[];
  const track=(target:MotionTrack['target'],sample:MotionTrack['sample'])=>tracks.push({target,sample});
  const board=document.createElement('div');board.className='icon-artboard';
  // Preserve the original art transform, including its optical offset.
  const transform=art.getAttribute('transform')!.match(/translate\(([^ ]+) ([^)]+)\) scale\(([^)]+)\)/)!;
  board.style.transform=`translate(${Number(transform[1])/1024*100}%,${Number(transform[2])/1024*100}%) scale(${transform[3]})`;
  const svgFor=(nodes:Element[],name:string,box=[0,0,1024,1024])=>{
    const image=make('svg',{viewBox:box.join(' '),'aria-hidden':'true'});
    const defs=make('defs',{}),seen=new Set<string>();
    const include=(node:Element)=>{
      for(const el of [node,...node.querySelectorAll('*')])for(const attr of [...el.attributes]){
        const refs=[...attr.value.matchAll(/url\(#([^)]*)\)/g)].map(match=>match[1]);
        if(attr.localName==='href'&&attr.value.startsWith('#'))refs.push(attr.value.slice(1));
        for(const id of refs){
          if(seen.has(id))continue;seen.add(id);
          const source=definitions.get(id);
          if(source){include(source);defs.append(source.cloneNode(true));}
        }
      }
    };
    for(const node of nodes){include(node);image.append(node.cloneNode(true));}
    if(defs.childElementCount)image.prepend(defs);
    // Each fragment owns only its referenced defs and has unique document IDs.
    return image.outerHTML.replaceAll('artwork-',`story-${name}-`);
  };
  const layer=(name:string,nodes:Element[],box=[0,0,1024,1024])=>{
    const el=document.createElement('div');el.className='icon-layer';el.dataset.storyLayer=name;
    const [x,y,w,h]=box;
    Object.assign(el.style,{left:`${x/1024*100}%`,top:`${y/1024*100}%`,width:`${w/1024*100}%`,height:`${h/1024*100}%`});
    el.innerHTML=svgFor(nodes,name,box);board.append(el);return el;
  };

  const ring=children.find(node=>node.getAttribute('stroke')==='url(#artwork-ring-solid)')!;
  const mind=children.find(node=>node.getAttribute('fill')==='url(#artwork-pearl-0)')!;
  const flatRing=ring.cloneNode(false) as Element;
  flatRing.setAttribute('stroke',definitions.get('artwork-ring-solid')!.querySelector('stop[offset="0.5"]')!.getAttribute('stop-color')!);
  const flatMind=mind.cloneNode(false) as Element;
  flatMind.setAttribute('fill',definitions.get('artwork-pearl-0')!.querySelector('stop[offset="0.72"]')!.getAttribute('stop-color')!);
  const centre=layer('mind',[flatRing,flatMind]);
  track(centre,state=>({opacity:state.mind*(1-ease(state.light,.35,.75))}));

  const ringSource=children.find(node=>node.getAttribute('stroke')==='url(#artwork-glass-band)')!;
  const diamondSource=children.find(node=>node.getAttribute('stroke')==='url(#artwork-diamond-edge)')!;
  const width=Number(diamondSource.getAttribute('stroke-width'));
  const radius=Number(ringSource.getAttribute('r'));
  const inset=(Number(ringSource.getAttribute('stroke-width'))-width)/2;
  const edges=[radius-inset,radius+inset].map(r=>{
    const edge=ringSource.cloneNode(false) as Element;
    for(const [key,value] of Object.entries({r:String(r),stroke:'currentColor','stroke-width':String(width),transform:'rotate(-90 512 512)'}))edge.setAttribute(key,value);
    return edge;
  });
  const diamond=diamondSource.cloneNode(false) as Element;diamond.setAttribute('stroke','currentColor');edges.push(diamond);
  for(const edge of edges){edge.setAttribute('pathLength','1');edge.setAttribute('stroke-dasharray','1');}
  const outline=layer('connection',edges);outline.classList.add('connection-line');
  track(outline,state=>({opacity:clamp(state.connection*32)*(1-ease(state.light,.05,.8))}));
  track(outline.querySelector('svg')!,state=>({strokeDashoffset:1-state.connection}));

  const faces=['device','harness'].map(kind=>{
    const source=host.querySelector<HTMLElement>(`[data-${kind}-marks]`)!;
    const marks=[...source.querySelectorAll<SVGSVGElement>('svg')];
    if(marks.length!==4)throw new Error(`Missing khor ${kind} marks`);
    return {source,marks};
  });
  for(let i=0;i<4;i++){
    const body=children.find(node=>node.getAttribute('fill')===`url(#artwork-pearl-${i+1})`)!;
    const x=Number(body.getAttribute('cx')),y=Number(body.getAttribute('cy')),r=Number(body.getAttribute('r'));
    const size=r*2+2,box=[x-size/2,y-size/2,size,size];
    const base=body.cloneNode(false) as Element;base.setAttribute('fill','var(--canvas)');
    const flat=body.cloneNode(false) as Element;flat.setAttribute('fill','var(--secondary)');
    const pearl=layer(`node-${i+1}`,[base],box);
    // An opaque base keeps the construction lines behind the grey nodes.
    const surface=document.createElement('div');surface.className='icon-node-shade';
    surface.innerHTML=svgFor([flat],`node-${i+1}-shade`,box);pearl.append(surface);
    track(surface,state=>({opacity:[.88,.74,.65,.82][i]+(.58-[.88,.74,.65,.82][i])*state.focus}));
    const journey=journeys[i];
    const personal=(state:IconState)=>ease(state.harness,journey.start,journey.end);
    track(pearl,state=>{
      const t=personal(state),arc=Math.sin(Math.PI*t);
      const looseX=journey.device.x+(journey.harness.x-journey.device.x)*t+journey.bend.x*arc;
      const looseY=journey.device.y+(journey.harness.y-journey.device.y)*t+journey.bend.y*arc;
      const looseScale=journey.device.scale+(journey.harness.scale-journey.device.scale)*t;
      const dx=(looseX-x)*(1-state.focus),dy=(looseY-y)*(1-state.focus);
      const scale=looseScale+(1-looseScale)*state.focus;
      const local=(x+y)/2048;
      return {transform:`translate(${dx/size*100}%,${dy/size*100}%) scale(${scale})`,opacity:1-ease(state.light,local*.65,local*.65+.3)};
    });
    faces.forEach(({marks},kind)=>{
      const face=marks[i],wrapper=document.createElement('div');wrapper.className='story-face';
      wrapper.dataset.storyMark=face.parentElement!.dataset.mark!;
      wrapper.style.width=`${24/size*100}%`;face.removeAttribute('width');face.removeAttribute('height');
      wrapper.append(face);pearl.append(wrapper);
      track(wrapper,state=>({opacity:(kind===0?1-personal(state):personal(state))*(1-state.focus)}));
    });
  }
  faces.forEach(({source})=>source.remove());

  // Reveal a static original SVG with opposing HTML translations. The diagonal
  // clip and its feather never change shape; no SVG mask/filter is recomputed.
  const material=document.createElement('div');material.className='icon-material';
  material.dataset.storyLayer='material';
  material.innerHTML=`<div class="reveal-axis"><div class="reveal-clip"><div class="reveal-counter"><div class="reveal-art">${svgFor(children,'material')}</div></div></div></div>`;
  board.append(material);
  const clip=material.querySelector<HTMLElement>('.reveal-clip')!;
  const counter=material.querySelector<HTMLElement>('.reveal-counter')!;
  track(clip,state=>({transform:`translateX(${(state.light-1)*100}%)`}));
  track(counter,state=>({transform:`translateX(${(1-state.light)*100}%)`}));

  const environment=host.closest('[data-opening]')!.querySelector<HTMLElement>('[data-meet-environment]')!;
  const ground=[...svg.children].filter(node=>node.tagName!=='defs'&&node!==art).map(node=>{
    const clone=node.cloneNode(true) as Element;
    if(node.tagName!=='rect')clone.setAttribute('opacity',String(Number(node.getAttribute('opacity')??1)*.42));
    return clone;
  });
  environment.innerHTML=svgFor(ground,'environment');
  environment.querySelector('svg')!.setAttribute('preserveAspectRatio','none');
  canvas.replaceChildren(board);canvas.dataset.iconReady='';
  return tracks;
}
