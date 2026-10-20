import path from 'node:path';
import {mergeConfig} from 'vite';
import base from '../vite.config';
// Override only this isolated Lab. The production model and bundle stay intact.
export default mergeConfig(base,{
  resolve:{alias:[
    {find:'@/lib/deck-scrub',replacement:path.resolve(import.meta.dirname,'deck-scrub-pause-model.ts')},
    {find:'@',replacement:path.resolve(import.meta.dirname,'../src')},
  ]},
  optimizeDeps:{entries:['lab/deck-scrub.html']},
  server:{host:'0.0.0.0',port:1437,strictPort:true},
});
