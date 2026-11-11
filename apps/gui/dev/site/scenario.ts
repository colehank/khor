import {exportAfter,exportBefore,machines,meeting,roots,tasks,terminalScreen} from './world';

const update = (sessionUpdate: string, values: Record<string, unknown>) => ({kind:'note',update:{update:{sessionUpdate,...values}}});
const text = (type: string, value: string) => update(type,{content:{type:'text',text:value}});
const boot = {kind:'agent',name:'Claude',replays:true,needs_login:false,images:false,embeds:false,lists:false};
const user=(value:string)=>text('user_message_chunk',value);
const agent=(value:string)=>text('agent_message_chunk',value);
const turn={kind:'turn',stop:'EndTurn'};
const tool=(id:string,title:string,command:string,result:string)=>update('tool_call',{toolCallId:id,title,kind:'execute',status:'completed',rawInput:{command},content:[{type:'content',content:{type:'text',text:result}}]});
export const scenario = {
  sampleImage:false,
  sessionTitles:[...tasks.slice(0,4).map(task=>task.title),'留言',tasks[4].title],
  sessionPreviews:[...tasks.slice(0,4).map(task=>task.preview),'',tasks[4].preview],
  sessionCategories:[...tasks.slice(0,4).map(task=>task.category),'khor'],
  sessionWords:['idle','idle','idle','idle','idle'],
  deviceNames:machines.map(machine=>machine.name),
  devicesOnline:true,
  autoStream:false,
  terminalScreen,
  reply:"Let's try Khor, [download it here.](/download/)",
  replyDelayMs:300,
  replyChunkMs:35,
  thinking:[],
  samples:[
    [boot,
      user('客服说订单导出有些日期会少一天。我查了一个，3 月 1 日凌晨半点下的单，CSV 里变成了 2 月 28 日。帮我看下，日期按北京时间。'),
      agent('问题在导出函数：先转成 UTC，再截取日期，凌晨的订单就会落到前一天。我会让导出与订单列表统一使用北京时间。'),
      update('tool_call',{toolCallId:'export-read',title:'读取 src/export.ts',kind:'read',status:'completed',rawInput:{file_path:roots.web+'/src/export.ts'}}),
      update('tool_call',{toolCallId:'export-edit',title:'修复日期格式',kind:'edit',status:'completed',content:[{type:'diff',path:roots.web+'/src/export.ts',oldText:exportBefore,newText:exportAfter.replace('下单日期','下单时间')}]}),
      tool('export-tests','检查日期边界','npm test -- export','PASS src/export.test.ts\n7 date tests passed'),
      agent('已改为显式使用 Asia/Shanghai。凌晨、月底、跨年和闰日的 7 个日期用例都通过了。'),turn,
      user('好，列头也从「下单时间」改成「下单日期」，免得客户以为包含时分秒。'),
      update('tool_call',{toolCallId:'header-edit',title:'更新 CSV 列名',kind:'edit',status:'completed',content:[{type:'diff',path:roots.web+'/src/export.ts',oldText:"export const headers = ['订单号', '下单时间', '金额'];",newText:"export const headers = ['订单号', '下单日期', '金额'];"}]}),
      tool('export-final','运行导出测试','npm test -- export','PASS src/export.test.ts (8 tests)\nTest Files 1 passed (1)\nTests 8 passed (8)'),
      agent('改好了。日期口径和列名都已更新，8 项测试通过。\n\n代码和测试分别在 `src/export.ts`、`src/export.test.ts`，修复记录写进了 `README.md`。这 3 个文件还没有提交。'),
    ],
    [{...boot,name:'Cursor'},
      user('报销单超过 500 元要强制传发票。金额或附件变化时都检查，别到最后提交才弹窗。'),
      agent('我把规则放到统一校验函数里，输入金额、添加附件或删除附件时都会重新检查。'),
      update('tool_call',{toolCallId:'invoice-edit',title:'更新 src/validate.ts',kind:'edit',status:'completed',content:[{type:'diff',path:roots.expense+'/src/validate.ts',oldText:'export function validateExpense() {\n  return null;\n}\n',newText:"export function validateExpense(amount: number, files: File[]) {\n  if (amount > 500 && files.length === 0) {\n    return '请上传发票后再提交';\n  }\n  return null;\n}\n"}]}),
      agent('现在 500 元可以直接提交，500.01 元起需要发票。删除最后一个附件后会恢复提示。验收项已写进 `docs/验收清单.md`。'),turn,
      user('提示文案就用「请上传发票后再提交」，别提必填字段。'),
      agent('已统一成这句话，保留在附件区域下方。金额降回 500 元及以下时，提示会消失。'),
    ],
    [{...boot,name:'Codex'},
      user('支付平台会重试回调。检查下现在会不会重复入账，尤其是两个请求同时进来的时候。'),
      agent('原来的处理直接创建入账记录，存在重复写入的风险。我补了交易号的唯一约束，并改成按 transactionId 原子写入。'),
      tool('payment-check','检查重复与并发回调','npm test -- payment','PASS payment callbacks\nnormal payment: 1 record\nreplayed payment: 1 record\n8 concurrent callbacks: 1 record\ninvalid signature: rejected'),
      agent('重复通知会返回成功，但不会再次入账。8 个并发回调最终也只保留 1 条记录。签名校验失败仍返回 401，没有绕过原来的校验。\n\n改动和检查结果在 `src/payment.ts`、`prisma/schema.prisma` 和 `docs/回调检查.md`。'),
    ],
    [{...boot,name:'Khor'},
      user('帮我把刚才远航的周会记录整理一下：首批 3 个部门，反馈 18 条，处理了 14 条。下周二加 2 个部门试用。林晨发名单模板和培训邀请，陈蕾补权限说明，小周确认导出日期，赵明收剩下的问题。审批人和培训时间还没确认。'),
      agent('已整理到「远航项目周会.md」，并把你负责的事项放进「本周待办.md」。\n\n'+meeting.replace('# 远航项目周会纪要\n\n','')),turn,
      user('名单模板今天 16 点发，培训邀请等时间定了再发。两件没确认的事给我单独留着。'),
      agent('已更新：名单模板在今天 16:00 前发；培训邀请暂不发送。待确认的两项单独保留为「审批人是否沿用」和「周二上午还是下午培训」。'),
    ],
  ].map(frames=>[...frames,turn]),
};
