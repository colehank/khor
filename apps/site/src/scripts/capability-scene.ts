import {capabilityGeometry} from './capability-geometry';
import {mountNetworkMap} from './network-map';

// Static relationships with CSS opacity pulses. No task clock, transport loop,
// moving user, progress simulation or per-frame JavaScript.
export function mountCapabilityScene(figure:HTMLElement) {
  const agent=figure.dataset.concept==='agent',map=mountNetworkMap(figure),compact=matchMedia('(max-width: 760px)');
  const find=(selector:string)=>figure.querySelector<SVGElement>(selector)!;
  const translate=(element:Element,p:{x:number;y:number})=>element.setAttribute('transform',`translate(${p.x} ${p.y})`);
  const layout=()=>{
    const g=capabilityGeometry(compact.matches,agent);
    for(const element of figure.querySelectorAll<SVGElement>('[data-relation-at]'))translate(element,g.nodes[element.dataset.relationAt as keyof typeof g.nodes]);
    for(const {id,d,end} of g.routes){
      for(const path of figure.querySelectorAll(`[data-relation-route="${id}"]`))path.setAttribute('d',d);
      const port=find(`[data-relation-port="${id}"]`);port.setAttribute('cx',String(end.x));port.setAttribute('cy',String(end.y));
    }
    if(agent){
      for(const path of figure.querySelectorAll('[data-relation-request]'))path.setAttribute('d',g.request);
      for(const path of figure.querySelectorAll('[data-relation-result]'))path.setAttribute('d',g.result);
      translate(find('[data-relation-task-label]'),g.taskLabel);translate(find('[data-relation-result-label]'),g.resultLabel);
      const cut=find('[data-relation-result-cut]');cut.setAttribute('x',String(g.resultLabel.x-50));cut.setAttribute('y',String(g.resultLabel.y-25));
    }else{
      find('[data-relation-access]').setAttribute('d',g.accessPath);translate(find('[data-relation-access-label]'),g.access);
      const origin=find('[data-relation-origin]');origin.setAttribute('cx',String(g.origin.x));origin.setAttribute('cy',String(g.origin.y));
    }
  };
  compact.addEventListener('change',layout);layout();
  return {
    play(){figure.classList.add('is-running');},pause(){figure.classList.remove('is-running');},
    destroy(){figure.classList.remove('is-running');map.destroy();compact.removeEventListener('change',layout);},
  };
}
