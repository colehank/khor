const update = (sessionUpdate: string, values: Record<string, unknown>) => ({kind:'note',update:{update:{sessionUpdate,...values}}});
const text = (type: string, value: string) => update(type,{content:{type:'text',text:value}});
const boot = {kind:'agent',name:'Claude',replays:true,needs_login:false,images:false,embeds:false,lists:false};
const ask = (id: number, title: string) => ({kind:'ask',ask:id,title,options:[['allow-once','本次允许'],['deny','暂不运行']]});
export const scenario = {
  sampleImage:false,
  answerText:"\n\n检查已完成。修改已保留，你可以接着处理下一个任务。",
  sessionTitles:['首页适配','检查移动布局','整理项目文档','Codex · 检查修改','设备与任务','项目终端'],
  sessionPreviews:['移动布局已调整，等你确认','准备检查标题和按钮','项目说明已整理','已查看首页修改','设备已连接','查看项目与修改记录'],
  deviceNames:['MacBook Pro','Linux','iPhone'],
  reply:'收到。这里使用示例数据，你可以继续查看会话、文件和修改记录。',
  thinking:['先查看任务和项目文件。','\n确认需要修改的内容。','\n整理结果，等待你的下一步。'],
  samples:[
    [boot,
      text('user_message_chunk','帮我把首页适配到手机上，完成后让我看一下。'),
      text('agent_message_chunk','已经调整了布局，你可以先看一下修改。'),
      update('tool_call',{toolCallId:'site-read',title:'读取项目文件',kind:'read',status:'completed',rawInput:{file_path:'/preview/project/src/app.ts'}}),
      update('tool_call',{toolCallId:'site-edit',title:'调整移动布局',kind:'edit',status:'completed',content:[{type:'diff',path:'/preview/project/src/app.ts',oldText:'const columns = 3;\nconst gap = 24;',newText:'const columns = window.innerWidth < 760 ? 1 : 3;\nconst gap = 24;'}]}),
      update('plan',{entries:[{content:'查看现有首页',status:'completed'},{content:'适配手机布局',status:'completed'},{content:'确认修改',status:'in_progress'}]}),
      text('agent_message_chunk','手机上已经改为单列，标题和按钮保留足够的空间。\n\n你可以展开上面的修改，或去工作区查看文件。'),
      ask(101,'要在远程机器上运行检查吗？'),
    ],
    [boot,text('user_message_chunk','再检查一下手机上的布局。'),text('agent_message_chunk','布局已调整，等你确认后继续。'),ask(201,'继续检查移动布局？')],
    [boot,text('user_message_chunk','整理一下项目说明。')],
    [{...boot,name:'Codex'},text('user_message_chunk','看看刚才的修改有没有遗漏。'),text('agent_message_chunk','已查看布局修改，手机上的标题、正文和按钮都能正常显示。')],
  ],
};
