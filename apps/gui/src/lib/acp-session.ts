/** ACP v1 option shapes. IDs and choices are owned by the agent. */
export type SessionConfigValue={value:string;name:string;description?:string|null};
export type SessionConfigGroup={group:string;name:string;options:SessionConfigValue[]};
export type SessionConfigOption={id:string;name:string;description?:string|null;category?:string|null;type:string;currentValue?:string|boolean;options?:(SessionConfigValue|SessionConfigGroup)[]};
export type AgentProtocol={agentInfo?:{name:string;title?:string;version?:string};capabilities?:{sessionCapabilities?:Record<string,unknown>;auth?:{logout?:unknown}};authMethods?:{id:string;name:string;type?:string;description?:string}[]};

export function agentPresentation(agent:string){
  switch(agent){
    case 'codex':return {name:'Codex',source:'内置对话接入',native:true};
    case 'claude':return {name:'Claude',source:'内置对话接入',native:true};
    case 'codex-acp':return {name:'Codex（ACP 扩展）',source:'独立 ACP 适配器',native:false};
    case 'claude-acp':return {name:'Claude（ACP 扩展）',source:'独立 ACP 适配器',native:false};
    default:return {name:agent,source:'ACP 对话接入',native:false};
  }
}

export function agentSetupFailure(reason:unknown,error:unknown):string{
  switch(reason){
    case 'npm_cache_unavailable':return '此机器的 npm 缓存不可用，修复后重试';
    case 'npm_package_unavailable':return '找不到此版本的智能体扩展';
    case 'npm_download_failed':return '智能体扩展下载失败，检查此机器的网络后重试';
    case 'node_unavailable':return '此机器未找到 Node.js，安装后重试';
  }
  if(typeof error==='string'&&/incoming[_ ]transport[_ ]closed/i.test(error))return '未能连接智能体，请重试';
  return typeof error==='string'&&error?error:'无法读取智能体能力';
}

export function configOptions(value:unknown):SessionConfigOption[]{
  return Array.isArray(value)?value.filter((v):v is SessionConfigOption=>v&&typeof v==='object'&&typeof v.id==='string'&&typeof v.name==='string'&&(v.type==='boolean'?typeof v.currentValue==='boolean':v.type==='select'&&typeof v.currentValue==='string'&&Array.isArray(v.options))):[];
}
