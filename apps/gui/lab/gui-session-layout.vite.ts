import {defineConfig,mergeConfig} from 'vite';
import preview from '../scripts/session-navigation-preview.vite';
export default mergeConfig(preview,defineConfig({server:{port:23795,strictPort:true}}));
