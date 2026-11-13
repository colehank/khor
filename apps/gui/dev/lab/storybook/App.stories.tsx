import {useEffect,useMemo,useRef} from 'react';
import type {Meta,StoryObj} from '@storybook/react-vite';
import {scenes,type Scene} from '../scenes';
import {type PreviewState} from '../preview-state';

type Args={scene:Scene;machines:number;items:number;opacity:number;keyboard:number;safeArea:boolean;platform:'auto'|'desktop'|'ios'|'android';history:boolean;width:number;height:number};
type Props=Args&{environment:PreviewState};
function AppPreview({scene,machines,items,history,width,height,environment}:Props){
  const frame=useRef<HTMLIFrameElement>(null),current=useRef(environment);current.current=environment;
  const src=useMemo(()=>{
    const version=location.pathname.match(/\/versions\/([a-zA-Z0-9]+)\/assets\/workbench\//)?.[1];
    const origin=import.meta.env.DEV?import.meta.env.KHOR_LAB_ORIGIN:location.origin;
    const url=new URL(version?`/snapshots/${version}/app`:'/app/',origin);
    url.searchParams.set('scene',scene);url.searchParams.set('machines',String(machines));url.searchParams.set('items',String(items));
    if(history)url.searchParams.set('history','long');
    url.searchParams.set('environment',JSON.stringify(current.current));
    return url.href;
  },[scene,machines,items,history]);
  const appOrigin=new URL(src).origin;
  const send=()=>frame.current?.contentWindow?.postMessage({type:'khor.lab.environment',value:current.current},appOrigin);
  useEffect(()=>{send();},[environment]);
  useEffect(()=>{
    const receive=(event:MessageEvent)=>{
      if(event.source!==frame.current?.contentWindow||event.origin!==appOrigin||event.data?.type!=='khor.lab.ready')return;
      frame.current!.dataset.labAppReady=String(event.data.tree);
      frame.current!.dataset.labInput=String(event.data.pointer);
      send();
    };
    addEventListener('message',receive);return()=>removeEventListener('message',receive);
  },[appOrigin]);
  return <div data-lab-workbench-preview style={{position:'relative',width:width||'100%',height:height||'100vh',overflow:'hidden',margin:'auto'}}>
    <iframe ref={frame} data-lab-app-frame title="Khor 正式 App Lab" src={src} onLoad={send} style={{display:'block',width:'100%',height:'100%',border:0}}/>
    {environment.platform!=='desktop'&&environment.keyboard>0&&<div data-lab-keyboard-placeholder aria-label="模拟键盘占位" style={{position:'absolute',insetInline:0,bottom:0,height:environment.keyboard,display:'grid',placeItems:'center',background:'Canvas',color:'CanvasText',borderBlockStart:'1px solid GrayText',pointerEvents:'none'}}>
      <span>键盘占位 · {environment.keyboard}px</span>
    </div>}
  </div>;
}
const meta={
  id:'khor-app',title:'Khor/完整 App',
  args:{scene:'list',machines:3,items:5,opacity:20,keyboard:0,safeArea:true,platform:'auto',history:false,width:0,height:0},
  argTypes:{
    scene:{name:'页面场景',control:'select',options:Object.keys(scenes).filter(key=>key!=='live')},
    machines:{name:'机器数量',control:'select',options:[0,1,3,6]},items:{name:'会话数量',control:'select',options:[2,3,5]},
    opacity:{name:'Desktop 不透明度',control:{type:'range',min:0,max:100,step:5}},
    keyboard:{name:'键盘占位高度',description:'Mobile 下保留底部空间；不是系统输入法',control:{type:'range',min:0,max:400,step:8}},
    safeArea:{name:'模拟安全区',control:'boolean'},
    platform:{name:'布局平台',description:'自动随所选视口；浏览器的真实输入能力不被伪造',control:'select',options:['auto','desktop','ios','android']},
    history:{name:'长对话历史',control:'boolean'},
    width:{name:'自定义宽度',description:'0 跟随视口工具',control:{type:'number',min:0,max:2560}},
    height:{name:'自定义高度',description:'0 跟随视口工具',control:{type:'number',min:0,max:1600}},
  },
  render:(args,context)=>{
    const viewport=String(context.globals.viewport?.value??'desktop');
    const platform=args.platform==='auto'?(viewport==='android'?'android':viewport.startsWith('phone')||viewport==='tablet'?'ios':'desktop'):args.platform;
    const mobile=platform!=='desktop';
    const environment:PreviewState={platform,theme:context.globals.theme??'dark',material:context.globals.material??'frost',background:context.globals.background??'wallpaper',opacity:args.opacity,
      keyboard:mobile?args.keyboard:0,safeTop:mobile&&args.safeArea?(platform==='ios'?47:24):0,safeBottom:mobile&&args.safeArea?(platform==='ios'?34:24):0};
    return <AppPreview {...args} environment={environment}/>;
  },
} satisfies Meta<Args>;
export default meta;
type Story=StoryObj<typeof meta>;
export const Sessions:Story={name:'会话列表'};
export const Khor:Story={name:'Khor 会话',args:{scene:'khor'}};
export const GUI:Story={name:'GUI 对话',args:{scene:'gui'}};
export const Terminal:Story={name:'终端',args:{scene:'terminal'}};
export const Devices:Story={name:'机器',args:{scene:'devices'}};
export const Workspace:Story={name:'工作区',args:{scene:'workspaces'}};
export const Files:Story={name:'工作区文件',args:{scene:'workspace'}};
export const Quota:Story={name:'额度',args:{scene:'quota'}};
export const Browser:Story={name:'浏览器',args:{scene:'browser'}};
export const History:Story={name:'长对话历史',args:{scene:'gui',history:true}};
