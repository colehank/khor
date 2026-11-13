// Four captions per loop. Transfers, execution and readable result holds have
// separate timings so a longer loop actually slows the visible actions.
export type TaskDevice='mac'|'linux'|'windows'|'phone';
export type TaskEndpoint=TaskDevice|'khor'|'you';
export type TaskSession='mac'|'linux';
export type TaskPacket='files'|'web'|'message'|'check'|'link';
export type TaskEffect='accept'|'claude'|'file-notice'|'server-request'|'file'|'web-notice'|'web-request'|'page'|'deliver';
export type TaskFlow={from:TaskEndpoint;to:TaskEndpoint};
export type TaskPhase={
  id:string;duration:number;caption:string;user:'windows'|'phone'|'outside';
  flow?:TaskFlow;packet?:TaskPacket;arrival?:TaskEffect;session?:TaskSession;complete?:boolean;
};
export const taskTiming={departure:200,transfer:1200,fade:160,outputHold:1000,handoff:1000} as const;
export const taskPackets:TaskPacket[]=['files','web','message','check','link'];
export const taskSessions=[{device:'mac',name:'Claude Code',mark:'Claude'},{device:'linux',name:'Codex',mark:'Codex'}] as const;

export const workPhases:TaskPhase[]=[
  {id:'dialogue',duration:3800,caption:'Talk to Claude Code',user:'windows',flow:{from:'windows',to:'mac'},packet:'message',arrival:'claude',session:'mac'},
  {id:'file-receipt',duration:1600,caption:'Talk to Claude Code',user:'windows',flow:{from:'mac',to:'windows'},packet:'check',arrival:'file-notice'},
  {id:'instruct',duration:1800,caption:'Continue with Codex',user:'windows',flow:{from:'windows',to:'linux'},packet:'message',arrival:'server-request'},
  {id:'transfer',duration:4000,caption:'Continue with Codex',user:'windows',flow:{from:'mac',to:'linux'},packet:'files',arrival:'file',session:'linux'},
  {id:'web-receipt',duration:2400,caption:'Web ready on Server',user:'windows',flow:{from:'linux',to:'windows'},packet:'link',arrival:'web-notice'},
  {id:'handoff',duration:1400,caption:'Open it on your phone',user:'phone'},
  {id:'browse',duration:1800,caption:'Open it on your phone',user:'phone',flow:{from:'phone',to:'linux'},packet:'link',arrival:'web-request'},
  {id:'page',duration:3200,caption:'Open it on your phone',user:'phone',flow:{from:'linux',to:'phone'},packet:'web',arrival:'page',complete:true},
];
export const agentPhases:TaskPhase[]=[
  {id:'delegate',duration:2400,caption:'Hand the task to Khor',user:'outside',flow:{from:'you',to:'khor'},packet:'message',arrival:'accept'},
  {id:'dialogue',duration:3800,caption:'Khor starts Claude Code',user:'outside',flow:{from:'khor',to:'mac'},packet:'message',arrival:'claude',session:'mac'},
  {id:'file-receipt',duration:1600,caption:'Khor starts Claude Code',user:'outside',flow:{from:'mac',to:'khor'},packet:'check',arrival:'file-notice'},
  {id:'instruct',duration:1800,caption:'Khor passes the file to Codex',user:'outside',flow:{from:'khor',to:'linux'},packet:'message',arrival:'server-request'},
  {id:'transfer',duration:4000,caption:'Khor passes the file to Codex',user:'outside',flow:{from:'mac',to:'linux'},packet:'files',arrival:'file',session:'linux'},
  {id:'web-receipt',duration:2400,caption:'Khor brings the result back',user:'outside',flow:{from:'linux',to:'khor'},packet:'link',arrival:'web-notice'},
  {id:'deliver',duration:4000,caption:'Khor brings the result back',user:'outside',flow:{from:'khor',to:'you'},packet:'link',arrival:'deliver',complete:true},
];
