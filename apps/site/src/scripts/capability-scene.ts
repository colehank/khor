import {agentPhases,workPhases} from '../components/concepts/capability-data';
import {capabilityGeometry} from './capability-geometry';

export function mountCapabilityScene(figure:HTMLElement) {
  const isAgent=figure.dataset.concept==='agent',phases=isAgent?agentPhases:workPhases;
  const reduced=matchMedia('(prefers-reduced-motion: reduce)'),compact=matchMedia('(max-width: 760px)');
  const svg=figure.querySelector<SVGSVGElement>('[data-cap-graph]')!;
  const caption=figure.querySelector<HTMLElement>('[data-cap-caption]')!,detail=figure.querySelector<HTMLElement>('[data-cap-detail]')!;
  let index=reduced.matches&&isAgent?phases.length-1:0;
  let active=false,remaining=phases[index].duration,started=0;
  let timer:ReturnType<typeof setTimeout>|undefined;
  const render=()=>{
    const phase=phases[index],selected=new Set<string>(phase.targets);
    for(const node of figure.querySelectorAll<SVGGElement>('[data-cap-node]'))if(selected.has(node.dataset.capNode!))selected.add(node.dataset.capHost!);
    figure.dataset.capStage=phase.id;figure.dataset.capComplete=String(!!phase.complete);
    caption.textContent=phase.caption;detail.textContent=phase.detail;
    for(const element of figure.querySelectorAll<SVGGElement>('[data-cap-node],[data-cap-route]'))element.dataset.active=String(selected.has(element.dataset.capNode??element.dataset.capRoute!));
  };
  const layout=()=>{
    const graph=capabilityGeometry(compact.matches,isAgent);svg.setAttribute('viewBox',graph.box);
    for(const node of graph.nodes){
      const group=figure.querySelector(`[data-cap-node="${node.id}"]`)!;
      group.querySelector('[data-cap-symbol]')!.setAttribute('transform',`translate(${node.x} ${node.y})`);
      const label=group.querySelector('[data-cap-label]')!;
      label.setAttribute('x',String(node.labelPosition.x));label.setAttribute('y',String(node.labelPosition.y));label.setAttribute('text-anchor',node.labelPosition.anchor);
    }
    for(const route of graph.routes)figure.querySelectorAll(`[data-cap-route="${route.id}"] path`).forEach(path=>path.setAttribute('d',route.d));
  };
  const schedule=()=>{
    started=performance.now();
    timer=setTimeout(()=>{index=(index+1)%phases.length;remaining=phases[index].duration;render();if(active)schedule();},remaining);
  };
  const play=()=>{if(active||reduced.matches)return;active=true;schedule();};
  const pause=()=>{if(!active)return;active=false;clearTimeout(timer);remaining=Math.max(0,remaining-(performance.now()-started));};
  const settle=()=>{if(!reduced.matches)return;pause();index=isAgent?phases.length-1:0;remaining=phases[index].duration;render();};
  compact.addEventListener('change',layout);reduced.addEventListener('change',settle);render();layout();
  return {play,pause,destroy(){pause();compact.removeEventListener('change',layout);reduced.removeEventListener('change',settle);}};
}
