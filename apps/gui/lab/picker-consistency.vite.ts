import {mergeConfig} from 'vite';
import {fileURLToPath} from 'node:url';
import base from '../vite.config';
export default mergeConfig(base,{cacheDir:fileURLToPath(new URL('../../../target/vite-picker-consistency/',import.meta.url)),optimizeDeps:{entries:['lab/session-launch.html']}});
