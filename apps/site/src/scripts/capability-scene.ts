import {capabilityGeometry} from './capability-geometry';
import {mountNetworkMap} from './network-map';
import {mountCapabilityMotion} from './capability-motion';

// Static relationships with browser-owned connection highlights. No task clock,
// moving user, progress simulation or per-frame JavaScript.
export function mountCapabilityScene(figure:HTMLElement) {
  const agent=figure.dataset.concept==='agent',map=mountNetworkMap(figure),compact=matchMedia('(max-width: 760px)');
  const motion=mountCapabilityMotion(figure,agent);
  const find=(selector:string)=>figure.querySelector<SVGElement>(selector)!;
  const translate=(element:Element,p:{x:number;y:number})=>element.setAttribute('transform',`translate(${p.x} ${p.y})`);
  const fitCaptions=()=>{
    if(!agent)return;
    for(const id of ['task','result']){
      const label=find(`[data-relation-${id}-label]`) as SVGTextElement;
      const box=label.getBBox(),{e:x,f:y}=label.transform.baseVal.getItem(0).matrix;
      const cut=find(`[data-relation-label-cut="${id}"]`);
      for(const [key,value] of Object.entries({x:x+box.x-8,y:y+box.y-5,width:box.width+16,height:box.height+10}))cut.setAttribute(key,String(value));
    }
  };
  const layout=()=>{
    const scale=map.markScale,unit=map.textScale,g=capabilityGeometry(compact.matches,agent,scale);
    for(const element of figure.querySelectorAll<SVGElement>('[data-relation-at]'))translate(element,g.nodes[element.dataset.relationAt as keyof typeof g.nodes]);
    for(const {id,d,end} of g.routes){
      for(const path of figure.querySelectorAll(`[data-relation-route="${id}"]`))path.setAttribute('d',d);
      const port=find(`[data-relation-port="${id}"]`);port.setAttribute('cx',String(end.x));port.setAttribute('cy',String(end.y));
    }
    for(const path of figure.querySelectorAll('[data-relation-access]'))path.setAttribute('d',g.accessPath);
    translate(find('[data-relation-access-label]'),g.access);
    find('.relation-you').setAttribute('y',String(-8*unit));
    find('.relation-shared').setAttribute('y',String(17*unit));
    for(const badge of figure.querySelectorAll<SVGGElement>('[data-relation-badge]')){
      const session=badge.dataset.relationBadge==='session';
      translate(badge,{x:28*scale,y:(session?-28:28)*scale});
      badge.querySelector(':scope > circle')?.setAttribute('r',String(16.5*scale));
      badge.querySelector('.relation-status')?.setAttribute('r',String(20*scale));
      const face=badge.querySelector('svg')!;
      for(const [key,value] of Object.entries({x:-11*scale,y:-11*scale,width:22*scale,height:22*scale}))face.setAttribute(key,String(value));
    }
    if(agent){
      for(const path of figure.querySelectorAll('[data-relation-request]'))path.setAttribute('d',g.request);
      for(const path of figure.querySelectorAll('[data-relation-result]'))path.setAttribute('d',g.result);
      translate(find('[data-relation-task-label]'),g.taskLabel);translate(find('[data-relation-result-label]'),g.resultLabel);
      find('.relation-router').setAttribute('r',String(27*scale));
      fitCaptions();
    }
  };
  const stopLayout=map.onLayout(layout);layout();
  return {
    map,
    play(){figure.classList.add('is-running');motion.play();},pause(){figure.classList.remove('is-running');motion.pause();},
    destroy(){figure.classList.remove('is-running');motion.destroy();stopLayout();map.destroy();},
  };
}
