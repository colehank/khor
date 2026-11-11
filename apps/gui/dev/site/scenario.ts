const update = (sessionUpdate: string, values: Record<string, unknown>) => ({kind:'note',update:{update:{sessionUpdate,...values}}});
const text = (type: string, value: string) => update(type,{content:{type:'text',text:value}});
const boot = {kind:'agent',name:'Claude',replays:true,needs_login:false,images:false,embeds:false,lists:false};
export const scenario = {
  sampleImage:false,
  answerText:"\n\n检查已完成。修改已保留，你可以接着处理下一个任务。",
  sessionTitles:['Claude · 首页适配','Cursor · 整理样式','Codex · 检查修改','Khor · 设备协作','协作留言','Shell · 项目终端'],
  sessionPreviews:['macOS · 首页布局已调整','Linux · 正在整理页面样式','macOS · 检查项目修改','Windows · 协调设备上的任务','iOS · 查看设备与任务','macOS · 查看项目与修改记录'],
  sessionCategories:['claude','cursor','codex','khor','khor'],
  deviceNames:['MacBook Pro','Linux Server','Mac mini','Windows PC','iPhone','Android'],
  devicesOnline:true,
  reply:"Let's try Khor, [download it here.](/download/)",
  replyDelayMs:300,
  replyChunkMs:35,
  thinking:[],
  samples:[
    [boot,
      text('user_message_chunk','帮我把首页适配到手机上，完成后让我看一下。'),
      text('agent_message_chunk','已经调整了布局，右侧是项目预览。'),
      update('tool_call',{toolCallId:'site-read',title:'读取项目文件',kind:'read',status:'completed',rawInput:{file_path:'/preview/project/src/app.ts'}}),
      update('tool_call',{toolCallId:'site-edit',title:'调整移动布局',kind:'edit',status:'completed',content:[{type:'diff',path:'/preview/project/src/app.ts',oldText:'const columns = 3;\nconst gap = 24;',newText:'const columns = window.innerWidth < 760 ? 1 : 3;\nconst gap = 24;'}]}),
      update('plan',{entries:[{content:'查看现有首页',status:'completed'},{content:'调整布局',status:'completed'},{content:'预览修改',status:'completed'}]}),
      text('agent_message_chunk','文件和网页都在同一个窗口里。\n\n试着发条消息，或打开右侧的项目文件。'),
    ],
    [{...boot,name:'Cursor'},text('user_message_chunk','整理一下首页的样式。'),text('agent_message_chunk','正在整理标题、按钮和页面间距。')],
    [{...boot,name:'Codex'},text('user_message_chunk','检查一下刚才的修改。'),text('agent_message_chunk','已查看布局修改，可以继续确认下一步。')],
    [{...boot,name:'Khor'},text('user_message_chunk','看看其他设备上的任务。'),text('agent_message_chunk','设备、会话和项目文件都可以从这个窗口访问。')],
  ].map(frames=>[...frames,{kind:'turn',stop:'EndTurn'}]),
};
