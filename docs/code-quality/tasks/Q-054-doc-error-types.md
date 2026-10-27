# Q054 Doc本地错误分类边界

状态：working；root，2026-10-27。N1 source11/crash14/trust15已检查点集成，root已承接作者树3路径，尚未native验证。此项属于原P4错误约定治理，非新增产品能力。

已确认来源：N1 `Error::from(&str)`默认为Invalid，三个 `BUSY.into()`使真实瞬时Busy被误成硬失败；candidate04已改为显式Error::busy，保持最小功能修复。另当前 `Error::from(String)`仍按完整英文文案 `document transfer state busy`识Busy，源于私有io gate返回String。这种内部语义依赖展示文字，未来改文案可能改变调度，是应清理的边界。

目标：私有本地gate/IO的Busy以类型贯穿，普通存储错误仍正确分类；删除以展示文案决定Busy的特殊分支。沿已有Doc Error/标准IO错误做小边界调整，不引入全仓错误框架、新crate或改公开协议/UI。外部未知诊断保留为诊断，用户语言在catalog边界处理；不能为统一风格吞掉错误、把Hard变Pending或反向。

先追实际io gate调用/typed kind传播与现有真实Busy用例，再实施；与N1作者协调同一文件写权，不能和当前flow修复并改。验证需真实持有gate得到typed Busy，正常释放后可继续；坏metadata/权限或IO仍显式硬错误，不能仅靠同一字符串断言。原N0/SeenPin/预算/角色/receipt不变量仍保，适用回归按影响范围。源码和运行证据齐才done。

## 当前实现候选

作者树 `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery`，基线 `fdfb6d0bdf31e5558c4f16c6e985227169019ab3`，root独占 `doc_transfer_state.rs`、`doc_transfer_state/io.rs`、`doc_transfer_state/tests.rs`。私有gate直接返回既有Doc Error，两个真实WouldBlock显式Busy，所有普通String保持Storage；非零gate字节显式Storage。未改journal/协议/预算/网络调度。新增两普通用例检查真实本地Mutex和文件锁阻塞/释放重开、真实坏metadata保留且Storage、普通诊断文字不影响分类。待独审、唯一槽实际构建与原Doc回归；当前不称done。

源码独审node_test_review未发现阻断；两gate调用方均核过，无其它合法String-Busy生产者，真实file测试使用独立open而非clone。root已提交作者检查点 `27093f4a4ff94afcdffa36aa4cd6143e11ecaaad`，仅3路径55+/12-，基线三路径与当前集成b196逐字相同；尚未cherry-pick，等待已占槽组合构建结束。未构建/测试，不能称功能通过。源码SHA：state `f09a0f713acc45cbd16ab14c9217f27e1314539ba59238bc1de65e31614f48d4`，io `219378afb228d514ada64f79bb840ee69b3bb315a8f33d9465f292f3d5c9e89e`，tests `2bde15bcfd8299460bced1906c7ca3c6490ef02eb0a80cb3317fb73d51a42ace`。

前置组合b196 build-only已实际通过（Q055/integration-current-build-run02，175.036s，CLI75.801s/Node84.476s）；root独核18dual、2378完整输入、实际fresh=false两产物及owned/TMP清。0业务父，不能据此说Q054已验证；Q054尚未集成编译。

作者27093f4a已exact3 cherry-pick为集成36e3918f，旧b196构建与actual inventory02（all1065/ignored58、0业务父、9dual/owned+TMP清）已收口后才修改集成树。最新同源CLI/Node两构建＋61 distinct回归已授node_transfer_owner执行；实际结果仍待，原两真实32MiB压力父不在本次范围。
