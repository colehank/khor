import {mergeConfig} from 'vite';
import {fileURLToPath} from 'node:url';
import base from '../vite.config';
export default mergeConfig(base,{cacheDir:fileURLToPath(new URL('../../../target/vite-content-pan/',import.meta.url)),optimizeDeps:{entries:['lab/content-pan.html']}});
