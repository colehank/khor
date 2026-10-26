# Q041-a1 独立源审

状态：review-complete-for-fixed-candidate；原loader阻断已真红→结构修复→门绿。当前摘要/版本/onset新边界暂未发现其他安装阻断；root已接收SSH回收状态诊断缺口，准备独立补充，不能先标全a1/整体SSH生命周期done。

## 范围与所有权

- 审查者 native_helpers，生产作者/接管者 root；原store_owner已停止写源。中央STATE较旧，以最新恢复指令、PLAN与现场为准，STATE未改。
- 树 `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-store-owner-recovery`，`fix/quality-install-integrity`，HEAD **c9f2962bba44128678771e26639430fbe4cb2f5b**。
- 初始冻结5文件：scripts/install.sh、check-release-install.py、onset.sh、onset-lib.sh、crates/node/src/join.rs。独立只读审后发现loader阻断，root临时仅交作者check-release-install.py新增一个窄回归；跑红后已明确交还并停止写测试。root随后新增canonical/generator并修改其余边界，故下文分两组源指纹，不把root授权修复当审查者越权写入。
- 新增 `install-components.sh` / `sync-install-components.py` 在root第二次冻结后纳入本次只读边界复审。b mirror/site及其新tests不在本卡范围，不借它们的门当a1证明。
- 未Cargo、未执行真实SSH、installer main、已下载binary、HOME/profile/.khor或服务操作。唯一运行是内存printf loader证明及获准的一个scratch loader窄回归；没有执行整个产品安装。
- 采用review-changes：minimal context→detect_changes/affected_flows/impact→tests_for。图谱built c371/HEAD不匹配，对loader/join覆盖缺失，0flow不是无风险证明；结论来自现场完整diff、实际helper/调用方/测试与root日志。

## 源指纹与冻结

接审初始SHA256：

| 文件 | SHA256 |
| --- | --- |
| scripts/install.sh | 63cd426a6fe783a3a17602c8749e19297d9736a3871aca388b671efeae294f52 |
| scripts/check-release-install.py | 328cddd27c5cce512917c1d9300049e8419f41083a0e17d6f87f4000221877ef |
| scripts/onset.sh | df8d5013990e1a69789d239d10787976d46f754bf82c5e9ba6ce26031330ea1a |
| scripts/onset-lib.sh | 07fece88234eddb128424f82d80e7a77a09623ba978a4d6e2931e134dec02b8f |
| crates/node/src/join.rs | c37731fd6fe985ff405bd454aa7d264b4f3524244cdea8352e760ce5d3759ba7 |

作者唯一授权test变更后SHA **3a26e88edb0014ed849409b6fdac3483a1cfb847bd061cdfd7d7d3de22f63933**；其余4文件仍同初始。窄红运行source-before/after逐项相同。

root结构修复后的最终受审SHA（两次读取相同）：

| 文件 | SHA256 |
| --- | --- |
| scripts/install.sh | d303a0c2dab399491104fb23cbd0c08926471b5d0c58881b1c20854234495b84 |
| scripts/install-components.sh | a28542ca40ecd3f3f638417d4bd6736556d8ce03e8fab823124b4520de40ebe5 |
| scripts/sync-install-components.py | f0269fdff21eb23903a7a1111b685948ce644e4c36fd894a66d7176492ff13bf |
| scripts/check-release-install.py | ee12502ea87b032e74b7fb9824759a1f48890e54374a8443c3c42330e74ccc07 |
| scripts/onset.sh | 8c0eba931d0b8e2ea53e7f806462d4432dc7d5b58096d6e60eee3f196db535ad |
| scripts/onset-lib.sh | 2d3c29467cb63048711f55eb36409c4f315051a3fb22028a9dd75909d55aa5c7 |
| crates/node/src/join.rs | c37731fd6fe985ff405bd454aa7d264b4f3524244cdea8352e760ce5d3759ba7 |

## P2确定阻断：旧声明loader可接受并执行顶层复合语句（已修）

旧Python loader寻找整行`)`作为helper关闭，旧onset AWK亦按同一行形状转state，没有真实Shell语义边界。把实际installer首个关闭行换成下面**合法Shell**形状，保持main/marker不动：

```
) ; printf LOADER_TOPLEVEL_RAN; khor_shadow() (
    :
)
```

Python loader接受、`sh -n`通过；只eval其返回的“declarations”即stdout=LOADER_TOPLEVEL_RAN。生产onset AWK经内存pipe亦exit0并返回相同helpers。没有执行main或访问HOME。这不是远程源码注入面的泛化结论，而是本地loader“只声明、拒绝顶层执行”明确隔离契约不成立。

另一个多行单引号变体把`)`和伪helper头置于字符串，再在缩进行里退出quote/函数、执行printf并定义另一helper，也被两loader接受；缩进或拒绝`) ;`正则不能替代Shell parser。

root临时授权作者只新增 `test_loaders_refuse_compound_closures_and_quoted_boundary_escape`：**1个实际unittest、2变体×2loader共4个确定失败，exit1，0.046秒，source_stable=true**。`Q041/a1-loader-compound-red/{run.log,result.json,source-before.json,source-after.json}`完整保留；root原30绿没有覆盖这两个变体，不能据此否认新红。

root修复采用可信canonical源，未继续堆伪Shell parser：

