export function mountCopyButtons() {
  document.querySelectorAll<HTMLButtonElement>('[data-copy]').forEach(button=>button.addEventListener('click',async()=>{
    const value=document.getElementById(button.dataset.copy!)?.textContent??'';
    try{await navigator.clipboard.writeText(value);button.textContent='已复制';}
    catch{button.textContent='复制未成功';}
    setTimeout(()=>{if(button.isConnected)button.textContent='复制';},1800);
  }));
}
