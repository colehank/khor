import {mergeConfig} from 'vite';
import {fileURLToPath} from 'node:url';
import base from '../vite.config';
export default mergeConfig(base,{
  cacheDir:fileURLToPath(new URL('../../../target/vite-terminal-colors/',import.meta.url)),
  server:{host:'127.0.0.1',port:23623,strictPort:true},
  optimizeDeps:{entries:['lab/terminal-colors.html']},
});
