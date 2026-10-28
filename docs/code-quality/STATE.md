# 本轮代码治理：交付状态

更新：2026-10-28。**用户最新范围内的实现、Clean Code结构治理与既定业务验收已完成；main未合并、未发布。** 原失败测试及下一轮问题保留，不声称全仓逐行重构或全测试/平台全绿。交付见[DELIVERY](DELIVERY.md)，范围和用户决定见[PLAN](PLAN.md)。此前详细检查点保留在[历史](history/STATE-before-os-business-closeout-20261028.md)，不能按其中“当前运行/缺权限/0拖放”重新启动。

## 版本与运行状态

- 主仓库：`/Users/zgh/Desktop/working/khor`，`main@8704a503a27b36ad58f922690de50e766f8bd485`，tracked clean，既有发布服务不动。本轮无main合并或ship授权。
- 集成：`/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，`integrate/code-quality-wave1@ecab575e3f978f6570bff2c05a9542be93fd7342`，tracked clean。最后产品改动仅main webview特定fullscreen查询grant，用户已明确“纳入”。作者树overwrite-recovery同提交，原N1分支保留e110。
- 协调：本目录，`plan/code-quality`；最终文档提交及全部实际refs以恢复集`current-worktree-refs.json`为准。
- root唯一native槽已释放。所有本轮exec均terminal；没有受控App/Cargo/helper/observer/TMP残留。不再读取任何已退出CUA App对象，以免重唤。专用Finder窗口已关，用户原窗口/服务保留。

## 交付与验证

| 范围 | 已验证事实 | 限制 |
| --- | --- | --- |
| 首两项故障 | 键盘/输入框增高时最新消息跟随、读历史锚点；commit连续跳转及焦点交接。真实红→浏览器/真实Tauri iOS模拟器/Safari既定验收，相关源与最终提交映射已核 | 不冒物理iPhone/IME |
| 已启动模块治理 | 编辑原子发布、保存代次/digest/owner；身份和存储范围；网络/Node/Pair当前协议恢复；Pi/ACP/MCP输入取消、资源/退出归属；错误语义与共享纯规则已集成 | 模块范围与证据在AUDIT/tasks，不冒所有文件逐行重构 |
| Clean Code | 命名与角色、职责分离、单registry/constructor/RAII、共享规则、明确副作用/失败和资源owner；机械格式单独成批，保留必要协议/并发解释 | 不用注释/行数减少替代质量判断，ENGINEERING含来源与具体约定 |
| 组合与N1 | 原115父113pass/2fail保；原062 Busy/LocalSet反例当前修复后14父/22summary/19phase绿并独审 | 不回填旧115；原040下一轮 |
| Q033/Pi | 真实四图取消恢复→同稿重发Allow/EndTurn→小图取消、mode/option；实际嵌入Bun启动/空secret Cancel/原5s退出门/正常Quit/清理均通过独审 | 无provider登录/API key输入；旧harness失败保 |
| 生产借网 | 双私有peer实际原五阶段、native与新TCP、重开释放、正常Quit与清理通过独审 | 同机fixture仅既有KHOR_NAME初始区分；不冒真实SSH/master/remotelease |
| 最终OS业务 | 用户两次真实Finder拖放，chat_put_paths与term_drop各唯一成功/顺序与落地内容正确；fullscreen成功query false→true→false；全局WorkspacePane拒写→关闭阻止→Don't quit→fixture恢复权限→真实成功保存精确20B；实际CmdQ/GUI exit0 | 原whole脚本因持久session契约不匹配仍false，详下，不补写原green |

最终交付对照：`E/P5/final-delivery-current-audit02`。E=`/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`，I=`/Users/zgh/Library/Caches/khor-code-quality/recovery`；同名证据双盘。

## 最新真实OS验收及原失败分类

`P5/fullscreen-query-os-owned-run03`，exec98281 terminal exit1，215.248890583s，result **537bdd936401ba2a998abe05ae90575a631ccddf89cfe22a8566ffb771e5c5c0**，whole/manual false保持。原run/launcher/observer字节不变，plan03仅5个命名空间数据键变化；270/285/300720/双盘5GiB/512/来源/cleanup门都保留。

原`business-oracles.json`完整成立：chat_put_paths seq76/term_drop seq95各唯一success，不发送消息/终端不回车；native drop3观察是两个真实拖动的广播记录。两处landed SHA **b32de19c93bcdd6330e2f945f35224d941975f235a080f98fa0d2fff51deeccd**；full压缩[false,true,false]；write1–6拒绝/write7成功；Not saved/Don't quit真实操作及20B保存一致。App47945实际CUA CmdQ正常exit0，launcher_sent_app_signal:false；accepted在app-exited后6.142836875s写下，早于launcher错误。

原失败是launcher在读取accepted前等所有观测App后代消失。该Shell的host.json/meta.json把50774/50775明确映射为本次创建的持久host/child；`docs/SESSION.md`109寿命、host.rs1–5/628、Node::open_persistent明确允许发起窗口离开后host继续。原门强于产品持久契约；不能称产品退出泄漏，也不声称原日志记录了10s末完整remaining名单。按用户“新独立问题下一轮”，登记harness契约纠正，不改原结果/断言、不要求用户重复拖放。非作者独审`P5/fullscreen-query-os-owned-run03-review01/report.json` SHA **97a42fdc03ba9a0f9c651e05f9513844c484f30c22c93f7f452d94ee50a413f4**；原902entry两盘逐SHA零缺件/差异/extras。最终cleanup无errors/remaining/footprint，TMP不存在；source/helpers/dist/bundle稳定、资源门无失败。

run01人工ChatView成功与fullscreen已证，原270超时整组false，独审a8dc670a…aaa；run02ready前资源门停止，result27afcfc4…040b、独审bf10bd3c…6167，无业务操作。全部历史失败保。

## 当前构建来源

- full2389 source frame **32101973afa2bdb0d2969154a0b4819a0cc275c7d04f34e5a06a8832e1277438**。
- normal GUI `.../caches/rust/khor-quality-target/debug/khor-gui` SHA **39b300263fe72763a437eedea7d14017674744ac79a7f74c7ec0498f995458e0**/374444376B。GUI producer40922 resultd159564b…522f原exit1仅Q055环境来源覆盖缺口；实际normal build/artifact/inputs/cleanup成功及生成ACL非作者独审ea482c96…0b63完成。没有伪造完整GUI accepted provenance。
- Core `debug/deps/chat_upload_cancel-a98ef2c294b70f0c` SHAd84f1167…ba1bbc8/75861904B；新source producer13238 exit0/result5a7fbeef…315，完整receipt **6644ec805d3c75a24bf87ecc25994f7f819d7a3b7ab1f09b1319bd86c76e6d50**，root实际full require和非作者独审9d8fdabc…b559通过。相同bytes来自实际新producer，不冒旧source收据。
- 五实际来源`P5/fullscreen-query-real-app-actual-producers01.json`；runtime重新校验。CLI e0a4d3e6…f69b06未改；旧收据仅历史。

## 明确留下一轮

1. 原040取消deadline；native四case中的late-close/close-reopen两channel closed，唯一race未证明。
2. Q055 GUI自动环境输入覆盖TAURI_CONFIG/REMOVE_UNUSED_COMMANDS；不冒完整proof。
3. 会话内/机器文件入口未向全局退出保护报告草稿；本轮已验全局WorkspacePane，不外推全部入口。
4. OS一次性harness把刻意持久terminal后代当全部必须随GUI退出，需按产品生命周期分别验证与fixture收尾。原结果不回填。

这些独立项按用户最新收口规则登记，不追加本轮实现或重跑已绿矩阵；没有因它们宣称全仓无缺陷。后续是否合并/发布或启动下一轮，由用户决定。

## 存储、恢复与继续指南

旧归档索引在历史及各独立archive index；原字节/模式/mtime恢复前旧fullmanifest只代表归档前事实，inode历史。当前normalGUI/Core/CLI/source不退役，没有global clean。run01/run02两份375MB stage逐SHA/lsof/独审后复用`fullscreen-current-gui-stage-archive01/current-gui-39b.zip`(SHA103530b1…214f)可逆退役，index01/02记录。run03当前完整双盘尚保留。

旧task-owned内盘incremental（无当前Core收据引用、current incremental0）11229files/4100120197B逐entry校验后压缩留I/`incremental-retired-20261028.zip`，SHAd15a2d88ee7eab5ac6f0aa60647fab25aca6335785feea70bb48965af63071d0/1710737465B；原目录退役，inventory/stat/report双盘`P5/fullscreen-old-incremental-retirement01`。实际释放约2.4GB，未动用户缓存或外盘currenttarget。

上下文恢复先读本页/PLAN、当前用户决定及实际refs/diff；外盘缺失读I/RESUME.md与current-code-quality-docs-manifest.json核内盘快照，禁止按摘要覆盖外盘未提交内容。此轮已收口，勿按历史live句柄重新启动App/Cargo/重跑矩阵；需后续任务时按新授权登记范围、所有者和证据。
