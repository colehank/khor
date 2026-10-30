# 第二轮当前状态

更新2026-10-31。目标 **完成第二轮 active：29反馈＋5遗留＝34项**；尚在实现，不能以单个回归通过代替完整完成。本轮无main合并/ship授权，main仍 `e7c335c8dd81de091bd12b92cd704a63f25d03f3`。逐项标准读[验收总账](ROUND-2-ACCEPTANCE.md)，执行读PLAN/ROUND-2-EXECUTION/ENGINEERING及[STORAGE](STORAGE.md)。前态完整保在[上一检查点](history/STATE-before-df0d-checkpoint-20261031.md)，失败不回填。

## 存储与执行约束

用户指出两盘长期仅剩5GB，已完成定向清理：外盘24项旧构建cache实际释放104.7152GiB；系统盘248个inactive编译payload释放4.0037GiB、3195个逐SHA相同的旧证据副本改引用唯一外盘原件再增1.7753GiB。当前外盘约112GiB、系统约13GiB，内盘recovery约2.2GiB。完整清单及结果在 `/Volumes/disk0/Khor/maintenance/storage-20261030`；源码、会话、worktree、main/quality/catalog保护target和正式发布保留。根目录部分khor*是兼容链接，不能当多份数据相加。

后续每批先核 **外20GiB／内10GiB保留量＋显式预计峰值**，再独立核源码/运行准入；原helper5GiB只是紧急停止线。同一quality-target，jobs1/incremental0/offline。私有程序一批一份、结束核身份后淘汰，禁止重复gzip/zip和双盘大payload；完整日志/截图外盘唯一，内盘只当前源码恢复与紧凑索引。quality50GiB/recovery5GiB为复核线，不自动删用户数据。无需再扩大空间治理。

## 当前源码与所有权

Root `integrate/round2@f1bde7741d87be13f5543f808e71e81327a912b2` clean；main未动。两作者＋独立reviewer，Root只协调和有限集成，不开第三实现线。所有运行槽 **FREE**，不恢复旧poll；作者源码许可不等于运行许可。

- H：`khor-round2-history` / `fix/round2-chunk-grouping`，唯一TS历史/搜索/复制/锚点owner。来源starts_message、Core真实record和必要测试已独审整合；Node窗口已交还input。保既有分页、传输、缓存、取消预算，跨页query和whole-part copy仍须闭合；不能用拒绝所有截断复制代替目标。
- input：`khor-round2-input` / `fix/round2-recovery-contract@63ad0311`，唯一Node/ACP恢复owner。RunKey、reservation481、Load/Resume225、生产PID11b4、私有Attempt6fbb、只读proc_identity70c及两处Gap构造类型窄修均已Root整合。最新8项组件实际通过，不冒完整恢复；OS事实叶只host编译，尚未KernelProbe。继续已授InitialNew不可序列化seed authority、真实Attempt接线；公共Core/TS恢复入口与Shell后续交接尚未授写。不得全系统扫描或用Meta.exit/锁空冒provider退出。
- reviewer：非作者只读核源码、预算、并发、真实证据及Clean Code；idle用followup_task唤醒。不自行运行。

## 已确认的新结果

| 范围 | 实际结果与限制 |
| --- | --- |
| R1 GUI运行代次/原子快照 | Root4dc3cc7d，57370 terminal0/86.304s；原5exact各1passed/0failed/0ignored，真实Node libtest no-prior V通过。source/env稳、owned空、TMPabsent、单私有binary已退役。result `616c96d389a924e78d3c2c513b2f3b569f752d38158c7f363ad50008922c613f`。证明旧代拒写/row gate/完整快照，不冒完整恢复事务或App。前次36944 E0382编译false保留，b443借用修复后实际通过。 |
| 恢复事务组件8项 | Rootf1bde774，57551 terminal0/89.491s；8exact父各1passed/0failed/0ignored，两个原隔离Python helper也各1/0/0。fullmeta/compile及8case共10stage全0；实际Node libtest no-prior V，2333源前后同、env稳、owned空、TMPabsent、单私有程序退役无gzip，已独审。result `6739b540384f28b511dc4beece80093e5a909d97e7900429767f06db9edac0c4`。验证reservation同fd独占、Load新base/旧binding保留、失败不换旧base、未发布运行不写旧代、Attempt纯模型与实际Resume新Note；OS叶未probe、Attempt尚未生产接线，不冒完整S。 |
| 当前reader | Rootdf0d，原脚本28/28，exit0/0.211s；result `d0a2a8a6fff75e8fffad2573162e7e0aa1b40dd690cf3a0b7def07043e65c560`。 |
| 当前完整Find worker | Rootdf0d，原脚本30项全通过，exit0/2.946s；result `0d48dc4ba59eb2bc4146234d32eed5d1d761b7a2b1d4a5bafb18c75c21f933c9`。读取/giant/cancel/换源/context/copy组均实际；不是未实现的跨page完整功能。旧8f65 raw fixture失败保留，c478只补真实物理来源事实，未放宽断言。 |
| 原AI4消费者 | Rootdf0d，63835 terminal0/53.708s，desktop-light/dark、mobile-light/dark全部errors[]；原60断言、队列、点击/默认展开及4场景不变。result `f6c28d80b5dc059d0d7773c17fa072f035204cf1a3b29b7b794762925f9edbe0`，report `c5afceebec9f0f29ee64e8cce5aebb3fbced701dfc67e6cab3ddcad0349a1c75`。source/env/tools/gen稳、owned空、TMPabsent，已独审。真实Chromium受控transport，不冒WK/当前Rust生产者/系统Copy/真手机。 |

