// One finite, browser-only workspace. Lists, conversations and file views
// refer to these same devices, paths and documents.
export const GiB=1024**3;
export const machines=[
  {name:'MacBook Pro',system:'macos',model:'MacBook Pro',kind:'macbook',cores:12,cpu:18,mem:[14.2,36],disk:[312,1024],gpu:9,uptime:302400,processes:[['Cursor',6.2,1.8],['node',34,1.1],['Google Chrome',11,2.4],['khor',1.2,.18]]},
  {name:'build-01',system:'linux',model:'Linux workstation',kind:'server',cores:16,cpu:24,mem:[21.8,64],disk:[487,2048],gpu:12,uptime:1252800,processes:[['node',130,2.6],['postgres',22,4.8],['redis-server',3,.3],['khor',.8,.12]]},
  {name:'Mac mini',system:'macos',model:'Mac mini',kind:'mac-mini',cores:8,cpu:8,mem:[6.4,16],disk:[154,512],gpu:4,uptime:172800,processes:[['Microsoft Word',5,1.2],['Google Chrome',9,1.8],['Finder',.6,.15],['khor',.5,.11]]},
  {name:'ThinkPad',system:'windows',model:'ThinkPad T14',kind:'unknown',cores:12,cpu:16,mem:[12.8,32],disk:[218,512],gpu:7,uptime:93600,processes:[['Code.exe',26,1.4],['node.exe',78,1.1],['Teams.exe',4,.9],['khor.exe',.8,.16]]},
  {name:'iPhone',system:'ios',model:'iPhone',kind:'unknown',cores:6,cpu:6,mem:[3.2,8],disk:[91,256],gpu:null,uptime:194400,processes:[['Khor',2,.13],['Safari',4,.42],['Mail',.4,.09]]},
  {name:'Pixel',system:'android',model:'Pixel',kind:'unknown',cores:8,cpu:9,mem:[4.1,12],disk:[78,256],gpu:null,uptime:216000,processes:[['Khor',3,.14],['Chrome',7,.56],['Calendar',.2,.08]]},
] as const;

export const roots={web:'/Users/lin/Projects/atlas-web',api:'/home/lin/atlas-api',office:'/Users/lin/Documents/Work',expense:'C:/Users/Lin/Projects/expense-portal'};
export const tasks=[
  {id:'wide/all',title:'Fix dates in order exports',category:'khor',machine:0,root:roots.web,preview:'Timezone and headings fixed; all 8 tests pass',minutes:3},
  {id:'gui/ai-elements-plan',title:'Validate expense attachments',category:'cursor',machine:3,root:roots.expense,preview:'Require an invoice for expenses over CNY 500',minutes:12},
  {id:'gui/ai-elements-stream',title:'Check payment idempotency',category:'codex',machine:1,root:roots.api,preview:'Record each transaction once, even with retries',minutes:28},
  {id:'gui/codex-review',title:'Summarize the Voyager weekly meeting',category:'claude',machine:2,root:roots.office,preview:'5 action items and 2 open questions',minutes:46},
  {id:'shell/layout-lab',title:'atlas-web',category:'shell',machine:0,root:roots.web,preview:'npm test -- export',minutes:65},
] as const;

