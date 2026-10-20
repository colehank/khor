/** A session directory's location editor. Completion and candidate scrolling
 * belong to shared fields; committing a directory asks its machine. */
import {useEffect,useLayoutEffect,useRef,useState} from 'react';
import {fetchLs,said,type DeviceRow} from '@/api';
import {MachineAvatar} from '@/components/Avatar';
import {PathField} from '@/components/PathField';
import {OmniMenu} from '@/components/OmniMenu';
import {MENU_TALLEST} from '@/components/ui/menu-viewport';
import {Input} from '@/components/ui/input';
import {useFieldDismiss} from '@/hooks/use-field-dismiss';
import {directoryPrefix,homeRelative} from '@/lib/path-input';
import {imeKey} from '@/lib/ime';
import {gui} from '@/gen/catalog';

type Props={
 device:DeviceRow;devices:DeviceRow[];path:string;base?:string;mobile:boolean;
 onDirectory:(device:DeviceRow,path:string)=>void;
 onFile:(device:DeviceRow,path:string)=>void;onLeave:()=>void;
};
export function ContentPathField({device,devices,path,base=path,mobile,onDirectory,onFile,onLeave}:Props) {
 const [draft,setDraft]=useState(path||'~/');
 const [choosing,setChoosing]=useState(false),[machineQuery,setMachineQuery]=useState(''),[at,setAt]=useState(-1);
 const [error,setError]=useState(''),[busy,setBusy]=useState(false),[room,setRoom]=useState(MENU_TALLEST);
 const seat=useRef<HTMLDivElement>(null),machineInput=useRef<HTMLInputElement>(null),pathInput=useRef<HTMLInputElement>(null);
 const turn=useRef(0);
 useFieldDismiss(true,()=>[seat.current],onLeave);
 useEffect(()=>()=>{turn.current++;},[]);
 useLayoutEffect(()=>{
   const el=seat.current,panel=el?.closest<HTMLElement>('[data-content-panel]');
   if(!el||!panel||!choosing)return;
   const sync=()=>setRoom(Math.max(0,Math.min(MENU_TALLEST,el.getBoundingClientRect().top-Math.max(panel.getBoundingClientRect().top,window.visualViewport?.offsetTop??0))));
   sync();const observer=new ResizeObserver(sync);observer.observe(panel);
   window.visualViewport?.addEventListener('resize',sync);
   return()=>{observer.disconnect();window.visualViewport?.removeEventListener('resize',sync);};
 },[choosing]);
 const candidates=devices.filter(d=>(d.shown+' '+d.name).toLocaleLowerCase().includes(machineQuery.replace(/^@/,'').toLocaleLowerCase()));
 const choose=(query='')=>{setChoosing(true);setMachineQuery(query);setAt(-1);};
 const pick=async(d:DeviceRow)=>{
   if(d.id===device.id){setChoosing(false);return;}
   const ticket=++turn.current;setBusy(true);setError('');
   try{const ls=await fetchLs(d.name,'');if(ticket===turn.current)onDirectory(d,ls.path);}
   catch(e){if(ticket===turn.current)setError(said(e));}
   finally{if(ticket===turn.current)setBusy(false);}
 };
 const go=async()=>{
   let value=draft.trim();if(!value)return;
   const ticket=++turn.current;setBusy(true);setError('');
   try{
     if(/^~([\\/]|$)/.test(value)){
       const home=await fetchLs(device.name,'');value=homeRelative(home.path,value.slice(2));
     }else if(!/^(\/|[a-z]:[\\/]|\\\\)/i.test(value)){
       const parent=base||(await fetchLs(device.name,'')).path;value=directoryPrefix(parent)+value;
     }
     if(ticket!==turn.current)return;
     try{const ls=await fetchLs(device.name,value);if(ticket===turn.current)onDirectory(device,ls.path);}
     catch{if(ticket===turn.current)onFile(device,value);}
   }catch(e){if(ticket===turn.current)setError(said(e));}
   finally{if(ticket===turn.current)setBusy(false);}
 };
 return <div ref={seat} className="content-path-editor" data-content-path-editor aria-busy={busy}>
   <button type="button" data-content-path-machine data-band-control data-glass-hit aria-label={`${gui.page_machine}：${device.shown}`} aria-expanded={choosing} className="content-location-face" onPointerDown={e=>e.preventDefault()} onClick={()=>{if(choosing)setChoosing(false);else choose();}}>
     <MachineAvatar face={device.face} className="size-tab-face"/>
   </button>
   {choosing?<div className="content-path-machine-query">
     <Input ref={machineInput} autoFocus aria-label={gui.page_choose_machine} placeholder={gui.page_choose_machine} value={machineQuery} onChange={e=>{setMachineQuery(e.target.value);setAt(-1);}} onKeyDown={e=>{
       if(imeKey(e))return;
       if((e.key==='ArrowDown'||e.key==='ArrowUp')&&candidates.length){e.preventDefault();setAt(n=>n<0?(e.key==='ArrowDown'?0:candidates.length-1):(n+(e.key==='ArrowDown'?1:-1)+candidates.length)%candidates.length);}
       if(e.key==='Enter'&&candidates[at<0?0:at]){e.preventDefault();void pick(candidates[at<0?0:at]);}
       if(e.key==='Escape'){e.preventDefault();e.stopPropagation();setChoosing(false);}
     }}/>
     <OmniMenu style={mobile?{maxHeight:room}:undefined} className={mobile?'content-location-offers content-location-offers-above':'content-location-offers'} rows={{groups:[{key:'machines',label:gui.devices_tab,candidates:candidates.map(d=>({key:d.id,label:d.shown,face:d.face,on:d.id===device.id})),empty:gui.no_matches}],at}} onPick={id=>{const d=candidates.find(d=>d.id===id);if(d)void pick(d);}}/>
   </div>:<PathField ref={pathInput} machine={device.name} value={draft} onValueChange={value=>{if(value.startsWith('@'))choose(value);else setDraft(value);}} files keepHome={false} autoFocus placement={mobile?'above':'below'} aria-label={gui.dir_path_hint} onFocus={e=>e.currentTarget.select()} onKeyDown={e=>{if(e.key==='Backspace'&&e.currentTarget.selectionStart===0&&e.currentTarget.selectionEnd===0){e.preventDefault();choose();}if(e.key==='Escape'&&e.currentTarget.getAttribute('aria-expanded')!=='true'){e.preventDefault();e.stopPropagation();onLeave();}}} onSubmit={()=>void go()}/>}
   {error&&<p role="alert" className="content-location-error">{error}</p>}
 </div>;
}
