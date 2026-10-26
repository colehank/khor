<!-- latest user decision 2026-10-26: current protocol only -->
<!-- resumed checkpoint 2026-10-26 -->
# 当前治理状态

更新：2026-10-26。用户明确“继续”，已恢复完整治理；尚未完成目标。下方暂停检查点保留为历史，当前指令优先。完整范围/多智能体与压缩恢复规则仍以 [PLAN.md](PLAN.md) 为准。

## 执行边界与权威入口

- main `/Users/zgh/Desktop/working/khor` = `8704a503a27b36ad58f922690de50e766f8bd485`，本次现场tracked clean。本轮没有main合并/发布授权。已ship的`target/release/khor-gui`包含前端可直接启动，SHA9c3e078f…；不能用旧binary证明本轮实现。
- 权威文档仅本目录 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-plan/docs/code-quality`（plan/code-quality）；其他树同名副本为快照。当前集成 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，`integrate/code-quality-wave1`，HEAD **c719741c**（含Q039、Q041a1/b及ACP阶段2三提交；Windows fanout harness/CI小补丁未提交，精确集成运行待）。
- 外盘资料统一在`/Volumes/disk0/Khor`，复用已有工作副本，不新建散落khor目录。内盘Git对象保存所有已提交refs；未提交外盘源另快照，不能按摘要覆盖。外盘离线先读`/Users/zgh/Library/Caches/khor-code-quality/recovery/RESUME.md`及current-code-quality-docs。
- 旧完整批次账已移入history/STATE-before-completion-fairness-20261026.md；此前native/集成状态为历史，下面现场状态优先。

## 已集成与固定验证

| 范围 | 当前事实与证据边界 |
| --- | --- |
| Q010/11、Q015 | c371同源run11真正Tauri iOS模拟器已验证nullable notification query、ACP/Khor实际UIKit键盘与历史锚点、commit第6/8跳转、ACP长稿真实收键盘、Q035 native page句柄4组。tasks/Q-010-native-run11.md含来源/恢复App收据；物理iPhone/IME未验。 |
| Q010 Safari后续 | 真MobileSafari旧c371长稿晚gap164红。测高暂保留composer占位54aa→4678，RO交付周期写尺寸警告（旧版也有）单rAF调度25c→8ca，门d9。实际集成8ca前端的同款编译driver无重建通过26份指标：ACP/Khor长稿各6次settled gap0，reader键盘开合锚点保留。源码稳定，原始截图/指标在run11/safari-compose-observer-final。组件36/36、既有scroll34、types通过；浏览器不冒充真机。native_helpers已完成固定d9独立源审无阻断；尚未重建整合Rust App。 |
| Q020/b/c、Q021/b | 自动保存、冲突与锁刷新治理均已分批集成/独立审，相关模块和正式GUI双引擎通过；详见对应任务卡。 |
| Q022、Q023/b | 身份原子首建、purge范围与真实binary owner验证已集成，真实进程/CLI/scratch边界通过；不声称断电目录项或Windows运行。 |
| Q025/Q036、Q032、Q040 | FIFO写完成/连接owner、web输入预算、canonical cache与once弱锁回收已集成。精确c371后端：agent33/0fail/6专用worker（父实际执行）、once10、core132/0fail/1benchmark ignore，source稳定；integrated-c371-backend。不是所有慢连接/收据生命周期已审完。 |
| Q030/b/Q031/Q038 | 凭据流式裁剪、A2A context、事件读取预算与stderr drain已集成/单独审，证据在各卡。仍有同步MCP/远端取消阻塞未闭环。 |
| Q033/Q034/Q035 | 上传取消receipt与GUI owner恢复、轮询退避/gone、native page owner/LRU已集成。GUI56/56、poll22/22；native直接page句柄4/4。Q033真正GUI→Rust上传取消及Q035 borrow lease仍待，不用HTTP mock冒充。 |
| Q037 Pi与ACP阶段1 | Pi监督进程/Windows叶、RAII、operation代次与owned ACP transport已集成。ACP50f→bf559，test013→7493；精确691集成Node acp12、Control6、Pi29过5专用worker、ACP lib9均过/source稳定（integrated-acp-native-events）。共享process完整组与conversation后续仍应补入最终集成，不把这四组当全Q037完成。Windows8组仅交叉编译+CI配置，未实跑。 |
| Q043 hear | adapter9fd→016467、runnerd475→f21a、门691。非作者源审/独立双引擎22/22；实际原c371 Rust质量App+d475候选JS的真实node:woken两次前后台转换：关闭4/8后只剩4收新事件，9listen/9unlisten确认且9callback均从原Map释放，无unhandled。已进入集成，d9精确GUI回归正在收口；剩余helpers下一行单独处理。 |

已有P1批次不是全仓深入审查完成。覆盖清单见AUDIT，格式/大入口/构建治理与最终全流程仍在完整目标内。

## 当前所有者、未提交内容与下一步

| 所有者 | 当前树与唯一写入边界 | 工作与下一步 |
| --- | --- | --- |
| root | wave1 d20d7121 clean；native-io fix/quality-sync-chunked HEAD8d87af99（test2a11+生产8d87），link已提交clean；中央STATE/集成/恢复总账 | 正常库缺futures依赖已独立修d20，新增正常cargo check CI（配置未冒充远程运行）。真实Node25,448,146B devices gap使Seen0/Pinfalse目标真红；同源epoch2/1控制Seen123/Pintrue绿。测试dbfd5127已提交2a11。生产候选继续独立文档并保留首个mandatory错误，固定link SHA62ebbe6b（最后恢复legacy boards发现时机），真实Node2/2绿（node-isolation-green-01，48.43s/source稳定，oversize Seen123/Pintrue且仍Err，epoch控制仍Ok）；25MB大差集本身仍不能收敛，chunk方案未实现。 |
| fanout_owner | 外盘agent-protocol fix/quality-acp-lifecycle HEAD91c80317；独占gui_host、cancel fixture/tests、fanout及ACP fence文件 | 阶段2此前host10/socket10及fence4绿。新持续drip reader真实红：取消仍占busy；修后取消约2.015s、同PID下一轮通过，但随后Close→Gone5s超时，去诊断复现/sample和host返回证实共享Unix owner报EPERM，不能计整批绿。已授权额外Unix叶khor_agent_process_unix.rs和独立新回归；自有C探针核Darwin zombie-group语义，未凭猜测吞错误。已收回Cargo/native槽，root当前不编译。未提交/集成。 |
| root（接管store_owner） | 内盘recovery/khor-quality-store-owner-recovery fix/quality-install-integrity HEADc9f2962b；install/onset/onset-lib/join/mirror-sync/site deploy allowlist | store_owner反复服务容量错误，已明确冻结不自动重试，root接管。a0已集成50ad；a1严格SHA、不可变selection、curl/GNU wget截断header/raw block，旧loader又被非作者复合Shell关闭/引用逃逸真红击穿。现canonical install-components.sh唯一源+自包含嵌入逐字门+onset直接source，完整31/31绿（a1-canonical-green.log），join独占stage/receipt/同buffer摘要全8/8 Cargo绿且source稳定。a1待review卡后补SSH未确认清理显式诊断；b镜像不可变代+原子VERSION及site两阶段已实施，10mirror+5site真实脚本scratch回归绿，待冻结非作者审。不运行真实installer main/HOME/SSH/mirror/ship。 |
| native_helpers | Q043 overwrite树adb868e0 clean冻结；只读Q041-a1固定五文件 | Q043已集成并获38/38独立与原生7listen/7unlisten/child WK title证据。Q039 test2a11已交付，link初稿5d51f726独立源审无阻断，仅legacy boards发现提前低风险由root恢复原时机。当前独立审Q041-a1，发现并先红验证loader缺口已交回；最新31/8绿后核源未见新增安装阻断，提醒丢receipt/cleanup拒绝有残留可能；正写review卡和Q039tiny差异，不改源/不Cargo。 |
| native_acceptance | 执行额度中断，已停止 | 已有run11证据完整，root已接管。不要等待或假设其仍运行。 |

## 资源与验证纪律

- 唯一Cargo/native槽：fanout_owner阶段2无诊断drip/Close固定源复跑，释放后root Q039绿及Q041 join测试串行。jobs2，复用quality-target；主发布khor-target从未清理或写入。
- 外盘风扇坏、SMART温度不可读，用户已允许继续。两盘可用必须>=5GiB；低于线/掉盘/连续3I/O错误仅停自己的PGID。2026-10-26曾4.8GiB触线已暂停新编译；1280闭合single-link第一方rcgu按manifest清（保留6个archive/binary元数据不变），再清52轮停跑Vite重复prebundle cache实际释放约2.6GB。raw日志/source/截图/xcresult/App/Rust质量archives/binaries保留。后续又触4.28GiB：定位未统一CARGO_TARGET_DIR的wave1/target为21小时前遗留独立generator缓存，无open handles/compilers且全ignored，manifest后移除12,473文件，实际4,406,730,752→7,136,890,880B。不是主发布/quality-target/App/source；任何governance gen也须显式共用CARGO_TARGET_DIR避免再长一套。未断言所有空间增长都由此造成。凭据不进证据。
- 证据根 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`。已停止的quality App/Safari/23840/23841不得误作活动；模拟器FE04AC7C-B375-4F2E-AE7E-B6F8F5608947保留booted，独立bundle io.github.colehank.khor.quality；用户主App/常驻服务不动。
- root只在确切HEAD/dirty指纹下计绿。原始失败和setup错误保留，不放宽阈值；图谱旧索引/0 affected flow不代替现场。任何源码变化后按影响复验。

