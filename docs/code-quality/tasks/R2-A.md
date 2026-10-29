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

## 独审修正与最小binding前置（root已授）

A1初稿只按leaf先Claude后Codex会串source；同id换home首render可复用旧available，均为固定前必须修。首次native3项为2过1失败，exit101/76.679s，实际是Codex fixture用了parser不接的response_item message；原日志observation-a1/node.log保留，不是产品行为失败。gen/catalog未运行未发布，Rust源unchanged。曾误用既有check-session-content.mjs名字，已逐HEAD原字节恢复，新检查独立check-session-observation.mjs（不删除/不追跑旧native门）。

root拒绝短命全局FACTS缓存：完整移除实现及every_session副作用，不在list全表poll放source判断。现获A1前置A2最小ObservationBinding，私有version/home/session/provider/完整native ID，存Khor自身元数据、原子发布；session_content语义为按打开解析/登记来源，不能仍称纯probe。Meta可信来源优先，无Meta既有binding优先，否则本次唯一local row/namespace解析；未知/歧义拒绝。page/raw只读Meta/binding，不再猜leaf/重扫全表。binding来源变更进入游标source身份，page reset、raw旧引用拒收。完整A2导入UI后续复用，不混同包。

必要小只读helper已授精确paths：crates/node/src/adaptor/claude.rs、crates/node/src/adaptor/codex.rs。只解析唯一完整原生ID，同ID多备份维持mtime规则、不同完整ID同clean_leaf拒绝；不复制分页/事件解析或写vendor文件。公共S1 Cursor必要source比较仍由H owner维护。跨home前端状态带id+machine query key，不能只等effect清理。

## 最新实现与验证现场

现源码已改为持久binding版，不再包含FACTS全局缓存或every_session list副作用。ObservationBinding version1/home/session/provider/full native ID，私有`.khor/observations/<id hash>.json`，元数据读取上限16KiB，复用files::write_whole的CAS/sync/atomic-rename发布；重复绑定不重写。打开解析可登记Khor自身元数据（不能继续称纯probe），纯page/raw只读Meta/binding，绝不为每一页调用全表发现。

Meta可信namespace优先；无Meta优先既有binding，否则仅本次打开从唯一local row实际category选namespace。两vendor helper拒不同完整ID同clean_leaf，同完整ID备份保mtime；绑定后按exact full ID读，避免另一个同prefix文件抢走原观察身份。统一source ID为version/home/row/provider/fullID，page来源改变reset，raw旧cursor拒收；必要只读身份桥修改adaptor/transcript_page.rs::Cursor::is_for_source。

前端ObservedSession的answer/fault都带id+machine queryKey，同id切换home首render不复用旧available；watch=false首次仍读tail，但已加载快照不再安排700ms timer。源状态查询不在全表poll挂接，失败GUI与外部TUI均按能力路由，没有UI vendor白名单。A1当前仅本机已存在native来源，A2导入/A3通用journal/A4 home网络源仍待后续，不冒全部ACP交付。

JS现场19项历史模块及7项真实ObservedSession函数回调检查通过（是实际函数提取/模拟hooks，不冒浏览器原生）。原160行check-session-content.mjs已恢复，新检查独立check-session-observation.mjs。6项native小夹具已写，含namespace/短leaf碰撞、exact绑定与mtime备份、unknown metadata、Node hosts=false跨重开binding与source改变page/reset和raw拒收。

root已重新授唯一native槽，exec37393，observation-a1-retry/running.json记实际PID/阶段；序列6tests→真实gen空staging→catalog临时stdout成功再原子发布。源manifest含所有新Rust文件与catalog，过程中不改这些输入。旧observation-a1/node.log exit101保留；本次结果与槽释放尚待现场确认，不得恢复时重复启动仍live进程。

## A1固定检查点631c0d0a

精确提交631c0d0a647bb0ed8f7b990ce40119dbcfb9b3cf，17文件，工作树clean，不带Lab。节点6项真实通过（68.323s构建/0.72s执行）；真实gen64.649s/catalog1.224s均exit0，source前后不变；19历史JS、7真实观察函数回调、全tsc、mobile-static[]均exit0/unchanged=true。产物快照observation-a1-retry/generated（含SessionContent/ContentUnavailable与inventory）、catalog.ts、generated-sha.json；root可按固定源/明确快照同步ignored输出。

随后独审指出raw拒收断言原EOF newer cursor本来就invalid range；已只改cfg(test)为有效record cursor先成功读，再换binding断言准确changed错误。root独立核生产prefix逐字节相同，授只跑此项而不重复gen/19+7/types；真实1项exit0/74.137s构建/0.12s执行、unchanged=true。observation-a1-record-cursor/source-proof.json保存前后全SHA与相同生产prefixSHA，test.log/result.json保留。新session_content.rs全SHA为3e27bcda21d811665e7617a512c4b9789938ed0a1a74c9f0c464f4815134783a，生产prefixSHA为4f7eceaa425a501a63c390362a88a9b5246ed360a24595f0c78928d1fb7f4218。旧6绿不冒覆盖修正断言。

所有native句柄已终态并向root释放唯一槽，无全App构建/用户会话操作。独审当前身份/namespace/queryKey主体无阻断，精确ref/指纹最终审待回传。root要求下一A2/3必要UI接线把gui.chat_readonly的“它正在自己的终端中运行”改中性“只读会话/历史”，因为following不能证明writer当前运行；不为此中断A1或重gen。另exact绑定helper可少做目录遍历为非阻断观察，当前只记录不追加优化专项。

A1仍不等于R005全部完成：A2观察导入与控制边界、A3全部托管ACP journal、A4 home路由继续按既定顺序。
