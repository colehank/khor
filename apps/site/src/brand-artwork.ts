import canonical from '../../gui/src-tauri/icons/src/mandala-glass.svg?raw';

// The canonical geometry lives in the product asset. Daylight needs transmitted
// color, paired refractive edges and a clear pane; dark keeps the source paints.
// All expressions remain live through the story's isolated SVG fragments and
// both morphs, including when appearance changes halfway through a transition.
const light='var(--site-glass-on-light,0)';
const alpha=(dark:string,bright:number)=>`calc(${dark} + (${bright} - ${dark})*${light})`;
const color=(dark:string,day:string)=>`color-mix(in srgb, ${dark} calc((1 - ${light})*100%), ${day})`;
const white='var(--mark-ink-light)',edge='var(--site-glass-edge)',tint='var(--site-glass-tint)';
type Stop=readonly [offset:number,color:string,opacity:number];
const paints:Record<string,readonly Stop[]>={
  'glass-band':[[0,white,.95],[.16,tint,.36],[.3,white,.91],[.5,edge,.14],[.69,tint,.29],[.84,white,.94],[1,edge,.32]],
  'outer-edge':[[0,white,1],[.2,white,.95],[.4,edge,.42],[.55,edge,.2],[.79,tint,.52],[1,white,1]],
  'inner-edge':[[0,edge,.52],[.25,white,.95],[.5,edge,.23],[.76,white,.93],[1,edge,.49]],
  'diamond-face':[[0,white,.72],[.25,white,.25],[.52,tint,.09],[.77,white,.14],[1,tint,.16]],
  'diamond-edge':[[0,white,1],[.26,white,1],[.5,edge,.3],[.78,tint,.48],[1,white,.98]],
  // A broad upper-left daylight source and a restrained transmitted green.
  'background':[[0,'var(--canvas)',0],[1,'var(--canvas)',0]],
  'environment':[[0,white,.9],[.38,white,.5],[.62,white,.12],[.82,white,.01],[1,white,0]],
  'transmitted-wash':[[0,tint,.16],[.35,tint,.12],[.7,tint,.05],[1,tint,0]],
};

const sample=(stops:readonly Stop[],offset:number):Stop=>{
  const exact=stops.find(stop=>stop[0]===offset);if(exact)return exact;
  const end=stops.findIndex(stop=>stop[0]>offset),a=stops[end-1],b=stops[end],t=(offset-a[0])/(b[0]-a[0]);
  return [offset,`color-mix(in srgb, ${a[1]} ${(1-t)*100}%, ${b[1]})`,a[2]+(b[2]-a[2])*t];
};

const withPaints=canonical.replace(/(<(?:linear|radial)Gradient id="artwork-([^"]+)"[^>]*>)([\s\S]*?)(<\/(?:linear|radial)Gradient>)/g,(_,open:string,name:string,stops:string,close:string)=>{
  const daylight=paints[name];if(!daylight)return open+stops+close;
  const original:Stop[]=[...stops.matchAll(/<stop\b[^>]*\/>/g)].map(([stop])=>[
    Number(stop.match(/offset="([^"]+)"/)![1]),stop.match(/stop-color="([^"]+)"/)![1],Number(stop.match(/stop-opacity="([^"]+)"/)?.[1]??1),
  ]);
  if(original[0]?.[0]!==0||original.at(-1)?.[0]!==1)throw new Error(`Khor ${name} gradient contract changed`);
  // Union the two ramps. Extra daylight reflections keep the source's linear
  // interpolation intact in dark mode, with no extra geometry or paint layer.
  const offsets=[...new Set([...original,...daylight].map(stop=>stop[0]))].sort((a,b)=>a-b);
  const themed=offsets.map(offset=>{
    const source=sample(original,offset),target=sample(daylight,offset);
    return `<stop offset="${offset}" stop-color="${color(source[1],target[1])}" stop-opacity="${alpha(String(source[2]),target[2])}"/>`;
  }).join('');
  return open+themed+close;
});

export const brandArtwork=withPaints
  .replace(/(<circle\b[^>]*stroke="#000000"[^>]*opacity="\.32")/,`$1 style="stroke:${color('#000000',edge)};opacity:${alpha('.32',.12)}"`)
  .replace(/(<path\b[^>]*fill="#000000"[^>]*opacity="\.24")/,`$1 style="fill:${color('#000000',edge)};opacity:${alpha('.24',.08)}"`)
  .replace(/(<path\b[^>]*fill="#000000"[^>]*opacity="0\.28")/,`$1 style="fill:${color('#000000',tint)};opacity:${alpha('.28',.08)}"`)
  .replace(/(<circle\b[^>]*fill="url\(#artwork-ring-halo\)"[^>]*opacity="0\.5704")/,`$1 style="opacity:${alpha('.5704',.06)}"`)
  .replace(/(<circle\b[^>]*r="155\.25"[^>]*opacity="0\.4107")/,`$1 style="opacity:${alpha('.4107',.09)}"`)
  .replace(/(<circle\b[^>]*r="70\.3"[^>]*opacity="0\.3614")/g,`$1 style="opacity:${alpha('.3614',.11)}"`)
  .replace(/(<ellipse\b[^>]*fill="url\(#artwork-light-wash\)"[^>]*opacity="0\.3822")/,`$1 style="opacity:${alpha('.3822',.13)}"`);