## 明确仍开放

Q037 ACP阶段2与同步MCP/远端取消；Q039合法25.4MB差集超过4MiB反复/重启不收敛（研究fcd；同轮失败隔离候选未运行绿，bulk尚未实现，不能只抬cap）；Q041不可变release/descriptor/checksum以及构建工具/制品来源；Q043 OS drag/windowclose/fullscreen值与最终同源App验收；Q033真实native上传全链、Q035 borrow lease、Windows runtime/物理iPhone/IME/辅助技术；其余模块深审、P4风格/职责/错误约定与P5精确整合全用户流程。无main合并/发布授权。

压缩后恢复顺序：用户指令→本STATE/PLAN/任务卡→实际worktree HEAD/diff/agent/进程→源指纹与真实证据。不能覆盖原盘未提交内容；各作者仅写自己卡，root维护总账与最终集成。


## 用户暂停检查点（历史，后已明确继续）

三位执行agent已由root中断，未授新构建；main仍未改。集成实际50ad20b7（含Q010/Q043及安装a0），无main合并/发布。ACP阶段2最新固定host10/10与capacity socket10/10已过，但未提交/集成，取消publisher最后error+HistoryEnd对极慢drip reader仍可久等的边界待审；停止状态保留当前10绿候选，不改源。Q039-A真实Node隔离测试已写，未编译/执行，不能标真红或已修；生产link.rs未改。Q041a1 curl/wget等26组曾过，新增截断HTTP header发现wget真实红，原始--debug raw block方案刚找到但未修完/冻结/提交；不得把26绿当完整a1。原生helpers真实7注册/7撤销与child WK title资源链过，OS drag/windowclose/fullscreen值未验。root旧manifest preflight即使已完成也未执行本轮删除，恢复后要重新核进程与文件指纹，不得直接用旧清单清理。


