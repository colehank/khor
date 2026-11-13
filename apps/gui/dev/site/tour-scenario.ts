// Finite, browser-only stories for website media capture. These do not
// execute commands, discover real devices or delegate to live agents.
export type TourScene='p2p'|'agent'|'terminal'|null;
export const agentTour={
  title:'检查支付回调，整理项目待办',
  preview:'Codex、Claude 已返回结果 · 2 项待确认',
  request:'让 build-01 的 Codex 检查支付回调，Mac mini 的 Claude 整理项目待办，再把结果一起给我。',
  tasks:[
    {machine:'build-01',agent:'Codex',task:'Check payment callbacks',result:'No duplicate charges'},
    {machine:'Mac mini',agent:'Claude',task:'Summarize project follow-ups',result:'5 actions · 2 decisions'},
  ],
};
const update=(sessionUpdate:string,values:Record<string,unknown>)=>({kind:'note',update:{update:{sessionUpdate,...values}}});
const say=(sessionUpdate:string,text:string)=>update(sessionUpdate,{content:{type:'text',text}});
const tool=(id:string,title:string,text:string)=>update('tool_call',{toolCallId:id,title,kind:'other',status:'completed',content:[{type:'content',content:{type:'text',text}}]});
export const agentTourFrames=[
  {kind:'agent',name:'Khor',replays:true,needs_login:false,images:false,embeds:false,lists:false},
  say('user_message_chunk',agentTour.request),
  say('agent_message_chunk','我会分别委派、跟踪进度，再汇总结果。暂不提交修改。'),
  tool('discover','发现设备与 agents','build-01 · Codex · /home/lin/atlas-api\nMac mini · Claude · /Users/lin/Documents/工作'),
  tool('delegate-codex','委派 Codex · build-01','检查支付回调的重复请求与并发情况。\n任务 payment-check 已完成：重复通知和 8 个并发回调都只产生 1 条入账记录。'),
  tool('delegate-claude','委派 Claude · Mac mini','整理远航项目周会记录与本周待办。\n任务 weekly-follow-ups 已完成：5 项待办，2 项待确认。'),
  tool('collect','汇集两台设备的结果','payment-check: completed\nweekly-follow-ups: completed'),
  say('agent_message_chunk','**两台设备的任务都已完成。**\n\nCodex 已检查支付回调，没有重复入账。Claude 已整理 5 项待办。\n\n还需你确认：审批人是否沿用，以及周二的培训安排在上午还是下午。'),
  {kind:'turn',stop:'EndTurn'},
];
export const terminalTour={
  title:'atlas-web · dev server',
  preview:'localhost:5173 · 开发服务运行中',
  machine:'MacBook Pro',
  path:'/Users/lin/Projects/atlas-web',
  url:'http://localhost:5173/',
  // The exact same bytes seed both desktop and phone captures.
  screen:'\x1b[2J\x1b[H\x1b[38;5;114mlin@MacBook-Pro\x1b[0m atlas-web % npm test -- export\r\n\r\n> atlas-web@0.8.0 test\r\n> vitest run export\r\n\r\n \x1b[38;5;114mPASS\x1b[0m src/export.test.ts (8 tests)\r\n\r\n Test Files  \x1b[38;5;114m1 passed\x1b[0m (1)\r\n      Tests  \x1b[38;5;114m8 passed\x1b[0m (8)\r\n   Duration  462ms\r\n\r\n\x1b[38;5;114mlin@MacBook-Pro\x1b[0m atlas-web % npm run dev\r\n\r\n> atlas-web@0.8.0 dev\r\n> vite\r\n\r\n  \x1b[38;5;114mVITE v7.0.0\x1b[0m  ready in 184 ms\r\n\r\n  \x1b[38;5;114m>\x1b[0m  Local:   http://localhost:5173/\r\n  \x1b[38;5;114m>\x1b[0m  press h + enter to show help\r\n',
};
