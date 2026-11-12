export function mountProductTour() {
  const tour=document.querySelector<HTMLElement>('[data-product-tour]');
  if(!tour||matchMedia('(prefers-reduced-motion: reduce)').matches||!('IntersectionObserver' in window))return;
  tour.setAttribute('data-tour-motion','');
  const observer=new IntersectionObserver(entries=>{
    for(const entry of entries){
      if(!entry.isIntersecting)continue;
      entry.target.classList.add('is-in-view');
      observer.unobserve(entry.target);
    }
  },{threshold:.12});
  tour.querySelectorAll('[data-tour-scene]').forEach(scene=>observer.observe(scene));
  window.addEventListener('pagehide',event=>{if(!event.persisted)observer.disconnect();},{once:true});
}
