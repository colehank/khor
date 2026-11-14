/** Khor conversations share draft/preview state with ACP; their server
 * resolves the execution home instead of borrowing an ACP view lease. */
import {khorAgent,termDropStage,termDropForget} from '@/api';
import {usePendingFiles} from '@/views/chat-files';

export function useKhorFiles(session:string,into:string|null,machine:string|null){
 const putPaths=async(paths:string[],target:string|null)=>{
  const view=await khorAgent('put_paths','',{session,paths,into:target});
  if(!Array.isArray(view.data)||view.data.length!==paths.length||!view.data.every(path=>typeof path==='string'&&path.length>0))throw new Error('附件未能送达');
  return view.data as string[];
 };
 return usePendingFiles({putPaths,put:async(file,target)=>{
  const staged=await termDropStage(file);
  try{return (await putPaths([staged],target))[0];}
  finally{await termDropForget([staged]).catch(()=>{});}
 }},into,'khor:'+session,machine);
}
