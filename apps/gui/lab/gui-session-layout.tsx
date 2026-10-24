import {useEffect,useRef,useState} from 'react';
import {createRoot} from 'react-dom/client';
import '@/styles/tokens.css';
import '@/styles/app.css';
import './gui-session-layout.css';
document.title='khor · 会话布局 Lab';

const scenes=[['gui/ai-elements-review','Claude · 工具与确认'],['gui/codex-review','Codex · 工具与补丁'],['wide/all','Khor · 网络任务'],['gui/ai-elements-plan','Claude · 计划'],['gui/ai-elements-stream','Claude · 流式回复']];
const sizes=[['390','窄屏 · 390'],['430','窄屏 · 430'],['667','横屏 · 667'],['1280','宽屏 · 1280']];
function Lab(){
  const [session,setSession]=useState(scenes[0][0]),[width,setWidth]=useState('390'),[theme,setTheme]=useState('light'),[keyboard,setKeyboard]=useState(false),[deck,setDeck]=useState('fan'),[count,setCount]=useState('5'),[classic,setClassic]=useState(false),[attachment,setAttachment]=useState(false),[approval,setApproval]=useState(false);
  const wide=Number(width)>760,height=width==='667'?375:wide?800:844;
  const params=new URLSearchParams({bridge:location.port,labSession:session,theme,deck,keyboard:keyboard?'1':'0',phone:wide?'0':'1',labTabs:[session,...scenes.map(([id])=>id).filter(id=>id!==session)].slice(0,Number(count)).join(','),composer:classic?'classic':'unified',attachment:attachment?'1':'0',approval:approval?'1':'0'});
  const src=`/?${params}`;
  const preview=useRef<HTMLIFrameElement>(null),observer=useRef<MutationObserver|null>(null);
  const drafts=useRef(new Map<string,{text:string;start:number;end:number}>());
  useEffect(()=>{
    const changed=(event:MessageEvent)=>{
      const data=event.data;
      if(event.origin!==location.origin||event.source!==preview.current?.contentWindow||data?.type!=='khor.layout.draft'||typeof data.text!=='string'||!scenes.some(([id])=>id===data.session))return;
      drafts.current.set(data.session,{text:data.text,start:data.text.length,end:data.text.length});
    };
    window.addEventListener('message',changed);
    return()=>{window.removeEventListener('message',changed);observer.current?.disconnect();};
  },[]);
  const captureDraft=()=>{
    const frame=preview.current,field=frame?.contentDocument?.querySelector<HTMLTextAreaElement>('[data-composer-text]');
    if(!frame||!field)return;
    const selected=JSON.parse(frame.contentWindow?.localStorage.getItem('khor.sessions.tabs')??'{}').active;
    const current=typeof selected==='string'&&scenes.some(([id])=>id===selected)?selected:session;
    drafts.current.set(current,{text:field.value,start:field.selectionStart,end:field.selectionEnd});
    if(current!==session)setSession(current);
  };
  const restoreDraft=()=>{
    observer.current?.disconnect();
    const doc=preview.current?.contentDocument,saved=drafts.current.get(session);if(!doc||!saved)return;
    const restore=()=>{
      const field=doc.querySelector<HTMLTextAreaElement>('[data-composer-text]');if(!field)return;
      observer.current?.disconnect();
      Object.getOwnPropertyDescriptor(Object.getPrototypeOf(field),'value')?.set?.call(field,saved.text);
      field.dispatchEvent(new Event('input',{bubbles:true}));field.setSelectionRange(saved.start,saved.end);
    };
    observer.current=new MutationObserver(restore);observer.current.observe(doc,{childList:true,subtree:true});restore();
  };
  return <main className="session-layout-lab">
    <aside className="layout-lab-controls" onChangeCapture={captureDraft}>
      <p className="layout-lab-eyebrow">KHOR · LAB</p><h1>会话操作区</h1>
      <p>左上直接选择模型与思考强度，右上新建会话。会话头像放在输入栏最右侧，点开展开会话药丸。长按轻拨预选，保持偏移循环翻选，回中暂停、反向倒退，松手停靠确认。扇形与药丸共用窗口顺序，最多五个会话按弧形排列，头像右侧显示标题，整组近大远小；空间不足时减少可见项。上推独立展开全部标签页，点开的药丸保留原来的连续翻选。</p>
      <label>会话<select aria-label="会话" value={session} onChange={e=>setSession(e.target.value)}>{scenes.map(([id,label])=><option key={id} value={id}>{label}</option>)}</select></label>
      <label>窗口数量<select aria-label="窗口数量" value={count} onChange={e=>setCount(e.target.value)}><option value="3">3 个窗口</option><option value="4">4 个窗口</option><option value="5">5 个窗口</option></select></label>
      <label>尺寸<select aria-label="尺寸" value={width} onChange={e=>setWidth(e.target.value)}>{sizes.map(([id,label])=><option key={id} value={id}>{label}</option>)}</select></label>
      <label>外观<select aria-label="外观" value={theme} onChange={e=>setTheme(e.target.value)}><option value="light">浅色</option><option value="dark">深色</option></select></label>
      <label className="layout-lab-check"><input type="checkbox" checked={keyboard} onChange={e=>setKeyboard(e.target.checked)}/>模拟键盘升起</label>
      <label className="layout-lab-check"><input type="checkbox" checked={attachment} onChange={e=>setAttachment(e.target.checked)}/>附带示例文件</label>
      {session==='wide/all'&&<label className="layout-lab-check"><input type="checkbox" checked={approval} onChange={e=>setApproval(e.target.checked)}/>演示 Khor 待审批</label>}
      <label>布局<select aria-label="布局" value={deck} onChange={e=>setDeck(e.target.value)}><option value="fan">右手扇形 · 循环拨杆</option><option value="compose">右侧头像 · 连续拨杆</option><option value="light">轻量页头 · 头像与文字</option><option value="combined">底部组合 · 输入与药丸</option><option value="top">顶部药丸 · 底部输入</option><option value="bottom">原底部 · 分开的胶囊</option></select></label>
      <label className="layout-lab-check"><input type="checkbox" checked={classic} onChange={e=>setClassic(e.target.checked)}/>对比原来的输入区</label>
      <a href={src} target="_blank" rel="noreferrer">打开全屏预览 ↗</a>
      <p className="layout-lab-note">{deck==='fan'?'右侧头像：长按轻拨预选，保持偏移按三档速度循环翻选，回中暂停、反向倒退，松手确认；正上方展开列表。顺序与药丸一致，预选不打开中间内容。':deck==='compose'?'右侧头像：点击展开药丸；横拨切换，长按进入浅 U 拨杆；回中暂停，松手确认，上推进入全部标签页。':deck==='light'?'点击头像或标题打开总览；长按头像显示轮盘，沿弧线滑动，回到头像取消。':(deck==='top'?'顶部药丸下拉':'底部药丸上滑')+'打开总览，长按左右拨动切换。'}会话内切换保留各自的文字草稿。预览不连接真实智能体；键盘与安全区是浏览器模拟。</p>
      {wide&&<p className="layout-lab-note">宽屏沿用顶部标签，切换到窄屏查看本轮布局。</p>}
      <p className="layout-lab-note">实验分支 lab/gui-session-layout，尚未合入 main。</p>
    </aside>
    <section className="layout-lab-stage" aria-label="会话预览">
      <iframe ref={preview} onLoad={restoreDraft} title="Khor 会话布局预览" key={src} src={src} style={{width:Number(width),height}}/>
    </section>
  </main>;
}
createRoot(document.getElementById('root')!).render(<Lab/>);
