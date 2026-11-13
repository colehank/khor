import type {Preview} from '@storybook/react-vite';
const preview: Preview = {
  parameters:{
    layout:'fullscreen',
    viewport:{options:{
      phone320:{name:'Mobile · 320 × 844',styles:{width:'320px',height:'844px'},type:'mobile'},
      phone390:{name:'Mobile · 390 × 844',styles:{width:'390px',height:'844px'},type:'mobile'},
      phone430:{name:'Mobile · 430 × 932',styles:{width:'430px',height:'932px'},type:'mobile'},
      android:{name:'Android · 412 × 915',styles:{width:'412px',height:'915px'},type:'mobile'},
      tablet:{name:'Tablet · 768 × 1024',styles:{width:'768px',height:'1024px'},type:'tablet'},
      desktop:{name:'Desktop · 1280 × 800',styles:{width:'1280px',height:'800px'},type:'desktop'},
      wide:{name:'Desktop · 1440 × 900',styles:{width:'1440px',height:'900px'},type:'desktop'},
    }},
    controls:{expanded:true},
    docs:{description:{component:'完整正式 App 与隔离夹具。视口工具切换尺寸和横竖屏；控件调节键盘占位、安全区、背景和不透明度。材质为 CSS 合成预览，真实系统键盘、触摸惯性与 AppKit 毛玻璃由原生 Lab 验证。'}},
  },
  globalTypes:{
    theme:{description:'App 主题',toolbar:{title:'主题',dynamicTitle:true,items:[{value:'dark',title:'深色'},{value:'light',title:'浅色'}]}},
    material:{description:'Desktop 窗口材质，CSS 模拟',toolbar:{title:'材质',dynamicTitle:true,items:[{value:'solid',title:'实色'},{value:'clear',title:'透明'},{value:'frost',title:'毛玻璃 · CSS 模拟'}]}},
    background:{description:'Desktop 背景',toolbar:{title:'背景',dynamicTitle:true,items:[{value:'wallpaper',title:'彩色背景'},{value:'grid',title:'网格'},{value:'light',title:'浅色'},{value:'dark',title:'深色'}]}},
  },
  initialGlobals:{viewport:{value:'desktop',isRotated:false},theme:'dark',material:'frost',background:'wallpaper'},
};
export default preview;
