import {useEffect,useRef,useState} from 'react';
type Request={key:string;value:string|boolean;name:string;run:(token:string)=>Promise<unknown>};
/** A queue acknowledgement is not a value acknowledgement. Keep the displayed
 * value canonical and wait for matching settings; stale failures cannot finish
 * a later request. Agents without correlated refusals time out as unconfirmed. */
export function useConfirmedSetting(values:ReadonlyMap<string,string|boolean>){
 const pending=useRef<(Request&{token:string})|null>(null);
 const [state,setState]=useState<{pending:boolean;name:string;error:boolean}>({pending:false,name:'',error:false});
 const retry=useRef<Request|null>(null),timer=useRef<ReturnType<typeof setTimeout>|null>(null);
 const clearTimer=()=>{if(timer.current!==null)clearTimeout(timer.current);timer.current=null;};
 const fail=(token:string)=>{if(pending.current?.token!==token)return;clearTimer();pending.current=null;setState(now=>({...now,pending:false,error:true}));};
 const request=(item:Request)=>{
  if(pending.current)return;
  const token=crypto.randomUUID();pending.current={...item,token};retry.current=item;
  setState({pending:true,name:item.name,error:false});
  timer.current=setTimeout(()=>fail(token),20000);
  void Promise.resolve().then(()=>item.run(token)).catch(()=>fail(token));
 };
 useEffect(()=>{
  const item=pending.current??retry.current;
  if(item&&values.get(item.key)===item.value){pending.current=null;retry.current=null;clearTimer();setState({pending:false,name:'',error:false});}
 },[values]);
 useEffect(()=>()=>{pending.current=null;clearTimer();},[]);
 return {...state,request,receipt:(token:string,error:string|null)=>{if(error)fail(token);},retry:()=>{if(retry.current)request(retry.current);}};
}