export const exportBefore=`export function orderDate(value: string) {\n  return new Date(value).toISOString().slice(0, 10);\n}\n\nexport const headers = ['Order ID', 'Order time', 'Amount'];\n`;
export const exportAfter=`export function orderDate(value: string) {\n  return new Intl.DateTimeFormat('sv-SE', {\n    timeZone: 'Asia/Shanghai',\n    year: 'numeric', month: '2-digit', day: '2-digit',\n  }).format(new Date(value));\n}\n\nexport const headers = ['Order ID', 'Order date', 'Amount'];\n`;
export const projectReadme="# Atlas Web\n\nOrder export fix\n\n> Dates and headings updated. All 8 tests pass. Exports now use the same timezone as the order list. Ready for review.\n\n## What changed\n\nAn order placed at 00:30 on March 1 appeared as February 28 in the CSV. The formatter converted to UTC before extracting the date.\n\n| Item | Decision |\n| --- | --- |\n| Timezone | Explicitly use Asia/Shanghai |\n| CSV heading | Order time → Order date |\n| Scope | Export formatting only; original orders unchanged |\n\n## Sample export\n\n```csv\nOrder ID,Order date,Amount\nA1048,2026-03-01,129.00\n```\n\n## Verification\n\nAll 8 tests pass, covering midnight, month boundaries, year rollover, leap days, and headings.\n\n| Check | Expected | Result |\n| --- | --- | --- |\n| 00:30 local | Keep the local date | Pass |\n| 23:30 local | Keep the local date | Pass |\n| UTC conversion | Advance to the next day | Pass |\n| Year rollover | 2027-01-01 | Pass |\n| Leap day | 2024-02-29 | Pass |\n| Regular date | Unchanged | Pass |\n| Month boundary | Advance to the next month | Pass |\n| CSV heading | Order date | Pass |\n\n## Review checklist\n\n- [x] Fix dates and CSV headings\n- [x] Add export tests\n- [x] Document the change\n- [ ] Code review\n- [ ] Commit changes\n\nBranch: `fix/order-export-date`. Changes: `README.md`, `src/export.ts`, and `src/export.test.ts`.\n";
export const meeting="# Voyager weekly meeting\n\n## Progress\n\n- Three departments completed the first pilot and submitted 18 feedback items.\n- Fourteen are resolved. The remaining four concern exports and permissions.\n- Two more departments are scheduled to join next Tuesday.\n\n## Actions\n\n| Action | Owner | Due |\n| --- | --- | --- |\n| Send the second-pilot roster template | Lin | Thursday, 16:00 |\n| Complete permissions guide and FAQ | Chen | Friday, 12:00 |\n| Confirm order export timezone | Zhou | Friday, 15:00 |\n| Follow up on the four remaining issues | Zhao | Friday, 17:00 |\n| Send Tuesday’s training invite | Lin | Once the time is confirmed |\n\n## Open questions\n\n1. Keep the current approvers for the second pilot?\n2. Schedule training on Tuesday morning or afternoon?\n";

