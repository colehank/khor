import {counter} from './counter.js';
const data=await fetch(new URL('./data.json',import.meta.url)).then(r=>r.json());
document.querySelector('#result').textContent=data.message;
document.documentElement.dataset.resourcesReady='yes';
counter(document.querySelector('#counter'));