## 本次恢复

实际集成50ad20b7 clean；三位agent按原路径恢复写权。Q039-A test已核dbfd5127、link未改；root独立源审未见首轮运行阻断，将排首个真实Node red/epoch control。Q037阶段2暂停前host10/socket10源指纹一致，正在只准备极慢drip取消终止帧2s收口窄回归；Q041截断header --debug raw方案在中断前实际3组已绿，但完整a1尚未冻结。初始外盘5.2GiB，root重新生成storage-audit-20261026-5清单（未复用旧暂停清单），2862个闭合single-link第一方rcgu、保留13个archive/binary；确认执行回执后才授新编译槽。main不变、不合并发布。

## 最新恢复检查点（2026-10-26，优先于上方历史）

用户再次“继续”，完整目标活跃，未获得main合并/发布授权。main8704 clean；集成d20 clean。root实读STATE/PLAN与现场diff，Q041完整shell30/30已结束，无并行Cargo。最后核容量外盘约10GiB、内盘约8.9GiB，继续保留5GiB停止线；此前空间突变来源未证实，不将恢复可用量说成根因已解决。root接管Q041，旧store_owner不可恢复写入；native_helpers只读独立审。Q037无诊断Close复现仍为开放风险，不能由诊断轮偶然通过关闭任务。

