export type IconPose = {presence:number; harness:number; mind:number; focus:number; connection:number; light:number};
const resting:IconPose = {presence:0,harness:0,mind:0,focus:0,connection:0,light:0};
const poses:IconPose[] = [
  {...resting,presence:1},
  {...resting,presence:1,harness:1},
  {...resting,presence:1,harness:1,mind:1,focus:1},
  {...resting,presence:1,harness:1,mind:1,focus:1,connection:1},
  {...resting,presence:1,harness:1,mind:1,focus:1,connection:1,light:1},
];
const duration = [840,700,900,1200,1500];

/** A chapter has one target pose and one uninterrupted transition into it. */
export function createIconMotion(host:HTMLElement) {
  const paint=animateIcon(host);
  let current={...resting},from={...resting};
  let scene=-1,started=0,settled=false;
  return (requested:number,now:number,reduced:boolean,tile=0)=>{
    // A direct jump still connects the simple circle before the light enters.
    const next=!reduced&&requested===4&&current.connection<1?3:requested;
    if(next!==scene){from={...current};scene=next;started=now;settled=false;}
    const time=reduced?1:clamp((now-started)/duration[scene]);
    const t=time===1?1:1-Math.pow(1-time,3);
    for(const key of Object.keys(current) as (keyof IconPose)[])current[key]=from[key]+(poses[scene][key]-from[key])*t;
    if(time===1){current={...poses[scene]};settled=true;}
    paint(current,tile);
    host.dataset.iconScene=['devices','harnesses','mind','together','meet'][requested];
    host.dataset.iconMotion=settled&&scene===requested?'settled':'running';
    host.dataset.iconColor=String(current.light);
    return {settled:settled&&scene===requested,color:current.light};
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
  // Let the icon's own environment fill the scene before it becomes an app tile.
  // Only the background is projected; the evolving mark remains the same SVG.
  const environment=host.closest('[data-opening]')?.querySelector('[data-meet-environment]');
  if(environment){
    const backdrop=svg.cloneNode(true) as SVGSVGElement;
    backdrop.querySelector('g[transform="translate(2.6 .88) scale(.9922)"]')!.remove();
    backdrop.setAttribute('preserveAspectRatio','none');
    environment.innerHTML=backdrop.outerHTML.replaceAll('artwork-','scene-artwork-');
  }
  const canvas=host.querySelector<HTMLElement>('.canonical-icon')!;
  let ready=false;
  const diamondStart=children.findIndex(el=>el.tagName==='path');
  const haloStart=children.findIndex(el=>el.getAttribute('fill')==='url(#artwork-ring-halo)');
  const pearlBodies=Array.from({length:5},(_,i)=>children.find(el=>el.getAttribute('fill')===`url(#artwork-pearl-${i})`));
  if(diamondStart<0||haloStart<0||pearlBodies.some(el=>!el))throw new Error('Missing canonical khor icon geometry');
  const wrap=(nodes:Element[],name:string)=>{
    const group=make('g',{'data-story-layer':name}) as SVGGElement;
    art.insertBefore(group,nodes[0]);
    nodes.forEach(node=>group.append(node));
    return group;
  };
  const outer=wrap(children.slice(0,diamondStart),'outer');
  const diamond=wrap(children.slice(diamondStart,haloStart),'diamond');
  const innerNodes=children.slice(haloStart,children.indexOf(pearlBodies[0]!)-1);
  // The ring itself is green from One mind. Its surrounding light waits for Meet.
  const innerLight=innerNodes.filter(el=>['290','155.25','70.3'].includes(el.getAttribute('r')??''));
  const inner=wrap(innerNodes,'inner');
  const pearlGroups=pearlBodies.map((body,i)=>{
    const index=children.indexOf(body!);
    const material=children.slice(index-1,index+3);
    const group=wrap(material,i===0?'mind':`node-${i}`);
    const flat=body!.cloneNode(false) as SVGCircleElement;
    const colour=svg.querySelector(`#artwork-pearl-${i} stop[offset="0.72"]`)!.getAttribute('stop-color')!;
    flat.setAttribute('fill',colour);flat.dataset.storySurface='flat';
    group.prepend(flat);
    return {group,flat,material:material.map(node=>({node,opacity:Number(node.getAttribute('opacity')??1)})),x:Number(body!.getAttribute('cx')),y:Number(body!.getAttribute('cy'))};
  });
  const overlays=['device','harness'].map(kind=>{
    const source=host.querySelector<HTMLElement>(`[data-${kind}-marks]`)!;
    const faces=[...source.querySelectorAll<SVGSVGElement>('svg')];
    if(faces.length!==4)throw new Error(`Missing khor ${kind} face marks`);
    const marks=pearlGroups.slice(1).map((pearl,i)=>{
      const face=faces[i];
      face.dataset.storyMark=face.parentElement!.dataset.mark!;
      face.setAttribute('x',String(pearl.x-12));face.setAttribute('y',String(pearl.y-12));
      face.setAttribute('width','24');face.setAttribute('height','24');
      face.classList.add('story-face');pearl.group.append(face);return face;
    });
    // Move, rather than clone, to keep Finder's mask IDs unique in the document.
    source.remove();return marks;
  });
  const defs=svg.querySelector('defs')!;
  // Native SVG primitives work on WebKit SVG groups; CSS filter functions do not.
  const colourFilter=(id:string)=>{
    const filter=make('filter',{id,x:'-50%',y:'-50%',width:'200%',height:'200%','color-interpolation-filters':'sRGB'});
    const saturation=make('feColorMatrix',{type:'saturate',values:'0'});
    const transfer=make('feComponentTransfer',{});
    const channels=['feFuncR','feFuncG','feFuncB'].map(name=>make(name,{type:'linear',slope:'1'}));
    channels.forEach(channel=>transfer.append(channel));filter.append(saturation,transfer);defs.append(filter);
    return {id,saturation,channels};
  };
  const nodeColours=pearlGroups.slice(1).map((_,i)=>colourFilter(`story-node-colour-${i}`));
  const applyColour=(node:Element,filter:ReturnType<typeof colourFilter>,saturation:number,brightness=1)=>{
    (node as SVGElement).style.removeProperty('filter');
    if(saturation===1&&brightness===1){node.removeAttribute('filter');return;}
    filter.saturation.setAttribute('values',String(saturation));
    filter.channels.forEach(channel=>channel.setAttribute('slope',String(brightness)));
    node.setAttribute('filter',`url(#${filter.id})`);
  };
  // Trace the final-width band and diamond edge once. Meet changes their
  // material along the light front; it never draws the geometry again.
  const ringSource=children.find(el=>el.getAttribute('stroke')==='url(#artwork-glass-band)')!;
  const connection=ringSource.cloneNode(false) as SVGCircleElement;
  for(const [key,value]of Object.entries({stroke:'currentColor',pathLength:'1','stroke-dasharray':'1',transform:'rotate(-90 512 512)',class:'connection-line'}))connection.setAttribute(key,value);
  connection.dataset.storyLayer='connection';art.insertBefore(connection,outer);
  const diamondEdge=children[haloStart-1].cloneNode(false) as SVGPathElement;
  for(const [key,value]of Object.entries({stroke:'currentColor',pathLength:'1','stroke-dasharray':'1',class:'connection-line'}))diamondEdge.setAttribute(key,value);
  diamondEdge.dataset.storyLayer='diamond-outline';art.insertBefore(diamondEdge,diamond);
  const lightMask=(id:string,inverse=false)=>{
    const gradient=make('linearGradient',{id:`${id}-gradient`,gradientUnits:'userSpaceOnUse',x1:'0',y1:'0',x2:'256',y2:'256'});
    gradient.append(make('stop',{offset:'0','stop-color':inverse?'black':'white'}),make('stop',{offset:'1','stop-color':inverse?'white':'black'}));
    const mask=make('mask',{id,maskUnits:'userSpaceOnUse',x:'-160',y:'-160',width:'1344',height:'1344'});
    mask.append(make('rect',{x:'-160',y:'-160',width:'1344',height:'1344',fill:`url(#${id}-gradient)`}));
    defs.append(gradient,mask);return gradient;
  };
  const light=lightMask('story-light');
  const unlit=lightMask('story-unlit',true);
  const ground=[...svg.children].filter(el=>el!==defs&&el!==art);
  const litLayers=[outer,diamond,...ground,...innerLight];
  const opacity=new Map(litLayers.map(node=>[node,Number(node.getAttribute('opacity')??1)]));
  const style=(node:Element,values:Record<string,string|number>)=>{for(const [key,value]of Object.entries(values))(node as SVGElement).style.setProperty(key,String(value));};
  return (pose:IconPose,tile=0)=>{
    const front=-256+1440*pose.light;
    for(const gradient of [light,unlit])gradient.setAttribute('gradientTransform',`translate(${front} ${front})`);
    for(const layer of litLayers){
      style(layer,{opacity:pose.light===0?0:opacity.get(layer)!*(ground.includes(layer)?tile:1)});
      if(pose.light===1)layer.removeAttribute('mask');else layer.setAttribute('mask','url(#story-light)');
    }
    for(const line of [connection,diamondEdge]){
      style(line,{opacity:pose.connection===0||pose.light===1?0:1});
      line.setAttribute('stroke-dashoffset',String(1-pose.connection));
      if(pose.light===0)line.removeAttribute('mask');else line.setAttribute('mask','url(#story-unlit)');
    }
    const centre=pearlGroups[0];
    style(centre.group,{opacity:pose.mind});
    style(inner,{opacity:pose.mind});
    pearlGroups.forEach(pearl=>{
      const localLight=pose.light===1?1:ease(front,(pearl.x+pearl.y)/2-256,(pearl.x+pearl.y)/2);
      style(pearl.flat,{opacity:localLight===1?0:1});
      pearl.material.forEach(({node,opacity})=>style(node,{opacity:opacity*localLight}));
    });
    pearlGroups.slice(1).forEach((pearl,i)=>{
      const scale=(2.25-1.25*pose.focus)*(.96+.04*pose.presence);
      const initial=[.78,.69,.64,.74][i];
      const grey=initial+(.48-initial)*pose.focus;
      const localLight=pose.light===1?1:ease(front,(pearl.x+pearl.y)/2-256,(pearl.x+pearl.y)/2);
      applyColour(pearl.group,nodeColours[i],localLight,grey+(1-grey)*localLight);
      style(pearl.group,{opacity:pose.presence,transform:scale===1?'none':`translate(${pearl.x}px,${pearl.y}px) scale(${scale}) translate(${-pearl.x}px,${-pearl.y}px)`});
      style(overlays[0][i],{opacity:pose.presence*(1-pose.harness)*(1-pose.focus)});
      style(overlays[1][i],{opacity:pose.presence*pose.harness*(1-pose.focus)});
    });
    host.dataset.iconTile=String(tile);
    if(!ready){canvas.dataset.iconReady='';ready=true;}
  };
}
