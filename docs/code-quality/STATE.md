# 当前治理状态

更新：2026-10-26。全仓目标继续执行，尚未完成。用户最新指令“继续”；完整范围/多智能体与压缩恢复规则仍以 [PLAN.md](PLAN.md) 为准。

## 执行边界与权威入口

- main `/Users/zgh/Desktop/working/khor` = `8704a503a27b36ad58f922690de50e766f8bd485`，本次现场tracked clean。本轮没有main合并/发布授权。已ship的`target/release/khor-gui`包含前端可直接启动，SHA9c3e078f…；不能用旧binary证明本轮实现。
- 权威文档仅本目录 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-plan/docs/code-quality`（plan/code-quality）；其他树同名副本为快照。当前集成 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，`integrate/code-quality-wave1`，HEAD **07c107f7及c9安装结构提取后续集成（以现场git为准）**。
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
| root | wave1 d9ef30de；native-io已保留fix/quality-safari-compose-range@25c，当前新fix/quality-sync-chunked HEAD9c61993e（d9+Q039研究fcd→9c），尚无生产修改；中央STATE与root任务卡 | 集成验证、独立源审、资源与恢复总账。Q010新回归已登记103脚本/96入口；等作者下一批明确冻结再集成。 |
| fanout_owner | 外盘agent-protocol fix/quality-acp-lifecycle HEAD013efc40；独占gui_host.rs、gui_host_cancel_tests.rs、fixture/acp-cancel.py、2个新Fanout文件及新ACP fence窄接口/测试 | root最初managed host候选真实不合作Stop1/1绿（10.21s，Gone立即ESRCH+重开下一轮）。独立审发现Control中Replay误退役、completion仍被notes/ops饿死，现接手修。真实RPC响应即ACK本Operation，显式forwarder barrier+有限notes fence再发UI终态；Abandoned不ACK。ACP实际forwarder barrier4/4与host首批7/7通过。新Fanout曾发现合法16,384小History快读者被128frame队列退役真红，已改实际capacity Waker+独立有限publisher，std10/10、完整host10/10（47.54s、2专用worker由父实际执行）固定源通过；partial取消明确ControlResult(cancelled)→HistoryEnd，close join publisher，SDK响应后live Note不再当history吞。尚未提交、待root非作者复审/兼容组/集成。此树尚无阶段2提交，不得把旧三路径视为仍冻结不可改；写权已交接。 |
| store_owner | 内盘recovery/khor-quality-store-owner-recovery，fix/quality-install-integrity HEADc9f2962b；install/onset/onset-lib/join/mirror-sync/site deploy allowlist及Q041卡 | a0纯函数提取c9已固定，root独立7/7与源审。a1缺/错/严格SHA等真红→shell13/13，join/resolver/onset仍写中未冻结未Cargo；loader函数间顶层命令拒绝缺口已要求补。只scratch/loopback，禁止真实HOME/install/SSH/mirror/ship。 |
| native_helpers | 内盘recovery/khor-quality-overwrite-recovery；保留d475旧分支，从d9建fix/quality-native-helper-ownership | Q043两笔b8ffbc44/adb868e0已交付clean，38/38+hear22/22+poll4+terminal/types、共享consumer真红→绿；root两次源审无阻断，已分别进入集成，root独立集成38/38 source稳定，global门07c107f7已登记104脚本/97入口。实际c371 Rust质量App+集成helpers JS原生7listen/7unlisten确认，Any/Window(main)/Webview(main)target准确、callback真实释放、child WK title实际交付，page_steps Some→null，无unhandled；OS drag/windowclose及fullscreen值查询仍未验。现转Q039-A：在root native-io只新增真实Node doc失败隔离test，root独占link.rs，待真红后修。 |
| native_acceptance | 执行额度中断，已停止 | 已有run11证据完整，root已接管。不要等待或假设其仍运行。 |

## 资源与验证纪律

- 唯一Cargo/native槽：root最终Safari已exit0且server停；当前优先授fanout_owner阶段2首个真实Cargo验证，启动前回报固定源码/命令/PGID。store_owner尚未申请。jobs2，复用quality-target；主发布khor-target从未清理或写入。
- 外盘风扇坏、SMART温度不可读，用户已允许继续。两盘可用必须>=5GiB；低于线/掉盘/连续3I/O错误仅停自己的PGID。2026-10-26曾4.8GiB触线已暂停新编译；1280闭合single-link第一方rcgu按manifest清（保留6个archive/binary元数据不变），再清52轮停跑Vite重复prebundle cache实际释放约2.6GB。raw日志/source/截图/xcresult/App/Rust质量archives/binaries保留。后续又触4.28GiB：定位未统一CARGO_TARGET_DIR的wave1/target为21小时前遗留独立generator缓存，无open handles/compilers且全ignored，manifest后移除12,473文件，实际4,406,730,752→7,136,890,880B。不是主发布/quality-target/App/source；任何governance gen也须显式共用CARGO_TARGET_DIR避免再长一套。未断言所有空间增长都由此造成。凭据不进证据。
- 证据根 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`。已停止的quality App/Safari/23840/23841不得误作活动；模拟器FE04AC7C-B375-4F2E-AE7E-B6F8F5608947保留booted，独立bundle io.github.colehank.khor.quality；用户主App/常驻服务不动。
- root只在确切HEAD/dirty指纹下计绿。原始失败和setup错误保留，不放宽阈值；图谱旧索引/0 affected flow不代替现场。任何源码变化后按影响复验。

## 明确仍开放

Q037 ACP阶段2与同步MCP/远端取消；Q039合法25.4MB差集超过4MiB反复/重启不收敛（研究fcd，生产未改，不能只抬cap）；Q041不可变release/descriptor/checksum以及构建工具/制品来源；Q043其余native helpers；Q033真实native上传全链、Q035 borrow lease、Windows runtime/物理iPhone/IME/辅助技术；其余模块深审、P4风格/职责/错误约定与P5精确整合全用户流程。无main合并/发布授权。

压缩后恢复顺序：用户指令→本STATE/PLAN/任务卡→实际worktree HEAD/diff/agent/进程→源指纹与真实证据。不能覆盖原盘未提交内容；各作者仅写自己卡，root维护总账与最终集成。