- `install-components.sh`为唯一helper源码，远程自包含install.sh嵌入其逐字内容；generator校验并可显式--write同步。
- Python仅在embedded与可信canonical一致后取得helper，变体拒绝且不执行。
- onset先extract+cmp一致，再直接source本地canonical，删除eval提取文本；不需远程额外下载运行依赖。
- 独立复审核对新边界/真实canonical纯声明，暂未发现该阻断残留。root实际 **31/31**（包括新增四变体）在 `Q041/a1-canonical-green.log`，本审查核看原始日志，不冒称作者重复跑31。

## 摘要、header与来源契约核对

- shell与Rust均只接受单条 `<64 lowercase hex><两个空格><精确压缩asset basename>.gz`，可无末尾LF；长度/空/多行/CRLF/额外basename或路径等不符均拒绝。Rust切片前有精确长度短路，未见短输入panic路径。
- SHA校验对象是压缩归档字节，校验成功才gzip解压、魔数、chmod和目的目录same-FS rename；失败保留旧binary/原preplaced输入。成功消费preplaced前用`-ef`保护路径alias，避免删除新binary。
- metadata用owned FIFO+dd收limit+1并观察producer status，避免仅看pipeline末端状态；header总budget64KiB、单行8192、最多20redirect、明确最终成功，拒重复Location/截断/多final/raw marker歧义。
- GitHub latest选择一次canonical同repo tag再取固定归档+sha，明确版本不读latest；mirror latest读取一次严格VERSION，固定releases/tag，旧flat无静默fallback。trusted header选择不接受跨repo/host/HTTP降级/危险编码。
- GNU wget raw block能力由help检测，状态8仅在真实redirect-limit notice存在时当受控redirect处理；未知能力拒绝，不把未验BusyBox当已支持。源码与root curl/GNU真实loopback矩阵一致。
- same-platform current_exe实际gzip buffer计算本次传输digest，明确为本地来源；cross-platform先核固定tag published SHA，再从同一buffer发远端digest。不是release签名/身份认证的替代说法。
- onset共用可信canonical resolver/verify流程，清除继承preplaced env后fetch到本地stage，再保留Linux ELF限定；不再手写第二套latest/sha-c规则。完整onset远端安装不是本次组件门覆盖范围。

## SSH暂存所有权与未闭环边界

正常路径暂未发现同名覆盖：PUSH_ARCHIVE_SH用umask077+mktemp独占regular stage，精确版本receipt单行绝对路径/受限basename，quote安全；installer失败只清自己的确切stage，旧UNDO通配删除所有download/push已删除。部分cat失败在发布前trap回收；root新增并发测试证明两个stage不同、删除一个不影响另一个。

但目前**不能宣称所有失败零残留**：

1. 远端printf发布后撤trap，而接收端回执丢失/无法解析会使push_asset直接Err，客户端未知确切path，当前没有可证明的精确回收协议。
2. 已知stage的installer失败分支原join.rs约635行完全丢弃cleanup run的结果；cleanup拒绝/SSH失败可能留下本人stage，却没有额外可见诊断。

root已明确接收边界：随后补transfer/receipt失败的回收不确定诊断；已知stage cleanup拒绝保留原安装错误并报告cleanup未确认，不以它覆盖首因。真正SSH传输超时、丢receipt回收协议另立 **Q045**，不扩大a1到整体SSH生命周期。本卡固定源尚不包含后续诊断补丁，需另核其差异。

现有8个Rust测试覆盖严格摘要、URL、quote、producer status/metadata cap、receipt parser、并发stage与cat失败；**没有**模拟lost receipt、cleanup拒绝、完整真实SSH事务。它们只能证明已运行范围，不是远端安装全链证明。

## 验证来源与剩余覆盖

- `Q041/a1-root-shell.log`原冻结30/30；新增loader红后由root `a1-canonical-green.log` **31/31**，原红均保留。
- root `Q041/a1-join-green-01/{cargo.log,result.json,source-before.json,source-after.json}`：**join8/8，exit0，source_stable=true，47.92秒**；source-before的install/canonical/test/onset/join SHA与本卡最终受审源相同。审查者未占Cargo槽。
- 未做真实installer main、真实SSH、真实HOME/服务、完整onset目标流程；BusyBox/non-GNU wget runtime未验。wget测试wrapper额外关闭config/HSTS/netrc/proxy，生产body download普通wget的配置副作用不在该组件隔离证明中；不把wrapper安全当全部生产配置矩阵。
- generator/embedding检查应随最终安装门注册；本审查未改全局package/CI。其text读取采用read_text规范化换行，严格“原始字节相同”表述还应保留LF源码前提；当前固定七文件均为该有效来源，没有据此发现当前安装行为错误。
- b mirror/site另有owner/独立回归，本卡未审其实现或借其绿作a1结论。

## 建议与最新交接

已修loader阻断可关闭；当前固定a1摘要/来源/新canonical边界可继续集成前收口。root按上述SSH失败不确定/cleanup诊断追加小补丁后，应对应复审/回归并在Q045保留真实协议缺口；没有将它们伪装“全失败已释放”。

作者临时test写权已交还，之后仅本卡写入；未改其他生产源，STATE未碰。最终七项SHA如上，root可在此边界继续。没有本任务Cargo/SSH/安装/服务进程；安全loader窄回归scratch由TemporaryDirectory回收。