root后续记录：Q039最终源62eb与testdbfd稳定，正常库集成测试真实2父/2专用worker通过，生产已提交8d87af99；最终小差异已获非作者复核，已cherry进wave1为0da/8f55，等精确集成运行。Q041 join8/8真实组件测试通过含并发两个PUSH_ARCHIVE_SH及cat失败清理；不等于真实SSH。Shell31/31已含非作者loader两变体×两入口的真红→绿；镜像10/10与site5/5测试调用实际脚本，仅网络/SSH/rsync/build在private scratch中适配，site旧--delete实际删除旧reader真红已保留，第一次mock未递归删除曾误绿已修fixture并重取真红，不冒充真实远端部署。a1已提交59da→集成e5f6，b已提交a723→集成de408；后续精确Rust/b运行待，不能由这些结果关闭全部Q041或全仓目标。

## 当前收口检查点（2026-10-26）

- 集成de408a06 clean；a1作者59da2774→e5f6b720，b作者a723bbba→de408a06。a1精确e5组件31/31绿/source稳定（Q041/integrated-e5-shell）；最终join d694小诊断源尚未重跑Rust，不借早期8绿。镜像13+site7最终20绿/source稳定（native_helpers Q041/b-final-review）；包括目录/目录symlink错误落位、公开755/644模式、FIFO损坏旧代cmp挂起，全部有独立原红与修后行为。b已保存/集成但精确集成20仍待。未运行真实installer main/SSH/镜像/站点部署。
- ACP最终共享Unix叶已用真实owned退出窗口回归先红→2绿，窄修只对Darwin EPERM等最多100ms至waitid可见且进程组严格只有自己unreaped leader；不吞未知EPERM。阶段2host24、shared12、NodeACP12、Pi29、ACPlib13、conversation19、stdpacket10与normal lib check均过，真实agent可选conversation2未跑。最后发现Closed+Ready public receiver尾部隐含假设，定向边界回归真红，guard仅允许已捕获remaining>0继续有限drain；正固定该最终源重跑host（预期25）与normal lib，仍fanout_owner独占Cargo/native槽，尚未提交阶段2。
- 重要构建恢复纪律：共享quality-target跨worktree已实际出现陈旧ACP artifact API不匹配编译前置失败（Unix首红轮不能算业务红）。fanout_owner touch授权ACP源、字节SHA不变后实际重编才得到Unix红。后续切worktree Cargo前须对选定第一方crate源显式更新mtime并记录字节指纹、核实际重编；最终集成尤其不能把顶层source稳定当所有依赖同源。保持同一quality-target/jobs2，不能另造target绕过盘资源约束；main发布target不动。
- 当前非作者a1审已保存Q-041-a1-review；b卡正在收尾，作者/test临时写权均已交还。root后续先固定最终集成Rust相关组和scripts正式入口，再继续Q039 bulk/legacy、Q037 MCP/远端取消、Q041 D04完整Pi构建/工具来源与P4/P5。仍无main合并/ship授权。

## 用户最新方向（2026-10-26，优先于所有旧compat计划）

原话：“不必考虑旧端兼容，我们还没有上线，按着纯净的开发就行”。按当前版本统一协议/接口/构建，
不再为旧Khor版本安排fallback或迁移设计。root已要求native_helpers停止legacy支线，新增legacy测试不提交/不集成；
刚结束的实际3/3仅留历史证据，完整源/差异/manifest/卡内盘备份后仅恢复作者自己的测试改动到dbfd。
当前数据epoch隔离、认证与平台/ACP真实能力协商继续按功能需要处理。main授权边界不变。

ACP最终3笔已进入wave1：8d916→07ba5f04、aa601→4da1804c、24b6478→c719741c；root已完成主要源审，
还需登记独立卡/精确集成运行。作者25host/12shared/12NodeACP/29Pi/13ACPlib/19conversation均过（2可选real agent未跑），
standalone10为重复harness不重复计。9文件SHA与正常lib check来源在Q037/acp-stage2-delivery.json。
root新加dependency-free Windows fanout harness与CI步骤，当前只是源码未提交/未验证，不声称Windows实跑。
唯一Cargo/native槽已归root，后续先完成精确集成相关回归及scripts正式入口，再按纯当前协议收敛Q039 bulk与Q041c完整构建。
