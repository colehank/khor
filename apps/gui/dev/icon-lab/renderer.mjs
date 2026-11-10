/* khor icon laboratory — entirely procedural SVG. No embedded bitmap artwork.
 * Angle: 0° = right, 90° = down, 225° = upper left.
 * Light transport is an authored 2D approximation, not a ray tracer.
 */
const VERSION = 1;
const controls = [
  { id: 'external', name: '外光强度', min: 0, max: 2, step: .01, value: .94, group: 'external' },
  { id: 'angle', name: '照射方向', min: 0, max: 359, step: 1, value: 225, unit: '°', group: 'external' },
  { id: 'softness', name: '柔光宽度', min: .15, max: 1, step: .01, value: .72, group: 'external' },
  { id: 'transmission', name: '透出绿光强度', min: 0, max: 2, step: .01, value: .65, group: 'external', optional: true },
  { id: 'transmissionSpread', name: '透出绿光范围', min: .2, max: 1.5, step: .01, value: .85, group: 'external', optional: true },
  { id: 'emission', name: '内部总发光', min: 0, max: 2, step: .01, value: .88, group: 'internal' },
  { id: 'center', name: '中心圆珠', min: 0, max: 2, step: .01, value: 1, group: 'internal' },
  { id: 'satellites', name: '四颗小圆珠', min: 0, max: 2, step: .01, value: .8, group: 'internal' },
  { id: 'innerRing', name: '内环', min: 0, max: 2, step: .01, value: 1, group: 'internal' },
  { id: 'bloom', name: '近光柔晕', min: 0, max: 1.5, step: .01, value: .52, group: 'internal' },
  { id: 'hue', name: '光色', min: 50, max: 100, step: 1, value: 71, unit: '°', group: 'internal' },
  { id: 'glass', name: '菱形面感', min: 0, max: 1, step: .01, value: .42, group: 'glass' },
  { id: 'transparency', name: '菱形透光度', min: .2, max: 1, step: .01, value: .82, group: 'glass', optional: true },
  { id: 'edge', name: '玻璃边缘高光', min: 0, max: 1.5, step: .01, value: .86, group: 'glass' },
  { id: 'outerGlass', name: '外环面感', min: 0, max: 1, step: .01, value: .60, group: 'glass' },
  { id: 'frost', name: '磨砂程度', min: 0, max: 1, step: .01, value: .18, group: 'glass' },
  { id: 'reflection', name: '内光映入玻璃', min: 0, max: 2, step: .01, value: .32, group: 'glass' },
  { id: 'diffusion', name: '映光晕开范围', min: .1, max: 1, step: .01, value: .69, group: 'glass' },
  { id: 'background', name: '底色明度', min: 0, max: .12, step: .001, value: .018, group: 'background' },
  { id: 'ambient', name: '外光环境', min: 0, max: 1.5, step: .01, value: .52, group: 'background' },
  { id: 'spill', name: '内光晕染背景', min: 0, max: 2, step: .01, value: .42, group: 'background' },
  { id: 'spread', name: '背景晕染范围', min: .2, max: 1.5, step: .01, value: .82, group: 'background' },
  // Kept in version-1 JSON for lossless import; no longer a light direction.
  { id: 'spillAngle', name: '旧版晕染偏向', min: 0, max: 359, step: 1, value: 45, unit: '°', group: 'background', legacy: true },
  { id: 'outerWidth', name: '外玻璃环宽度', min: 12, max: 42, step: 1, value: 30, unit: 'px', group: 'geometry' },
  { id: 'innerWidth', name: '内环宽度', min: 10, max: 32, step: 1, value: 22, unit: 'px', group: 'geometry' },
  { id: 'pearlSize', name: '中心圆珠半径', min: 46, max: 82, step: 1, value: 69, unit: 'px', group: 'geometry' },
  { id: 'polish', name: '圆珠表面高光', min: 0, max: 1.5, step: .01, value: .92, group: 'geometry' }
];
const defaults = Object.freeze({ ...Object.fromEntries(controls.map(c => [c.id, c.value])), directionalSpill: false });
// Calibrated against the accepted 07 artwork. Existing saved values remain
// untouched; the UI exposes this as an explicit starting preset.
const aligned07 = Object.freeze({ ...defaults,
  external:.98, softness:.86, transmission:.52, transmissionSpread:1.22,
  emission:.92, satellites:.88, bloom:.62, hue:69,
  glass:.52, transparency:.72, edge:1.08, outerGlass:.68, frost:.10,
  reflection:.10, diffusion:.75, background:.027, ambient:.62,
  spill:.20, spread:1.08, polish:1.02
});
function normalize(input) {
  const source = input && typeof input === 'object' ? input : {};
  return { ...Object.fromEntries(controls.map(c => {
    const raw = source[c.id];
    const n = typeof raw === 'number' && Number.isFinite(raw) ? raw : c.value;
    return [c.id, Math.min(c.max, Math.max(c.min, n))];
  })), directionalSpill: source.directionalSpill === true };
}
function parsePreset(payload) {
  if (!payload || payload.format !== 'khor-icon-lab' || payload.version !== VERSION || !payload.parameters || typeof payload.parameters !== 'object') {
    throw new Error('这不是支持的 khor 图标参数文件');
  }
  for (const c of controls) {
    const n = payload.parameters[c.id];
    if(c.optional && n===undefined) continue;
    if (typeof n !== 'number' || !Number.isFinite(n) || n < c.min || n > c.max) throw new Error(`参数「${c.name}」缺失或超出范围`);
  }
  if (payload.parameters.directionalSpill !== undefined && typeof payload.parameters.directionalSpill !== 'boolean') throw new Error('定向晕染开关必须为布尔值');
  return normalize(payload.parameters);
}
function packagePreset(parameters, name) {
  return { format: 'khor-icon-lab', version: VERSION, name: String(name || '我的图标'), reference: '07', parameters: normalize(parameters) };
}
function rgb(h, s, l) {
  s /= 100; l /= 100;
  const a = s * Math.min(l, 1 - l);
  const f = n => { const k = (n + h / 30) % 12; return l - a * Math.max(-1, Math.min(k - 3, 9 - k, 1)); };
  return '#' + [f(0), f(8), f(4)].map(v => Math.round(v * 255).toString(16).padStart(2, '0')).join('');
}
const n = v => Number(v.toFixed(4));
const op = v => n(Math.min(1, Math.max(0, v)));
function lightingState(input) {
  const p=normalize(input);
  const radians=p.angle*Math.PI/180;
  return {
    incident: { angle:p.angle, power:p.external },
    transmitted: { angle:(p.angle+180)%360, power:p.external*p.transmission, x:-Math.cos(radians), y:-Math.sin(radians) },
    internal: { power:p.emission*(p.center*.35+p.satellites*.2+p.innerRing*.45) }
  };
}
function render(input, prefix = 'icon') {
  const p = normalize(input);
  // Prefix is used only for filter/gradient references and cannot inject markup.
  const id = String(prefix).replace(/[^a-zA-Z0-9_-]/g, '') || 'icon';
  const u = name => `url(#${id}-${name})`;
  const a = p.angle * Math.PI / 180;
  const lx = Math.cos(a), ly = Math.sin(a);
  const point = (d, k = 1) => [n(512 + lx * d * k), n(512 + ly * d * k)];
  const [x1, y1] = point(510), [x2, y2] = point(-510);
  const lighting=lightingState(p);
  const T=lighting.transmitted.power, tx=lighting.transmitted.x, ty=lighting.transmitted.y;
  const exitAngle=lighting.transmitted.angle;
  const E = p.external, I = p.emission;
  const transport = I * (p.center*.35 + p.satellites*.2 + p.innerRing*.45);
  const glassLight=transport+T*.72;
  const ringPower = I*p.innerRing;
  const R = p.reflection * transport;
  const green = rgb(p.hue, 82, 65), soft = rgb(p.hue, 65, 74), pale = rgb(p.hue, 91, 81);
  const ambientTint=rgb(p.hue,40,66);
  const diamond = 'M512 144 L880 512 L512 880 L144 512 Z';
  const r = 368, w = p.outerWidth, ir = 172;
  const stops = values => values.map(v => `<stop offset="${v[0]}" stop-color="${v[1]}" stop-opacity="${op(v[2] ?? 1)}"/>`).join('');
  const linear = (name, values, attrs = '') => `<linearGradient id="${id}-${name}" gradientUnits="userSpaceOnUse" x1="${x1}" y1="${y1}" x2="${x2}" y2="${y2}" ${attrs}>${stops(values)}</linearGradient>`;
  const radial = (name, values) => `<radialGradient id="${id}-${name}">${stops(values)}</radialGradient>`;
  const blur = (name, sd) => `<filter id="${id}-${name}" x="-100%" y="-100%" width="300%" height="300%" color-interpolation-filters="sRGB"><feGaussianBlur stdDeviation="${n(sd)}"/></filter>`;
  const glow = (x, y, rx, ry, fill, opacity, rotation = 0) => `<ellipse cx="${n(x)}" cy="${n(y)}" rx="${n(rx)}" ry="${n(ry)}" fill="${fill}" opacity="${op(opacity)}"${rotation ? ` transform="rotate(${n(rotation)} ${n(x)} ${n(y)})"` : ''}/>`;
  const glowCircle = (x, y, rad, opacity) => `<circle cx="${x}" cy="${y}" r="${n(rad)}" fill="${u('light-wash')}" opacity="${op(opacity)}"/>`;
  const pearls = [[512, 512, p.pearlSize, p.center], [512, 144, 18.5, p.satellites], [880, 512, 18.5, p.satellites], [512, 880, 18.5, p.satellites], [144, 512, 18.5, p.satellites]];
  let defs = `<clipPath id="${id}-diamond"><path d="${diamond}"/></clipPath>`;
  defs += `<clipPath id="${id}-annulus"><path clip-rule="evenodd" fill-rule="evenodd" d="M${512+r+w/2} 512 a${r+w/2} ${r+w/2} 0 1 0 -${2*(r+w/2)} 0 a${r+w/2} ${r+w/2} 0 1 0 ${2*(r+w/2)} 0 M${512+r-w/2} 512 a${r-w/2} ${r-w/2} 0 1 0 -${2*(r-w/2)} 0 a${r-w/2} ${r-w/2} 0 1 0 ${2*(r-w/2)} 0"/></clipPath>`;
  defs += linear('glass-band', [[0, '#f8f9ec', p.outerGlass * .88 * E], [.30, '#dce0c7', p.outerGlass * .4 * E], [.53, '#c8d49a', .045 + p.outerGlass * .07], [.72, green, p.outerGlass * .10 * glassLight], [1, green, p.outerGlass * .40 * glassLight]]);
  defs += linear('outer-edge', [[0, '#ffffff', E], [.28, '#f7f9ed', .72 * E], [.52, '#dae1c0', .09], [.82, green, .13 * glassLight], [1, pale, .48 * glassLight]]);
  defs += linear('inner-edge', [[0, '#edf2db', .12 * E], [.4, '#dfe8bd', .10], [.65, '#f0f4d7', .25 * glassLight], [1, pale, .54 * glassLight]]);
  defs += linear('diamond-face', [[0, '#ffffff', p.glass * .28 * E], [.35, '#f7fae8', p.glass * .10 * E], [.60, '#e9efce', .01 * p.glass], [1, soft, p.glass * .08 * glassLight]]);
  defs += linear('diamond-edge', [[0, '#ffffff', E], [.34, '#fafbf2', .85 * E], [.54, '#dce2c9', .12], [.77, soft, .20 * glassLight], [1, pale, .62 * glassLight]]);
  const ringLight = Math.min(81, 26 + E * 14 + I * p.innerRing * 23);
  const [rx1,ry1]=point(ir), [rx2,ry2]=point(-ir);
  const ringGradient = (name, values) => `<linearGradient id="${id}-${name}" gradientUnits="userSpaceOnUse" x1="${rx1}" y1="${ry1}" x2="${rx2}" y2="${ry2}">${stops(values)}</linearGradient>`;
  const ringExposure=ringLight-60.9;
  const ringColor=(h,s,l)=>rgb(h+p.hue-69,s,Math.min(90,Math.max(12,l+ringExposure)));
  defs += ringGradient('ring-solid', [
    [0,ringColor(66.2,90.7,79),1],
    [.1464,ringColor(67.8,83.2,67.3),1],
    [.5,ringColor(69.5,70.2,57.8),1],
    [.8536,ringColor(70.4,70.9,56.9),1],
    [1,ringColor(69,79.5,63.7),1]
  ]);
  defs += ringGradient('ring-shine-outer', [[0, '#fffff1', .98 * E], [.25, '#ffffe2', .14 * E], [.55, '#ffffff', 0], [1, '#ffffff', 0]]);
  defs += ringGradient('ring-shine-inner', [[0, '#ffffed', .07 * E], [.35, '#ffffff', 0], [.70, '#ffffe8', .03 * ringPower], [1, '#ffffed', .98 * ringPower]]);
  defs += radial('light-wash', [[0, green, .68], [.3, green, .34], [.64, green, .09], [1, green, 0]]);
  defs += radial('environment', [[0, ambientTint, .5], [.38, ambientTint, .36], [.62, ambientTint, .06], [.82, ambientTint, .008], [1, ambientTint, 0]]);
  defs += radial('transmitted-wash', [[0,green,.5],[.35,green,.30],[.70,ambientTint,.14],[1,ambientTint,0]]);
  defs += radial('white-wash', [[0, '#fffff4', .73], [.35, '#f9f9e5', .29], [.72, '#f3f6e2', .06], [1, '#ffffff', 0]]);
  defs += radial('pearl-specular', [[0,'#ffffff',1],[.55,'#ffffff',.95],[.84,'#fffff8',.20],[1,'#ffffff',0]]);
  defs += radial('pearl-diffuse', [[0,'#f3ffc6',.30],[.45,'#eff8bd',.12],[1,'#eff8bd',0]]);
  defs += `<linearGradient id="${id}-background" x1="0" y1="0" x2=".7" y2="1">${stops([[0,rgb(64,13,p.background*100+E*2.1),1],[1,rgb(64,13,p.background*100*.80),1]])}</linearGradient>`;
  // Analytic soft ring avoids large raster filter surfaces during live dragging.
  const haloRadius=290, sigma=18+p.bloom*33;
  defs += radial('ring-halo', Array.from({length:33}, (_,i) => {
    const t=i/32;
    return [t,green,Math.exp(-.5*((t*haloRadius-ir)/sigma)**2)*.32];
  }));
  defs += blur('shadow', 9);
  defs += blur('pearl-soft', 2.4);
  defs += blur('specular-soft', 1.1);
  // One opaque surface gradient per pearl; these are never transparent glass.
  pearls.forEach(([x, y, size, strength], i) => {
    const lightness = Math.min(80, 28 + (size<30?2:0) + E * 18 + I * strength * 17);
    defs += `<radialGradient id="${id}-pearl-${i}" cx="${n(.5 + lx * .17)}" cy="${n(.5 + ly * .28)}" r=".64">${stops([[0, rgb(p.hue, 88, Math.min(89, lightness+17)), 1], [.35, rgb(p.hue, 84, Math.min(87,lightness+9)), 1], [.72, rgb(p.hue, 72, lightness), 1], [1, rgb(p.hue, 62, Math.max(14,lightness-9)), 1]])}</radialGradient>`;
    defs += `<clipPath id="${id}-pearl-clip-${i}"><circle cx="${x}" cy="${y}" r="${size}"/></clipPath>`;
  });
  const bgRadius = 430 * p.spread;
  const extX = 512 + lx * 285, extY = 512 + ly * 285;
  let content = `<rect width="1024" height="1024" fill="${u('background')}"/>`;
  // Background light is drawn before the objects, including beyond the outer ring.
  content += glow(extX, extY, 560+p.softness*65, 450+p.softness*60, u('environment'), E*p.ambient*.95);
  content += glow(extX, extY, 420+p.softness*160, 330+p.softness*110, u('white-wash'), E*p.ambient*.15,p.angle-90);
  // White incident light is colored by the mandala and exits on the opposite
  // side. Both the bridge through the object and the pool beyond it follow
  // the SAME external source, independently of the emissive pearls/ring.
  content += `<g data-light="external-transmission">`;
  content += glow(512+tx*205,512+ty*205,300+p.transmissionSpread*70,90+p.softness*90,u('transmitted-wash'),T*.26,exitAngle);
  content += glow(512+tx*320,512+ty*320,220+p.transmissionSpread*170,100+p.transmissionSpread*65+p.softness*100,u('transmitted-wash'),T*.59,exitAngle);
  content += `</g>`;
  content += `<g data-light="internal-background">`;
  // Internal emitters spread around their actual positions, separately from
  // the directional pool caused by transmitted external illumination.
  content += glowCircle(512, 512, bgRadius*1.42, transport * p.spill * .62);
  // Pearl spill at the four junctions follows the same internal master.
  pearls.slice(1).forEach(([x, y, , strength]) => { content += glowCircle(x, y, 96 * p.spread, p.spill * I * strength * .22); });
  content += `</g>`;
  // Small optical registration to 07's circle center and footprint; all
  // foreground shapes share the same transform, so their geometry stays tied.
  content += `<g transform="translate(2.6 .88) scale(.9922)">`;
  // Thin clear annulus; external reflection and internal transmitted tint are separate.
  content += `<circle cx="${n(512-lx*3)}" cy="${n(512-ly*3+5)}" r="368" fill="none" stroke="#000000" stroke-width="${w+3}" opacity=".32" filter="${u('shadow')}"/>`;
  content += `<circle cx="512" cy="512" r="368" fill="none" stroke="${u('glass-band')}" stroke-width="${w}"/>`;
  content += `<g clip-path="${u('annulus')}">`;
  content += glow(512 + lx*300, 512 + ly*300, 310, 110 + p.softness*110, u('white-wash'), E * p.outerGlass * .35, p.angle - 90);
  content += glow(512 + lx*(r+3), 512 + ly*(r+3), 145 + p.softness*90, 40 + p.softness*60, u('white-wash'), E * p.outerGlass * .35, p.angle - 90);
  content += glowCircle(512,512,500,R*.45);
  content += glow(512+tx*352,512+ty*352,210+p.transmissionSpread*60,85+p.softness*60,u('light-wash'),T*.65,p.angle-90);
  content += `</g><circle cx="512" cy="512" r="${r+w/2-.5}" fill="none" stroke="${u('outer-edge')}" stroke-width="5.5" opacity="${op(p.edge*.5)}" filter="${u('pearl-soft')}"/><circle cx="512" cy="512" r="${r+w/2-.5}" fill="none" stroke="${u('outer-edge')}" stroke-width="2.2" opacity="${op(p.edge)}"/><circle cx="512" cy="512" r="${r-w/2+.5}" fill="none" stroke="${u('inner-edge')}" stroke-width="1.1" opacity="${op(p.edge*.8)}"/>`;
  // Diamond: broad reflections, never duplicated ring outlines or ripple arcs.
  content += `<path d="${diamond}" transform="translate(${n(-lx*3)} ${n(-ly*3+7)})" fill="#000000" opacity=".24" filter="${u('shadow')}"/>`;
  content += `<g clip-path="${u('diamond')}"><path d="${diamond}" fill="#000000" opacity="${op(1-p.transparency)}"/><path d="${diamond}" fill="${u('diamond-face')}"/>`;
  content += glow(512 + lx*250, 512 + ly*250, 410, 80 + p.softness*65, u('white-wash'), E * p.glass * .86, p.angle - 90);
  content += glow(512+tx*275,512+ty*275,235,110+p.softness*90,u('light-wash'),T*.75,exitAngle);
  content += glow(512,512,450,450,u('environment'),p.frost*.22*(E*.7+transport*.3));
  content += `<g opacity="${op(R * .64)}">`;
  content += glowCircle(512,512,190+p.diffusion*185,p.center*.7);
  content += glowCircle(512,512,280+p.diffusion*130,p.innerRing*.63);
  pearls.slice(1).forEach(([x,y,,strength]) => { content += glow(x+(512-x)*.13, y+(512-y)*.13, 85+p.diffusion*65, 55+p.diffusion*65, u('light-wash'), strength*.43); });
  content += `</g><path d="${diamond}" fill="none" stroke="${u('diamond-edge')}" stroke-width="${n(6+p.frost*7)}" opacity="${op(p.edge*.22)}" filter="${u('pearl-soft')}"/>`;
  content += `</g><path d="${diamond}" fill="none" stroke="${u('diamond-edge')}" stroke-width="1.7" stroke-linejoin="round" opacity="${op(p.edge)}"/>`;
  // Light bloom is independent of the opaque ring/bead surfaces.
  content += `<circle cx="512" cy="512" r="${haloRadius}" fill="${u('ring-halo')}" opacity="${op(p.bloom*ringPower)}"/>`;
  pearls.forEach(([x,y,size,strength]) => { content += glowCircle(x,y,size*(size>30 ? 2.25 : 3.8), p.bloom * I * strength * .72); });
  // Non-glass inner ring: a smooth opaque material, with controlled emission.
  content += `<circle cx="512" cy="513" r="${ir}" fill="none" stroke="#293415" stroke-width="${p.innerWidth+1}" opacity=".18" filter="${u('pearl-soft')}"/>`;
  content += `<circle cx="512" cy="512" r="${ir}" fill="none" stroke="${u('ring-solid')}" stroke-width="${p.innerWidth}"/>`;
  content += `<circle cx="512" cy="512" r="${ir}" fill="none" stroke="${pale}" stroke-width="${p.innerWidth}" opacity="${op(Math.max(0,ringPower-1.2)*.05)}"/>`;
  content += `<circle cx="512" cy="512" r="${ir+p.innerWidth/2-.8}" fill="none" stroke="${u('ring-shine-outer')}" stroke-width="1.8" opacity="${op(p.polish)}"/><circle cx="512" cy="512" r="${ir-p.innerWidth/2+.5}" fill="none" stroke="${u('ring-shine-inner')}" stroke-width="1.8" opacity="${op(p.polish)}"/>`;
  // Five pigmented pearls. Highlight positions all follow the external source.
  pearls.forEach(([x,y,size,strength], i) => {
    const hx=x+lx*size*(size>30?.58:.45), hy=y+ly*size*(size>30?.84:.65);
    content += `<circle cx="${n(x-lx*2)}" cy="${n(y-ly*2+3)}" r="${size}" fill="#000000" opacity=".23" filter="${u('pearl-soft')}"/>`;
    content += `<circle cx="${x}" cy="${y}" r="${size}" fill="${u('pearl-'+i)}"/>`;
    content += `<g clip-path="${u('pearl-clip-'+i)}">`;
    if(size>30) content += glow(x+lx*size*.15,y+ly*size*.35,size*.85,size*.90,u('pearl-diffuse'),E*.30,p.angle-40);
    content += glow(hx, hy, size*.41, size*.20, u('pearl-specular'), E*p.polish,p.angle-90);
    content += `<g filter="${u('specular-soft')}">${glow(hx, hy, size*.22, size*.07, '#ffffff', E*p.polish*.5, p.angle-90)}</g></g>`;
    content += `<circle cx="${x}" cy="${y}" r="${size-.7}" fill="none" stroke="${pale}" stroke-width="1.3" opacity="${op(I*strength*.16)}"/>`;
  });
  content += `</g>`;
  return `<svg xmlns="http://www.w3.org/2000/svg" width="1024" height="1024" viewBox="0 0 1024 1024" role="img" aria-label="khor 参数化图标"><defs>${defs}</defs>${content}</svg>`;
}
export { VERSION as version, controls, defaults, aligned07, normalize, parsePreset, packagePreset, lightingState, render };
