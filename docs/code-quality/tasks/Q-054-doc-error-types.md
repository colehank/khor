# Q054 Doc本地错误分类边界

状态：done（本片，非全仓）；root，2026-10-27。作者27093f4a→集成36e3918f，fixture修订ec404dd7；同源CLI/Node构建与61 distinct功能父实际通过，详见末尾原始证据。此项属于原P4错误约定治理，非新增产品能力。

已确认来源：N1 `Error::from(&str)`默认为Invalid，三个 `BUSY.into()`使真实瞬时Busy被误成硬失败；candidate04已改为显式Error::busy，保持最小功能修复。另当前 `Error::from(String)`仍按完整英文文案 `document transfer state busy`识Busy，源于私有io gate返回String。这种内部语义依赖展示文字，未来改文案可能改变调度，是应清理的边界。

目标：私有本地gate/IO的Busy以类型贯穿，普通存储错误仍正确分类；删除以展示文案决定Busy的特殊分支。沿已有Doc Error/标准IO错误做小边界调整，不引入全仓错误框架、新crate或改公开协议/UI。外部未知诊断保留为诊断，用户语言在catalog边界处理；不能为统一风格吞掉错误、把Hard变Pending或反向。

先追实际io gate调用/typed kind传播与现有真实Busy用例，再实施；与N1作者协调同一文件写权，不能和当前flow修复并改。验证需真实持有gate得到typed Busy，正常释放后可继续；坏metadata/权限或IO仍显式硬错误，不能仅靠同一字符串断言。原N0/SeenPin/预算/角色/receipt不变量仍保，适用回归按影响范围。源码和运行证据齐才done。

## 当前实现候选

作者树 `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery`，基线 `fdfb6d0bdf31e5558c4f16c6e985227169019ab3`，root独占 `doc_transfer_state.rs`、`doc_transfer_state/io.rs`、`doc_transfer_state/tests.rs`。私有gate直接返回既有Doc Error，两个真实WouldBlock显式Busy，所有普通String保持Storage；非零gate字节显式Storage。未改journal/协议/预算/网络调度。新增两普通用例检查真实本地Mutex和文件锁阻塞/释放重开、真实坏metadata保留且Storage、普通诊断文字不影响分类。待独审、唯一槽实际构建与原Doc回归；当前不称done。

源码独审node_test_review未发现阻断；两gate调用方均核过，无其它合法String-Busy生产者，真实file测试使用独立open而非clone。root已提交作者检查点 `27093f4a4ff94afcdffa36aa4cd6143e11ecaaad`，仅3路径55+/12-，基线三路径与当前集成b196逐字相同；尚未cherry-pick，等待已占槽组合构建结束。未构建/测试，不能称功能通过。源码SHA：state `f09a0f713acc45cbd16ab14c9217f27e1314539ba59238bc1de65e31614f48d4`，io `219378afb228d514ada64f79bb840ee69b3bb315a8f33d9465f292f3d5c9e89e`，tests `2bde15bcfd8299460bced1906c7ca3c6490ef02eb0a80cb3317fb73d51a42ace`。

前置组合b196 build-only已实际通过（Q055/integration-current-build-run02，175.036s，CLI75.801s/Node84.476s）；root独核18dual、2378完整输入、实际fresh=false两产物及owned/TMP清。0业务父，不能据此说Q054已验证；Q054尚未集成编译。

作者27093f4a已exact3 cherry-pick为集成36e3918f，旧b196构建与actual inventory02（all1065/ignored58、0业务父、9dual/owned+TMP清）已收口后才修改集成树。最新同源CLI/Node两构建＋61 distinct回归已授node_transfer_owner执行；实际结果仍待，原两真实32MiB压力父不在本次范围。

## 首次组合run03与fixture修正

最新36e3918f的CLI/Node两构建通过（79.90s/82.44s）；原61中前22 distinct实际过，第23坏source/receipt sibling父在peer.finish先报EAGAIN(errno35)，业务断言尚未执行，后38未跑。整run226.705s非绿，root独核43dual和前22 raw summary；2378源、helpers、dist稳定，252 observed PID/145 groups全退，无forced/guard错误/TMP清。原失败不回填成产品红或61绿。

root追实际Peer：listener为nonblocking，accept后只设置read/write timeout，未清nonblocking。`Q037/remote-r1/peer-socket-diagnostic01`在本机macOS通过真实libc.accept/fcntl验证accepted flags6/O_NONBLOCK4、未发送数据read立即errno35；这是平台机制诊断，不算业务父。官方[std Unix socket accept](https://doc.rust-lang.org/src/std/sys/net/connection/socket/unix.rs.html)的Apple分支仅accept/cloexec，未清该flag，支持此机制。集成树只在测试respond加set_nonblocking(false)及原因注释，原2s/1s和业务断言不变；生产0改。已派node_transfer_owner独审并同源重建/原61回归，先跑原失败父；运行结果仍待。R2作者树另有写权，不直接覆盖其WIP，后续集成须保留这两行。

## 组合run04实际通过

Q055/integration-combined-run04在36e3918f＋fixture两行的2378确切输入上，CLI/Node两构建及61 distinct父全部通过，333.683733s。61=R1原9＋tail21＋trust18＋Doc State6（含新2）＋IO3＋flow quiet/Busy/purepull3＋crash唯一业务父1；内层receiver/verifier不计业务数。未包含paired另target或真实release32MiB边界，不称全App验收。原失败父排首，真实业务/cleanup断言均通过，未宽化超时或改断言。

root逐SHA核81dual、全部61raw原summary规则及全部2378实际源，source-before=after=current；独立探269组均不存在、TMP /tmp/iwc4不存在，无forced/guard错误/剩余footprint。实际Busy6次body=charged=6,377,994B；purepull首Pending/目标缺失/重开目标完成；crash实际signal9、Importing重开/Equivalent/并发epoch/exactGC/new-id/lateWriteMissing原标志全部保留通过。CLI SHA45feefe212670037f3d6ea9a42e481830661633593ab8967c74a6f9892f95654，Node SHA4ff647a8e0c375dd9a29b5a65fb51da444565410154be413e75809c7ae0fbe95。源审与功能验收本片已闭环，待fixture检查点提交及总账更新；原run03非绿原件保留。

fixture exact2行已提交 `ec404dd71df62c15703c818f65fabb699be1877b`，集成clean；当前提交的2378输入逐字等于run04。Q054本片done，完整治理/P5与其它协议任务保持open。
