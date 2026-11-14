export function mountCopyButtons() {
  document.querySelectorAll<HTMLButtonElement>('[data-copy]').forEach(button=>{
    const label=button.querySelector<HTMLElement>('[data-copy-label]');
    let reset=0;
    button.addEventListener('click',async()=>{
      const value=document.getElementById(button.dataset.copy!)?.textContent??'';
      try{await navigator.clipboard.writeText(value);button.setAttribute('data-copied','');if(label)label.textContent='Copied';}
      catch{button.removeAttribute('data-copied');if(label)label.textContent='Could not copy';}
      clearTimeout(reset);
      reset=window.setTimeout(()=>{button.removeAttribute('data-copied');if(label?.isConnected)label.textContent='';},1800);
    });
  });
}
