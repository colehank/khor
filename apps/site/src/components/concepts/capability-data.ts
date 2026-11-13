// One task, two ways to drive it. The Mac owns the workspace, tests and preview;
// Codex on Linux receives a review request. Changing the client never moves files.
export type CapabilityTarget='review'|'edit'|'test'|'preview'|'changes';
export type CapabilityClient='mac'|'windows'|'phone';
export type CapabilityPhase={id:string;duration:number;caption:string;detail:string;targets:CapabilityTarget[];client:CapabilityClient;handoff?:'request'|'return';complete?:boolean};
export const taskSteps=[
  {id:'review',icon:'Codex',label:'Review logic',host:'Linux · Codex'},
  {id:'edit',icon:'files',label:'Edit files',host:'macOS'},
  {id:'test',icon:'terminal',label:'Run tests',host:'macOS'},
  {id:'preview',icon:'web',label:'Check preview',host:'macOS'},
  {id:'changes',icon:'git',label:'Review changes',host:'macOS'},
] as const;
const all=taskSteps.map(step=>step.id);
export const workPhases:CapabilityPhase[]=[
  {id:'review',duration:3500,caption:'Ask Codex to check the date logic',detail:'From your Mac → Codex on Linux',targets:['review'],client:'mac'},
  {id:'edit',duration:3500,caption:'Continue the fix from Windows',detail:'Edit src/export.ts in the same Mac workspace',targets:['edit'],client:'windows'},
  {id:'test',duration:3500,caption:'Run the tests from your phone',detail:'npm test runs on your Mac',targets:['test'],client:'phone'},
  {id:'preview',duration:3500,caption:'Check the fix on your phone',detail:'Open the Mac’s web app · localhost:5173',targets:['preview'],client:'phone'},
  {id:'changes',duration:3500,caption:'Review your changes',detail:'The same workspace · 3 files changed',targets:['changes'],client:'phone'},
  {id:'done',duration:4500,caption:'Different devices, one task finished',detail:'3 files changed · 8 tests passed',targets:all,client:'phone',complete:true},
];
export const agentPhases:CapabilityPhase[]=[
  {id:'request',duration:2400,caption:'You give Khor the task',detail:'“Fix the export dates”',targets:[],client:'phone',handoff:'request'},
  {id:'review',duration:2600,caption:'Khor asks Codex to review the logic',detail:'Linux · use the order’s local date',targets:['review'],client:'phone'},
  {id:'edit',duration:2600,caption:'Khor applies the fix',detail:'macOS · src/export.ts',targets:['edit'],client:'phone'},
  {id:'test',duration:2600,caption:'Khor runs the tests',detail:'macOS · npm test',targets:['test'],client:'phone'},
  {id:'preview',duration:2600,caption:'Khor checks the web preview',detail:'macOS · localhost:5173',targets:['preview'],client:'phone'},
  {id:'changes',duration:2600,caption:'Khor gathers the changes',detail:'macOS · 3 changed files',targets:['changes'],client:'phone'},
  {id:'done',duration:5200,caption:'Back to you, ready for review',detail:'3 files changed · 8 tests passed',targets:all,client:'phone',handoff:'return',complete:true},
];
