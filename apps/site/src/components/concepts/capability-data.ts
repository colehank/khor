// Four readable beats. Routing handoffs share their caption instead of adding
// a new explanatory step for every file or command.
export type TaskDevice='mac'|'linux'|'windows'|'phone';
export type TaskEndpoint=TaskDevice|'khor'|'you';
export type TaskPacket='files'|'terminal'|'web'|'message'|'check'|'metrics';
export type TaskMark=TaskPacket|'agent'|'edit';
export type TaskEffect='request'|'plan'|'train'|'refine'|'update'|'preview'|'collect'|'result';
export type TaskFlow={from:TaskEndpoint;to:TaskEndpoint};
export type TaskPhase={
  id:string;duration:number;caption:string;user:TaskDevice|'outside';
  flow:TaskFlow;packet:TaskPacket;arrival:TaskEffect;
  focus:TaskEndpoint[];run?:1|2;complete?:boolean;
};
export const taskPackets:TaskPacket[]=['files','terminal','web','message','check','metrics'];
export const taskMarks:TaskMark[]=[...taskPackets,'agent','edit'];

export const workPhases:TaskPhase[]=[
  {id:'train',duration:3000,caption:'Train on Server',user:'mac',focus:['mac','linux'],flow:{from:'mac',to:'linux'},packet:'terminal',arrival:'train',run:1},
  {id:'refine',duration:2300,caption:'Refine on your laptop',user:'windows',focus:['linux','windows'],flow:{from:'linux',to:'windows'},packet:'metrics',arrival:'refine'},
  {id:'update',duration:2100,caption:'Let Server run it again',user:'windows',focus:['windows','linux'],flow:{from:'windows',to:'linux'},packet:'files',arrival:'update',run:2},
  {id:'preview',duration:3000,caption:'Try it on your phone',user:'phone',focus:['linux','phone'],flow:{from:'linux',to:'phone'},packet:'web',arrival:'preview',complete:true},
];
export const agentPhases:TaskPhase[]=[
  {id:'request',duration:800,caption:'Give Khor a goal',user:'outside',focus:['you','khor'],flow:{from:'you',to:'khor'},packet:'message',arrival:'request'},
  {id:'plan',duration:800,caption:'Give Khor a goal',user:'outside',focus:['khor','mac'],flow:{from:'khor',to:'mac'},packet:'message',arrival:'plan'},
  {id:'train',duration:2200,caption:'Khor starts the training',user:'outside',focus:['khor','linux'],flow:{from:'khor',to:'linux'},packet:'terminal',arrival:'train',run:1},
  {id:'refine',duration:1800,caption:'Khor coordinates the revision',user:'outside',focus:['khor','windows'],flow:{from:'khor',to:'windows'},packet:'metrics',arrival:'refine'},
  {id:'update',duration:1800,caption:'Khor coordinates the revision',user:'outside',focus:['windows','linux'],flow:{from:'windows',to:'linux'},packet:'files',arrival:'update',run:2},
  {id:'preview',duration:800,caption:'Khor brings it back to you',user:'outside',focus:['linux','phone'],flow:{from:'linux',to:'phone'},packet:'web',arrival:'preview'},
  {id:'collect',duration:600,caption:'Khor brings it back to you',user:'outside',focus:['linux','khor'],flow:{from:'linux',to:'khor'},packet:'files',arrival:'collect'},
  {id:'deliver',duration:2200,caption:'Khor brings it back to you',user:'outside',focus:['khor','you'],flow:{from:'khor',to:'you'},packet:'web',arrival:'result',complete:true},
];
