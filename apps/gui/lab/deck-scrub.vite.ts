import {mergeConfig} from 'vite';
import base from '../vite.config';
export default mergeConfig(base,{optimizeDeps:{entries:['lab/deck-scrub.html']}});
