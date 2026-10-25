/** ACP v1 option shapes. IDs and choices are owned by the agent. */
export type SessionConfigValue={value:string;name:string;description?:string|null};
export type SessionConfigGroup={group:string;name:string;options:SessionConfigValue[]};
export type SessionConfigOption={id:string;name:string;description?:string|null;category?:string|null;type:string;currentValue?:string|boolean;options?:(SessionConfigValue|SessionConfigGroup)[]};
export type AgentProtocol={capabilities?:{sessionCapabilities?:Record<string,unknown>;auth?:{logout?:unknown}};authMethods?:{id:string;name:string;type?:string;description?:string}[]};

export function configOptions(value:unknown):SessionConfigOption[]{
  return Array.isArray(value)?value.filter((v):v is SessionConfigOption=>v&&typeof v==='object'&&typeof v.id==='string'&&typeof v.name==='string'&&(v.type==='boolean'?typeof v.currentValue==='boolean':v.type==='select'&&typeof v.currentValue==='string'&&Array.isArray(v.options))):[];
}
