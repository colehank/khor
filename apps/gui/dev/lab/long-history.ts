type Frame=Record<string,unknown>;
const note=(sessionUpdate:string,rest:Record<string,unknown>):Frame=>({kind:'note',update:{update:{sessionUpdate,...rest}}});
const text=(sessionUpdate:string,value:string)=>note(sessionUpdate,{content:{type:'text',text:value}});
/** Replay-shaped data, not a replacement history reader or body folder. */
export function longHistoryFrames(count=240,image?:string):Frame[]{
  const frames:Frame[]=[];
  for(let turn=1;turn<=count;turn++){
    frames.push(text('user_message_chunk',`第${turn}轮：检查工作区中的记录，并说明这次处理的结果。`));
    if(image&&turn%40===0)frames.push(note('user_message_chunk',{content:{type:'image',mimeType:'image/png',name:`历史附件-${turn}.png`,data:image}}));
    const rich=turn%12===1;
    if(rich){
      frames.push(text('agent_thought_chunk',`先核对第${turn}轮的输入与前后文，再检查文件与执行结果。`));
      frames.push(note('tool_call',{toolCallId:`history-read-${turn}`,title:'Read README.md',kind:'read',status:'completed',rawInput:{file_path:'/preview/project/README.md'},
        content:[{type:'content',content:{type:'text',text:'这是长对话验收数据，没有读取真实文件。'}}]}));
    }
    const answer=rich
      ?`第${turn}轮已检查。下面保留中文、English、表格、链接和代码，便于反复测试连续滚动。\n\n| 项目 | 结果 |\n| --- | --- |\n| 文件 | 已读取 |\n| 记录 | 已核对 |\n\n\`\`\`ts\nconst revision = ${turn};\nconsole.log(revision);\n\`\`\`\n\n[打开示例文件](/preview/project/README.md)\n\n继续向上浏览时，应保留当前阅读位置；回到底部后，可以继续阅读最新回复。`
      :`第${turn}轮的记录已经核对，结果已保存在工作区，可以继续处理下一项。`;
    const glyphs=Array.from(answer);
    for(let part=0;part<20;part++){
      const chunk=glyphs.slice(Math.floor(glyphs.length*part/20),Math.floor(glyphs.length*(part+1)/20)).join('');
      if(chunk)frames.push(text('agent_message_chunk',chunk));
    }
    frames.push({kind:'turn',stop:'EndTurn'});
  }
  return frames;
}
