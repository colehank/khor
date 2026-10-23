import {useState} from 'react';
import {createRoot} from 'react-dom/client';
import '@/styles/tokens.css';
import '@/styles/app.css';
import './gui-session-layout.css';
document.title='khor · 会话布局 Lab';

const scenes=[['gui/ai-elements-review','Claude · 工具与确认'],['gui/codex-review','Codex · 工具与补丁'],['wide/all','Khor · 网络任务'],['gui/ai-elements-plan','Claude · 计划'],['gui/ai-elements-stream','Claude · 流式回复']];
const sizes=[['390','窄屏 · 390'],['430','窄屏 · 430'],['667','横屏 · 667'],['1280','宽屏 · 1280']];
function Lab(){
  const [session,setSession]=useState(scenes[0][0]),[width,setWidth]=useState('390'),[theme,setTheme]=useState('light'),[keyboard,setKeyboard]=useState(false),[bottom,setBottom]=useState(false),[classic,setClassic]=useState(false),[attachment,setAttachment]=useState(false),[approval,setApproval]=useState(false);
  const wide=Number(width)>760,height=width==='667'?375:wide?800:844;
  const params=new URLSearchParams({bridge:location.port,labSession:session,theme,deck:bottom?'bottom':'top',keyboard:keyboard?'1':'0',phone:wide?'0':'1',composer:classic?'classic':'unified',attachment:attachment?'1':'0',approval:approval?'1':'0'});
  const src=`/?${params}`;
  return <main className="session-layout-lab">
    <aside className="layout-lab-controls">
      <p className="layout-lab-eyebrow">KHOR · LAB</p><h1>输入区统一</h1>
      <p>顶部管理模型与会话设置，底部只负责添加附件、输入和发送。点击顶部模型或输入框左侧的 ＋，可以查看新的分工。</p>
      <label>会话<select aria-label="会话" value={session} onChange={e=>setSession(e.target.value)}>{scenes.map(([id,label])=><option key={id} value={id}>{label}</option>)}</select></label>
      <label>尺寸<select aria-label="尺寸" value={width} onChange={e=>setWidth(e.target.value)}>{sizes.map(([id,label])=><option key={id} value={id}>{label}</option>)}</select></label>
      <label>外观<select aria-label="外观" value={theme} onChange={e=>setTheme(e.target.value)}><option value="light">浅色</option><option value="dark">深色</option></select></label>
      <label className="layout-lab-check"><input type="checkbox" checked={keyboard} onChange={e=>setKeyboard(e.target.checked)}/>模拟键盘升起</label>
      <label className="layout-lab-check"><input type="checkbox" checked={attachment} onChange={e=>setAttachment(e.target.checked)}/>附带示例文件</label>
      {session==='wide/all'&&<label className="layout-lab-check"><input type="checkbox" checked={approval} onChange={e=>setApproval(e.target.checked)}/>演示 Khor 待审批</label>}
      <label className="layout-lab-check"><input type="checkbox" checked={bottom} onChange={e=>setBottom(e.target.checked)}/>对比原来的底部甲板</label>
      <label className="layout-lab-check"><input type="checkbox" checked={classic} onChange={e=>setClassic(e.target.checked)}/>对比原来的输入区</label>
      <a href={src} target="_blank" rel="noreferrer">打开全屏预览 ↗</a>
      <p className="layout-lab-note">顶部药丸下拉打开总览，长按左右拨动切换。预览不连接真实智能体；键盘与安全区是浏览器模拟。</p>
      <p className="layout-lab-note">实验分支 lab/gui-session-layout，尚未合入 main。</p>
    </aside>
    <section className="layout-lab-stage" aria-label="会话预览">
      <iframe title="Khor 会话布局预览" key={src} src={src} style={{width:Number(width),height}}/>
    </section>
  </main>;
}
createRoot(document.getElementById('root')!).render(<Lab/>);
