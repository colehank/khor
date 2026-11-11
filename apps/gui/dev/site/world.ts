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

export const roots={web:'/Users/lin/Projects/atlas-web',api:'/home/lin/atlas-api',office:'/Users/lin/Documents/工作',expense:'C:/Users/Lin/Projects/expense-portal'};
export const tasks=[
  {id:'gui/ai-elements-review',title:'修复订单导出的日期偏移',category:'claude',machine:0,root:roots.web,preview:'已修复时区和列名，8 项测试通过',minutes:3},
  {id:'gui/ai-elements-plan',title:'报销表单增加附件校验',category:'cursor',machine:3,root:roots.expense,preview:'超过 500 元时要求上传发票',minutes:12},
  {id:'gui/ai-elements-stream',title:'支付回调的幂等性检查',category:'codex',machine:1,root:roots.api,preview:'同一笔交易重复回调只入账一次',minutes:28},
  {id:'gui/codex-review',title:'整理远航项目周会纪要',category:'khor',machine:2,root:roots.office,preview:'已整理 5 项待办和 2 个待确认问题',minutes:46},
  {id:'shell/layout-lab',title:'atlas-web',category:'shell',machine:0,root:roots.web,preview:'npm test -- export',minutes:65},
] as const;

export const exportBefore=`export function orderDate(value: string) {\n  return new Date(value).toISOString().slice(0, 10);\n}\n\nexport const headers = ['订单号', '下单时间', '金额'];\n`;
export const exportAfter=`export function orderDate(value: string) {\n  return new Intl.DateTimeFormat('sv-SE', {\n    timeZone: 'Asia/Shanghai',\n    year: 'numeric', month: '2-digit', day: '2-digit',\n  }).format(new Date(value));\n}\n\nexport const headers = ['订单号', '下单日期', '金额'];\n`;
export const projectReadme=`# Atlas Web\n\n订单导出修复记录\n\n> 导出日期与订单列表统一使用北京时间。改动已通过测试，等待代码复核。\n\n## 问题\n\n3 月 1 日 00:30 下单的订单，在 CSV 中被导出为 2 月 28 日。原函数先转为 UTC 再截取日期，导致凌晨的订单跨到前一天。\n\n| 项目 | 处理方式 |\n| --- | --- |\n| 日期口径 | 显式使用 Asia/Shanghai |\n| CSV 列名 | 下单时间 → 下单日期 |\n| 影响范围 | 订单导出，不改变原始订单数据 |\n\n## 导出前后\n\n\`\`\`csv\n订单号,下单日期,金额\nA1048,2026-03-01,129.00\n\`\`\`\n\n## 验证结果\n\n8 项测试全部通过，覆盖凌晨、月底、跨年、闰日与列名。\n\n| 检查项 | 期望 | 结果 |\n| --- | --- | --- |\n| 凌晨 00:30 | 保留本地日期 | 通过 |\n| 当天 23:30 | 保留本地日期 | 通过 |\n| UTC 转北京时间 | 正确进入次日 | 通过 |\n| 跨年 | 2027-01-01 | 通过 |\n| 闰日 | 2024-02-29 | 通过 |\n| 普通日期 | 不受影响 | 通过 |\n| 月底边界 | 正确进入次月 | 通过 |\n| CSV 列名 | 下单日期 | 通过 |\n\n## 交付清单\n\n- [x] 修复日期格式与 CSV 列名\n- [x] 补齐导出测试\n- [x] 更新本页修复记录\n- [ ] 代码复核\n- [ ] 提交到仓库\n\n当前分支：\`fix/order-export-date\`。修改包含 \`README.md\`、\`src/export.ts\` 和 \`src/export.test.ts\`。\n`;
export const meeting=`# 远航项目周会纪要\n\n## 本周进展\n\n- 首批 3 个部门已完成试用，收回 18 条反馈。\n- 已处理 14 条；剩余 4 条集中在导出和权限说明。\n- 客户希望下周二开始第二批试用，增加 2 个部门。\n\n## 待办\n\n| 事项 | 负责人 | 截止时间 |\n| --- | --- | --- |\n| 发送第二批试用名单模板 | 林晨 | 周四 16:00 |\n| 补齐权限说明和常见问题 | 陈蕾 | 周五 12:00 |\n| 确认订单导出的日期口径 | 小周 | 周五 15:00 |\n| 汇总剩余 4 条反馈的处理结果 | 赵明 | 周五 17:00 |\n| 发出下周二培训邀请 | 林晨 | 时间确认后 |\n\n## 待确认\n\n1. 第二批试用是否沿用现有审批人？\n2. 培训安排在周二上午还是下午？\n`;

