import type {StorybookConfig} from '@storybook/react-vite';
const config: StorybookConfig = {
  stories:['./App.stories.tsx'],
  framework:'@storybook/react-vite',
  core:{disableTelemetry:true},
  viteFinal(config){
    config.base='./';
    config.optimizeDeps={...config.optimizeDeps,entries:['dev/lab/storybook/App.stories.tsx']};
    const target=process.env.KHOR_LAB_ORIGIN??'http://127.0.0.1:23910';
    config.define={...config.define,'import.meta.env.KHOR_LAB_ORIGIN':JSON.stringify(target)};
    return config;
  },
};
export default config;
