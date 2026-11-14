import {exportAfter,exportBefore,machines,meeting,roots,tasks,terminalScreen} from './world';

const update = (sessionUpdate: string, values: Record<string, unknown>) => ({kind:'note',update:{update:{sessionUpdate,...values}}});
const text = (type: string, value: string) => update(type,{content:{type:'text',text:value}});
const boot = {kind:'agent',name:'Khor',replays:true,needs_login:false,images:false,embeds:false,lists:false};
const user=(value:string)=>text('user_message_chunk',value);
const agent=(value:string)=>text('agent_message_chunk',value);
const turn={kind:'turn',stop:'EndTurn'};
const tool=(id:string,title:string,command:string,result:string)=>update('tool_call',{toolCallId:id,title,kind:'execute',status:'completed',rawInput:{command},content:[{type:'content',content:{type:'text',text:result}}]});
export const scenario = {
  sampleImage:false,
  sessionTitles:[...tasks.slice(0,4).map(task=>task.title),'Notes',tasks[4].title],
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
      user('Orders placed at 00:30 on March 1 show February 28 in the CSV. Use Beijing time, and rename “Order time” to “Order date”.'),
      agent('The export truncates a UTC timestamp. I’ll use Asia/Shanghai and update the CSV heading and tests.'),
      update('tool_call',{toolCallId:'export-read',title:'Read src/export.ts',kind:'read',status:'completed',rawInput:{file_path:roots.web+'/src/export.ts'}}),
      update('tool_call',{toolCallId:'export-edit',title:'Fix dates and CSV headings',kind:'edit',status:'completed',content:[{type:'diff',path:roots.web+'/src/export.ts',oldText:exportBefore,newText:exportAfter}]}),
      tool('export-final','Run export tests','npm test -- export','PASS src/export.test.ts (8 tests)\nTest Files 1 passed (1)\nTests 8 passed (8)'),
      agent('**Dates and headings updated. All 8 tests pass.**\n\nThe fix and a sample export are in `README.md` on the right. Three files changed: code, tests, and documentation. Nothing committed yet.'),
    ],
    [{...boot,name:'Cursor'},
      user('Require an invoice for expenses over CNY 500. Validate as the amount or attachments change, rather than waiting for submission.'),
      agent('I’ll use one validation function and run it whenever the amount or attachments change.'),
      update('tool_call',{toolCallId:'invoice-edit',title:'Update src/validate.ts',kind:'edit',status:'completed',content:[{type:'diff',path:roots.expense+'/src/validate.ts',oldText:'export function validateExpense() {\n  return null;\n}\n',newText:"export function validateExpense(amount: number, files: File[]) {\n  if (amount > 500 && files.length === 0) {\n    return 'Upload an invoice before submitting';\n  }\n  return null;\n}\n"}]}),
      agent('CNY 500 can be submitted without an invoice; CNY 500.01 requires one. Removing the last attachment restores the message. Checks are in `docs/acceptance.md`.'),turn,
      user('Use “Upload an invoice before submitting” for the message. No need to mention required fields.'),
      agent('Updated below the attachment field. The message disappears when the amount drops to CNY 500 or less.'),
    ],
    [{...boot,name:'Codex'},
      user('The payment provider retries callbacks. Check whether we record payments twice, especially when two requests arrive together.'),
      agent('The handler created a record on every request. I added a unique transaction ID constraint and an atomic upsert by transactionId.'),
      tool('payment-check','Check repeated and concurrent callbacks','npm test -- payment','PASS payment callbacks\nnormal payment: 1 record\nreplayed payment: 1 record\n8 concurrent callbacks: 1 record\ninvalid signature: rejected'),
      agent('Retries return success without recording another payment. Eight concurrent callbacks also produce one record. Invalid signatures still return 401.\n\nChanges and checks are in `src/payment.ts`, `prisma/schema.prisma`, and `docs/callback-checks.md`.'),
    ],
    [{...boot,name:'Claude'},
      user('Summarize the Voyager meeting: three departments in the first pilot, 18 feedback items, 14 resolved. Two more departments join next Tuesday. Lin handles the roster and training invite; Chen documents permissions; Zhou checks export dates; Zhao follows up on the remaining issues. Approvers and training time are still open.'),
      agent('Saved to `Voyager-weekly.md`, with your actions in `This-week.md`.\n\n'+meeting.replace('# Voyager weekly meeting\n\n','')),turn,
      user('Send the roster template today by 16:00. Hold the training invite until the time is confirmed. Keep both open questions separate.'),
      agent('Updated: roster template due today at 16:00; training invite on hold. The open questions are whether to keep the current approvers and whether training is Tuesday morning or afternoon.'),
    ],
  ].map(frames=>[...frames,turn]),
};