export type DemoFile={path:string;text:string;before?:string};
export const projects=[
  {machine:0,root:roots.web,name:'atlas-web',branch:'fix/order-export-date',subject:'修复订单导出的日期边界',files:[
    {path:'README.md',text:projectReadme,before:'# Atlas Web\n\n订单与客户管理前端。\n\n## 开发\n\n```sh\nnpm install\nnpm run dev\nnpm test\n```\n'},
    {path:'package.json',text:JSON.stringify({name:'atlas-web',private:true,version:'0.8.0',scripts:{dev:'vite',test:'vitest run'},devDependencies:{typescript:'~5.9.0',vitest:'^3.2.0',vite:'^7.0.0'}},null,2)+'\n'},
    {path:'src/export.ts',text:exportAfter,before:exportBefore},
    {path:'src/export.test.ts',text:`import {expect, test} from 'vitest';\nimport {orderDate, headers} from './export';\n\ntest.each([\n  ['2026-03-01T00:30:00+08:00', '2026-03-01'],\n  ['2026-03-01T23:30:00+08:00', '2026-03-01'],\n  ['2026-02-28T16:00:00Z', '2026-03-01'],\n  ['2026-12-31T16:00:00Z', '2027-01-01'],\n  ['2024-02-28T16:00:00Z', '2024-02-29'],\n  ['2026-01-01T04:00:00Z', '2026-01-01'],\n  ['2026-06-30T16:00:00Z', '2026-07-01'],\n])('formats %s', (input, date) => {\n  expect(orderDate(input)).toBe(date);\n});\n\ntest('uses a date column heading', () => {\n  expect(headers[1]).toBe('下单日期');\n});\n`,before:"import {expect, test} from 'vitest';\nimport {orderDate} from './export';\n\ntest('formats a date', () => {\n  expect(orderDate('2026-03-01T12:00:00+08:00')).toBe('2026-03-01');\n});\n"},
    {path:'docs/导出说明.md',text:'# 订单导出\n\n导出日期以北京时间为准，与订单列表中的日期保持一致。CSV 包含订单号、下单日期和金额三列。金额保留两位小数。\n'},
  ] as DemoFile[]},
  {machine:1,root:roots.api,name:'atlas-api',branch:'fix/payment-idempotency',subject:'支付回调按交易号去重',files:[
    {path:'README.md',text:'# Atlas API\n\n订单、支付与客户接口。\n\n支付回调先校验签名，再以 transactionId 做幂等处理。并发重复通知仅产生一条入账记录，已处理交易直接返回成功。\n'},
    {path:'src/payment.ts',text:"export async function settlePayment(event, db) {\n  return db.transaction(async tx => {\n    await tx.payments.upsert({\n      where: {transactionId: event.transactionId},\n      create: event,\n      update: {},\n    });\n    return {ok: true};\n  });\n}\n",before:"export async function settlePayment(event, db) {\n  await db.payments.create({data: event});\n  return {ok: true};\n}\n"},
    {path:'prisma/schema.prisma',text:'model Payment {\n  id            String @id @default(cuid())\n  transactionId String @unique\n  amount        Int\n}\n',before:'model Payment {\n  id            String @id @default(cuid())\n  transactionId String\n  amount        Int\n}\n'},
    {path:'docs/回调检查.md',text:'# 支付回调检查\n\n- 正常支付成功入账。\n- 同一交易重复请求返回成功，不重复入账。\n- 8 个并发回调最终仅保留 1 条入账记录。\n- 签名错误返回 401，金额不一致进入人工核对。\n'},
  ] as DemoFile[]},
  {machine:3,root:roots.expense,name:'expense-portal',branch:'feat/invoice-validation',subject:'大额报销提交前校验附件',files:[
    {path:'README.md',text:'# 报销审批\n\n员工提交费用，直属主管审批，财务复核。单笔超过 500 元时必须上传发票，500 元及以下不强制要求。\n'},
    {path:'src/validate.ts',text:"export function validateExpense(amount: number, files: File[]) {\n  if (amount > 500 && files.length === 0) {\n    return '请上传发票后再提交';\n  }\n  return null;\n}\n",before:'export function validateExpense() {\n  return null;\n}\n'},
    {path:'docs/验收清单.md',text:'# 附件校验\n\n- 500 元，无附件，可以提交。\n- 500.01 元，无附件，提示上传发票。\n- 800 元，有附件，可以提交。\n- 删除最后一个附件后，恢复校验提示。\n'},
  ] as DemoFile[]},
  {machine:2,root:roots.office,name:'工作',branch:null,subject:'更新远航项目周会记录',files:[
    {path:'远航项目周会.md',text:meeting},
    {path:'本周待办.md',text:'# 本周待办\n\n- [x] 整理远航项目周会纪要\n- [x] 汇总第一批试用反馈：18 条，已处理 14 条\n- [ ] 周四 16:00 发送第二批试用名单模板\n- [ ] 周五 12:00 前校对权限说明\n- [ ] 确认下周二培训时间，再发出培训邀请\n'},
    {path:'客户方案.md',text:'# 远航科技第二批试用方案\n\n范围：在首批 3 个部门之外增加 2 个部门。\n\n安排：周一收齐名单，周二集中培训，周五回收反馈。第二批继续使用现有环境，不迁移历史数据。\n'},
  ] as DemoFile[]},
];

export const terminalScreen='\x1b[2J\x1b[Hlin@MacBook-Pro atlas-web % git status --short\r\n M README.md\r\n M src/export.ts\r\n M src/export.test.ts\r\n\r\nlin@MacBook-Pro atlas-web % npm test -- export\r\n\r\n> atlas-web@0.8.0 test\r\n> vitest run export\r\n\r\n RUN  v3.2.0 /Users/lin/Projects/atlas-web\r\n\r\n  PASS  src/export.test.ts (8 tests)\r\n\r\n Test Files  1 passed (1)\r\n      Tests  8 passed (8)\r\n   Duration  462ms\r\n\r\nlin@MacBook-Pro atlas-web % ';
