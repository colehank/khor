export function mountProductTour() {
  const tour=document.querySelector<HTMLElement>('[data-product-tour]');
  if(!tour||matchMedia('(prefers-reduced-motion: reduce)').matches||!('IntersectionObserver' in window))return;
  // One finite demonstration per concept; no scroll loop or live renderer.
  const observer=new IntersectionObserver(entries=>{
    for(const entry of entries){
      if(!entry.isIntersecting)continue;
      entry.target.classList.add('is-in-view');
      observer.unobserve(entry.target);
    }
  },{threshold:.3});
  tour.querySelectorAll('.concept').forEach(scene=>observer.observe(scene));
  window.addEventListener('pagehide',event=>{if(!event.persisted)observer.disconnect();},{once:true});
}
