// One illustrative project: files, preview and tests stay on the Mac.
// The Linux agent receives a delegated review, not an implied filesystem sync.
export type CapabilityTarget='files'|'web'|'git'|'terminal'|'codex'|'claude'|'cursor';
export type CapabilityPhase={id:string;duration:number;caption:string;detail:string;targets:CapabilityTarget[];complete?:boolean};
export const workPhases:CapabilityPhase[]=[
  {id:'web',duration:3600,caption:'Open a web app on your Mac',detail:'macOS · localhost:5173',targets:['web']},
  {id:'files',duration:3600,caption:'Edit files and review their changes',detail:'macOS · atlas-web · 3 changed files',targets:['files','git']},
  {id:'terminal',duration:3600,caption:'Connect to a running terminal',detail:'macOS · npm run dev',targets:['terminal']},
  {id:'agents',duration:4200,caption:'Pick up an agent session on Linux',detail:'Codex · export review',targets:['codex']},
];
export const agentPhases:CapabilityPhase[]=[
  {id:'request',duration:2200,caption:'One task, across your net',detail:'Fix the export dates',targets:[]},
  {id:'read',duration:2400,caption:'Read the source',detail:'macOS · src/export.ts',targets:['files']},
  {id:'delegate',duration:3200,caption:'Ask Codex to review the date logic',detail:'Linux · use the order’s local date',targets:['codex']},
  {id:'edit',duration:2400,caption:'Apply the fix',detail:'macOS · 3 files changed',targets:['files','git']},
  {id:'test',duration:2800,caption:'Run the tests',detail:'macOS · npm test',targets:['terminal']},
  {id:'preview',duration:2600,caption:'Check the web preview',detail:'macOS · localhost:5173',targets:['web']},
  {id:'done',duration:5200,caption:'Ready for you to review',detail:'3 files changed · 8 tests passed',targets:['files','git','terminal','web','codex'],complete:true},
];
