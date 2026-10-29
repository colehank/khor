# A 全部ACP的统一观看

状态working；用户“开始吧”授权完整下一轮。root已授A1唯一paths；main/ship未授。
作者round2_history；树khor-round2-history/fix/round2-history，A起点499dbe4f。H已验Lab23836源码保持冻结；生产必要修改的SHA/证据另记，不能回填旧验收。

只读调查/矩阵及40个known launch IDs（不是40安装或实测）见evidence/round2/history/observation-survey/R005-observation-survey.md与source.json。官方当前v1及锁schema1.12.0已核load是恢复/回放并连接MCP，list是metadata，未保证只读多observer；不一概声称load必然抢占。

## A1已授最小纵向切片

新增Node session_content.rs统一已有native source选择；按打开调用typed session_content(id,machine?)，不往SessionRow全表poll放vendor扫描。能力仅Available{following}/Unavailable{reason}，不把readable等同于可交互/可恢复。Node probe和page/record共用source resolver；UI只有能力分支，没有Claude/Codex名单。

允许写：node新session_content.rs及lib内容能力/source选择；core lib/chat/api能力桥、真实gen_bindings producer；Tauri lib async command/registry；DetailPane/ChatView观察接线、catalog必要原因词条、定向测试。必要API.ts真实绑定消费属原H owner。未知原因/远端未接线显式提示，不把Unavailable当A全部完成。native当前空闲，测试/生成先向root领唯一槽。

A1只接已存在native来源与明确Unavailable；A2观察import/binding（不resume），A3全部托管ACP唯一canonical journal及replay事务去重，A4执行home只读history路由仍待后续授paths/实施。数据owner/持久身份和共同读管线见调查文档；不按vendor复制流水线，不顺势开S大范围。

## 实施契约

- source probe只在打开该观察页/显式重试时执行；不spawn、不登录、不session/load/resume、不写供应商文件。
- 本机现有Claude/Codex source由同一resolver解析真实文件；找不到来源与文件不可读分开。远端来源当前显式not-connected，A4接线后再更新能力。
- still ChatView只负责显示/取消自己的读取；观察页不启动live attachment或开放控制。结束GUI也按能力展示存盘内容，不按vendor判断。
- 新types由真实producer生成，不手改TS；read/view支持不能推断恢复或writer归属。
- 最小验证：既有两native夹具及非vendor未知来源；无全表probe；能力响应/前端路由、失败/离页迟到结果与retry；无spawn/load/control。

## A3实施前已确认的必要契约（独审补充，不扩A1）

不能只记录Event::Note：ACP live不保证回显user_message_chunk；gui_host::Say/SayWith直接prompt_with，统一journal owner还须记录已接纳prompt的文字/图片/文件与turn完成/取消/失败/结果不明边界。echo按可靠turn/message关联，不用文本hash吞掉合法重复chunk。

start_resume初始session/load的Note在Event::Ready前排队，尚不在GuiOp::Replay状态。startup load与显式Replay均须独立snapshot事务/generation，Ready/fence后提交、失败保旧；不能只以replay.is_some分流，避免每次重开重复append历史。A3定向用例含两次resume/bootstrap、重复显式replay、合法相同live chunk。schema未提供通用去重ID保证。
