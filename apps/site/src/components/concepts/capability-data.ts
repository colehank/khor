// The same build travels through the same four-device network in both scenes.
// Only the operator changes. The desktop is available but not used in this run.
export type TaskDevice='mac'|'linux'|'windows'|'phone';
export type TaskPacket='files'|'terminal'|'web'|'message'|'check';
export type TaskPhase={
  id:string;duration:number;caption:string;detail:string;user:TaskDevice;
  route?:'upload'|'download'|'watch'|'preview'|'request'|'deliver';packet?:TaskPacket;
  focus:TaskDevice[];codex?:boolean;progress:[number,number];complete?:boolean;
};
export const workPhases:TaskPhase[]=[
  {id:'prepare',duration:3000,caption:'Prepare the project with Codex',detail:'Workstation · Atlas Web',user:'mac',focus:['mac'],codex:true,progress:[0,0]},
  {id:'transfer',duration:2800,caption:'Send the project to your server',detail:'atlas-web.tar · Workstation → Server',user:'mac',focus:['mac','linux'],route:'upload',packet:'files',progress:[0,0]},
  {id:'run',duration:3000,caption:'Start the build and tests',detail:'Server · npm run build && npm test',user:'mac',focus:['linux'],route:'upload',packet:'terminal',progress:[.04,.43]},
  {id:'handoff',duration:2400,caption:'Pick up the same task on mobile',detail:'The build keeps running on your server',user:'phone',focus:['linux'],progress:[.43,.63]},
  {id:'monitor',duration:3000,caption:'Watch the running task',detail:'Mobile · the same terminal session on Server',user:'phone',focus:['phone','linux'],route:'watch',packet:'terminal',progress:[.63,1]},
  {id:'preview',duration:2800,caption:'Open the server’s web preview',detail:'Mobile · Atlas Web is ready to check',user:'phone',focus:['phone','linux'],route:'preview',packet:'web',progress:[1,1]},
  {id:'report',duration:2600,caption:'Save the build report',detail:'8 tests passed · report sent to Workstation',user:'phone',focus:['mac','linux'],route:'download',packet:'files',progress:[1,1]},
  {id:'done',duration:4600,caption:'One task, across your devices',detail:'Build ready · 8 tests passed',user:'phone',focus:['phone'],progress:[1,1],complete:true},
];
export const agentPhases:TaskPhase[]=[
  {id:'request',duration:2800,caption:'You hand Khor the task',detail:'“Build Atlas Web and check the result”',user:'phone',focus:['mac','phone'],route:'request',packet:'message',progress:[0,0]},
  {id:'prepare',duration:2800,caption:'Khor works with Codex',detail:'Workstation · prepare the project',user:'phone',focus:['mac'],codex:true,progress:[0,0]},
  {id:'transfer',duration:2800,caption:'Khor sends the project to Server',detail:'atlas-web.tar · Workstation → Server',user:'phone',focus:['mac','linux'],route:'upload',packet:'files',progress:[0,0]},
  {id:'run',duration:3000,caption:'Khor starts the build and tests',detail:'Server · npm run build && npm test',user:'phone',focus:['mac','linux'],route:'upload',packet:'terminal',progress:[.04,.45]},
  {id:'monitor',duration:3000,caption:'Khor follows the running task',detail:'The terminal keeps running on Server',user:'phone',focus:['mac','linux'],route:'download',packet:'terminal',progress:[.45,1]},
  {id:'preview',duration:2600,caption:'Khor checks the web preview',detail:'Atlas Web · build ready, 8 tests passed',user:'phone',focus:['mac','linux'],route:'download',packet:'web',progress:[1,1]},
  {id:'report',duration:2600,caption:'Khor collects the build report',detail:'Server → Workstation',user:'phone',focus:['mac','linux'],route:'download',packet:'files',progress:[1,1]},
  {id:'deliver',duration:2800,caption:'Khor sends the result back to you',detail:'Mobile · preview and build report',user:'phone',focus:['mac','phone'],route:'deliver',packet:'check',progress:[1,1]},
  {id:'done',duration:4600,caption:'Your task, ready for review',detail:'Build ready · 8 tests passed',user:'phone',focus:['phone'],progress:[1,1],complete:true},
];