export type DemoFile={path:string;text:string;before?:string};
export const projects=[
  {machine:0,root:roots.web,name:'atlas-web',branch:'fix/order-export-date',subject:'Fix date boundaries in order exports',files:[
    {path:'README.md',text:projectReadme,before:'# Atlas Web\n\nOrder and customer management frontend.\n\n## Development\n\n```sh\nnpm install\nnpm run dev\nnpm test\n```\n'},
    {path:'package.json',text:JSON.stringify({name:'atlas-web',private:true,version:'0.8.0',scripts:{dev:'vite',test:'vitest run'},devDependencies:{typescript:'~5.9.0',vitest:'^3.2.0',vite:'^7.0.0'}},null,2)+'\n'},
    {path:'src/export.ts',text:exportAfter,before:exportBefore},
    {path:'src/export.test.ts',text:`import {expect, test} from 'vitest';\nimport {orderDate, headers} from './export';\n\ntest.each([\n  ['2026-03-01T00:30:00+08:00', '2026-03-01'],\n  ['2026-03-01T23:30:00+08:00', '2026-03-01'],\n  ['2026-02-28T16:00:00Z', '2026-03-01'],\n  ['2026-12-31T16:00:00Z', '2027-01-01'],\n  ['2024-02-28T16:00:00Z', '2024-02-29'],\n  ['2026-01-01T04:00:00Z', '2026-01-01'],\n  ['2026-06-30T16:00:00Z', '2026-07-01'],\n])('formats %s', (input, date) => {\n  expect(orderDate(input)).toBe(date);\n});\n\ntest('uses a date column heading', () => {\n  expect(headers[1]).toBe('Order date');\n});\n`,before:"import {expect, test} from 'vitest';\nimport {orderDate} from './export';\n\ntest('formats a date', () => {\n  expect(orderDate('2026-03-01T12:00:00+08:00')).toBe('2026-03-01');\n});\n"},
    {path:'docs/exports.md',text:'# Order exports\n\nExport dates use Asia/Shanghai, matching the order list. The CSV contains Order ID, Order date, and Amount. Amounts use two decimal places.\n'},
  ] as DemoFile[]},
  {machine:1,root:roots.api,name:'atlas-api',branch:'fix/payment-idempotency',subject:'Deduplicate payment callbacks by transaction ID',files:[
    {path:'README.md',text:'# Atlas API\n\nOrder, payment, and customer APIs.\n\nVerify the signature before applying idempotency by transactionId. Concurrent retries create one payment record; processed transactions return success.\n'},
    {path:'src/payment.ts',text:"export async function settlePayment(event, db) {\n  return db.transaction(async tx => {\n    await tx.payments.upsert({\n      where: {transactionId: event.transactionId},\n      create: event,\n      update: {},\n    });\n    return {ok: true};\n  });\n}\n",before:"export async function settlePayment(event, db) {\n  await db.payments.create({data: event});\n  return {ok: true};\n}\n"},
    {path:'prisma/schema.prisma',text:'model Payment {\n  id            String @id @default(cuid())\n  transactionId String @unique\n  amount        Int\n}\n',before:'model Payment {\n  id            String @id @default(cuid())\n  transactionId String\n  amount        Int\n}\n'},
    {path:'docs/callback-checks.md',text:'# Payment callback checks\n\n- A valid payment creates one record.\n- Retries return success without creating another record.\n- Eight concurrent callbacks create exactly one record.\n- Invalid signatures return 401; amount mismatches require manual review.\n'},
  ] as DemoFile[]},
  {machine:3,root:roots.expense,name:'expense-portal',branch:'feat/invoice-validation',subject:'Validate invoices before submitting large expenses',files:[
    {path:'README.md',text:'# Expense approval\n\nEmployees submit expenses for manager approval and finance review. An invoice is required for amounts over CNY 500; amounts at or below CNY 500 do not require one.\n'},
    {path:'src/validate.ts',text:"export function validateExpense(amount: number, files: File[]) {\n  if (amount > 500 && files.length === 0) {\n    return 'Upload an invoice before submitting';\n  }\n  return null;\n}\n",before:'export function validateExpense() {\n  return null;\n}\n'},
    {path:'docs/acceptance.md',text:'# Invoice validation\n\n- CNY 500, no attachment: submission allowed.\n- CNY 500.01, no attachment: request an invoice.\n- CNY 800, with an attachment: submission allowed.\n- Removing the last attachment restores the validation message.\n'},
  ] as DemoFile[]},
  {machine:2,root:roots.office,name:'Work',branch:null,subject:'Update the Voyager weekly notes',files:[
    {path:'Voyager-weekly.md',text:meeting},
    {path:"This-week.md",text:"# This week\n\n- [x] Summarize the Voyager weekly meeting\n- [x] Review pilot feedback: 18 items, 14 resolved\n- [ ] Send the roster template Thursday by 16:00\n- [ ] Review the permissions guide Friday by 12:00\n- [ ] Confirm Tuesday\u2019s training time before sending invites\n"},
    {path:"Pilot-plan.md",text:"# Voyager second pilot\n\nAdd two departments to the existing three. Collect rosters on Monday, run training on Tuesday, and review feedback on Friday. Keep the current environment and historical data.\n"},
  ] as DemoFile[]},
];

export const terminalScreen='\x1b[2J\x1b[Hlin@MacBook-Pro atlas-web % git status --short\r\n M README.md\r\n M src/export.ts\r\n M src/export.test.ts\r\n\r\nlin@MacBook-Pro atlas-web % npm test -- export\r\n\r\n> atlas-web@0.8.0 test\r\n> vitest run export\r\n\r\n RUN  v3.2.0 /Users/lin/Projects/atlas-web\r\n\r\n  PASS  src/export.test.ts (8 tests)\r\n\r\n Test Files  1 passed (1)\r\n      Tests  8 passed (8)\r\n   Duration  462ms\r\n\r\nlin@MacBook-Pro atlas-web % ';