最新f1源增量bundle `4177059d190a42ddb2c2ed2f8a677812371595f00169428b958f06547f2a90c8`，18951B，prerequisite aa323，verify0并双盘，见 `evidence/round2/integration/recovery-history-f1bde774`；此前aa323的四片源bundle `2f099967331e5717ccb0497622374a34dc83fba5a297aabbadd1a80f83c74e1d`（24898B）亦已验证保。最新8组件紧凑内盘10文件约838KB，无binary/log镜像。

上述紧凑结果已内盘保，原始日志及截图仅外盘。增量源码bundle `e92fde7765f00cd517cc2289a4549b568bb17724fd9b033f68a02ff431a13c69`，15776B，prerequisite c4aa，真实verify0并双盘，位于 `evidence/round2/integration/r1-history-df0d3157`。证据来源不能跨版本冒用。

本次前两次native8编译false保留：75482在aa323为两Gap构造E0109/E0107，22391在dc469仅两E0109，均0test/0artifact/0V且完整清理。63ad改为带类型局部变量后57551实际编译及8项通过；parse不是编译证明。

AI此前13652/76618/73389/71253/57537各false原样保留。实际关闭了三处相关边界：合法SDK chunk误拆、manual disclosure停止canonical读取、pending Ask尾部误收起最新思考。读取意图readingLatest与几何following分别归同一scroll owner；最新canonical body按真实Turn/Gap边界定位。两次导航来自原脚本显式reload，不推断为HMR故障。

旧成功仍按各自范围复用：Store95235真实67,174,400B payload、有界启动；Node60432 13父/7辅助；ACP63174三exact；Recovery98901的13受控transport界面；Read62914完整。完整ACP74275仍false，已审附件/registry现有入口测试迁移待整合与实际全脚本，不称已绿。无关Store/A/B及旧H不因新TS变化重复跑。

## 当前关键路径与队列

1. input四个已审小片与OS事实叶已顺序整合，8组件实际通过；继续InitialNew seed/Attempt每phase真实接线逐片独审。明确恢复offer/action、当前epoch/idempotent token、Pending/Unknown不重启、完整Load新代/Resume实际Note、Shell新SID与旧记录关系仍须接完。RunKey五测试不代表这些已完成。
2. H已审scroll-compose/真实文字Range anchor＋迟到owner拒收、ACP测试迁移已整合（5ff/1f29/c571/923/237/8d），这些后继浏览器消费仍待实际。跨页query f16ab4bf有非重叠相位阻断，8a3只关闭有限left-prefix反例，整体仍未Root整合；为保原语义，未知头组需找到真实start后沿既有newer API到固定fence有界顺序重走一次，不提前发错误hit，不重复算coverage，无新RPC/完整组map。共享增量paint→query/whole-part流式copy/覆盖结算仍实现中。真实DOM Range多帧及ResizeObserver最终位置待测，卡片bottom不替代文字点。
3. Node/Core公开接口稳定后一次真正生成/catalog，再完整noEmit/Vite及受影响consumer。现179gen/278assets仍旧255→16ee lineage；新增HistoryEntry.starts_message/record尚未实际再生成，不能冒当前完整types绿。
4. C/Core受影响定向原生后，最后同源CLI/GUI生产构建。旧GUI48383 Cargo0但来源门拒hash旧资产的false保留；最终仅精确Tauri OUT核身份/SHA/lsof可逆retain，再真实cold。旧合同过期不直接重跑，禁止mtime/fakeprior/弱化V。无当前合格整包GUI。
5. 完整S实际同host正常Quit重挂、host死/重启/断线/显式恢复/新Shell；E真实Mac IME、路径拖选、全入口草稿、保存失败取消/最后一键正常OS Quit；长历史系统Save/Copy；W/C/G/Q/B桌面组合；手机真实viewport/连续候选/键行/滑删均尚需完成。私有fixture，不触用户真实会话，不真实模型/API。真人前重新确认方便，旧回答不算新可用性。
6. R2-001旧卡死、R2-016原网址连接拒绝尚未实证修复；不能被其它绿例替代。新独立问题只NEXT。34项对应证据＋非作者Clean Code＋必要平台验收齐备才完成；此前剩余粗估3–5工作日，真人等待另计，不再说仅剩验收。

## 压缩恢复

先本页/PLAN/STORAGE，再核实际Git、dirty及current-runtime.json；源码/证据不一致保现场，不按摘要覆盖。作者卡未提交内容分别保留，不全局stage。内盘 `/Users/zgh/Library/Caches/khor-code-quality/recovery` 只同步当前docs/refs/runtime、未合并源码bundle/patch和紧凑索引；历史mirror是当时事实，退役的binary/gzip不能再当存在。所有维护与先前运行均已terminal，不重开旧句柄。
