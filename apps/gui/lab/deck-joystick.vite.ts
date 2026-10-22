import path from 'node:path';
import {defineConfig} from 'vite';
import base from '../vite.config';

// The alias is only a tuning adapter around the production DeckBand.
// SessionScreen, gesture model, selector, glass and tab rows are all real.
export default defineConfig({
  ...base,
  cacheDir:path.resolve(import.meta.dirname,'../../../target/joystick-vite'),
  resolve:{alias:[
    {find:'@/components/DeckBand',replacement:path.resolve(import.meta.dirname,'deck-joystick/Band.tsx')},
    {find:'@',replacement:path.resolve(import.meta.dirname,'../src')},
  ]},
  optimizeDeps:{entries:['lab/deck-joystick.html']},
  server:{host:'0.0.0.0',port:23637,strictPort:true},
});
