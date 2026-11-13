import type {StorybookConfig} from '@storybook/react-vite';
const config: StorybookConfig = {
  stories:['./App.stories.tsx'],
  framework:'@storybook/react-vite',
  core:{disableTelemetry:true},
  viteFinal(config){
    config.base='./';
    config.optimizeDeps={...config.optimizeDeps,entries:['dev/lab/storybook/App.stories.tsx']};
    const target=process.env.KHOR_LAB_ORIGIN??'http://127.0.0.1:23910';
    config.server={...config.server,proxy:Object.fromEntries(['/app','/lab','/snapshots','/versions'].map(route=>[route,{target,changeOrigin:false}]))};
    return config;
  },
};
export default config;
