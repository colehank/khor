export function counter(button){let count=0;button.addEventListener('click',()=>{button.textContent='点击计数：'+ ++count;});}
