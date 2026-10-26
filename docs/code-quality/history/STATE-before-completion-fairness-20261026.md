# 当前状态

更新：2026-10-26，继续完整治理；尚未完成全仓目标。

## 当前恢复检查点（优先于下方历史批次描述）

- 集成新增 bf559f4f/7493e8e9（ACP阶段1及独立test followup）、01646778/f21a840b（Q043 hear适配及runner），当前691bb90e（新增native-events全局门/CI）；未合并 main、未发布。精确集成后端已实跑：Node khor_agent 33过0失败6专用worker（父实际调用），once 10过，全 core 132过0失败1 benchmark忽略，source稳定；证据 integrated-c371-backend/results.json。
- run11 同源 c371 iOS模拟器App：Q015 notification nullable契约、ACP/Khor实际UIKit键盘与历史锚点、commit第6/8文件跳转、ACP长稿收键盘、Q035 native page句柄四组均通过。borrow lease与Q033 native上传取消全链仍待；物理iPhone/IME未验。详细来源/恢复App记录见 tasks/Q-010-native-run11.md。
- Safari独立原生键盘测试发现 Khor 长稿收键盘后的晚回跳 gap164。root已接管（native_acceptance额度中断）。真实组件Chrome/WebKit32对照中28绿/4红，唯一失败为长稿+尾部+父重绘但消息引用不变（有/无只读诊断probe均红）；不是扩大等待阈值消除故障。候选测量期间保留composer占位，32/32固定通过，同款实际Safari/UIKit 26指标与34附件通过，已提交54aa0f3f。补充发现旧版也有ResizeObserver交付周期警告，独立RO候选四组已过，默认32与既有34复验中。源码树 native-io，fix/quality-safari-compose-range，HEAD54aa0f3f，当前仅RO候选生产hook未提交。见 tasks/Q-010-safari.md。
- Q043 hear adapter 9fd2603f + runner d4757136，overwrite-recovery树clean冻结；root独立源审暂未发现生产阻断，全Chrome/WebKit22/22 source_stable=true。真实App候选JS/native双端资源验收已过：原c371 Rust质量App+d475 JS，真实host node:woken前后台触发，9listen/9unlisten确认，全部已关闭callback实际从Map释放，8owner先全收一次、关闭4后仅另4收到第二次，无unhandled；Q043/native-candidate-02/acceptance.json。其余native helpers仍待，不标全Q043完成。
- Q037 ACP阶段1 50f73f7f固定，host尚未接线。root复审测试追加真deadline独立触发与合作取消超过旧grace存活：真实Node acp_process 12/12、Control6/6通过。仅test followup待独立提交；gui_host test入口/host red/fixture三路径仍冻结红，不随阶段1集成。
- 唯一Cargo/native槽当前root；74006 ACP编译已退出；7495集成ACP/Control/Pi/acp lib四组串行编译运行。浏览器32/scroll34由root独立进程运行。旧native/Safari服务器已停，用户main进程不动。当前两盘都高于5GiB。
- Q041设计卡已保存；store_owner短暂容量错误后原模型恢复，复用旧Q033树从c371建fix/quality-install-integrity，正在独占install.sh纯函数提取a0与真实loopback夹具。未执行真实安装/镜像/SSH。



## 目标与边界

[PLAN.md](PLAN.md) P1–P5仍完整有效：全仓深入审查、功能修复、结构与工程治理、真实用户流程验收。用户最新指令为“继续”，不能以首批/单项通过结束目标。main、合并与发布本轮没有授权；只在独立分支集成和验收。治理在进行，旧目录整理“暂停”记录不再代表当前状态。

## 权威路径与实际版本

- 主仓 `/Users/zgh/Desktop/working/khor`，main `8704a503a27b36ad58f922690de50e766f8bd485`，最近核对tracked clean。上次ship的target/release/khor-gui含前端、可直接启动；SHA `9c3e078f355aa65e3f9d0137ae38548a1f2d75d21bf261528335e031ee6dbc01`。没有以它代替本轮新App验收。
- 本文件在 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-plan/docs/code-quality`，plan/code-quality；其他worktree同名文件为快照。
- 集成 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，integrate/code-quality-wave1，最新 `c37151a1ed90abc81ce7c119994912574e16163f`：Q033 GUI三提交、Q037 Pi生命周期与Windows叶、Windows CI门及Q040a弱锁回收均已集成；尚需当前整合头的完整后端/原生验收。
- Git对象在内盘主仓`.git`；未提交外盘源码另备份。外盘 `/Volumes/disk0/Khor` 是唯一开发资料分类根，不新增散落khor*目录，复用既有worktree。
- 恢复入口 `/Users/zgh/Library/Caches/khor-code-quality/recovery/RESUME.md`；当前文档缓存同目录current-code-quality-docs。目录迁移/内盘恢复细节见STORAGE/RECOVERY及历史STATE。

## 已集成批次与证据边界

| 任务 | 当前事实 |
| --- | --- |
| Q001A/B/C | 首轮GUI525文件/42重点、后端165文件/36关键路径盘点已做；仍非全仓深入审查。AUDIT保存后续范围。 |
| Q010/Q011/Q013 | 首批键盘/提交跳转/Lab已集成。Q010b 093d4115在真实Tauri iOS模拟器取得ACP长稿收键盘gap164→0、实际XCUI swipe触发隐藏通过；真实reader drag历史gap460不被拉回，commit第6/8真实触摸通过。最终确切集成App与物理iPhone仍待验。 |
| Q015 | 最小notification is-permission-granted cap已静态独立审过并集成15a4660d；真实重建App启动拒绝消失仍待验。 |
| Q020/b/c | 自动保存回环、取消、编辑线隔离与显式覆盖已集成；模块30、正式编辑器Chrome/WebKit26通过，门已登记。 |
| Q021/b | 已知/首次未知锁刷新失败保护集成至c3fe828b；hook20、独立App交互及精确当时双引擎全交互30通过。 |
| Q022 | 身份原子首建e89da2e6；真实Cargo9+专用worker/64子进程通过，未声称断电目录项或Windows实跑。 |
| Q023/b | dry-run/purge范围/指纹拒绝与严格binary owner回执集成至7251f2db；实际规则/CLI/80场景红绿及独立复验通过。 |
| Q024 | netlab失败/空跑、移动静态/编译分层、quality CI/平台Tab门已修；GitHub实际CI结果仍待。 |
| Q025/Q036 | FIFO真实写完成/控制屏障/独立connection shutdown/旧owner隔离，集成918127ea；原core123过2失败1bench忽略后，Q036 canonical缓存路径与陈旧scheduler测试已修46d5d0eb，作者117过0失败1bench（不含Q025新增8）。最终整合全core仍待。 |
| Q030/b/Q031 | 流式凭据裁剪、一次redact、live receipt与JSONRPC/REST恢复集成至3f73f8ec/908d60e4；真实单组/worker与独立审查过，不直接代替当前集成全组。 |
| Q032 | web输入预算/断开固定buffer集成0e667094；真实Web26、vendor5过，不声称慢连接deadline。 |
| Q033 backend | 21994830→dd15b9d1，3d0dc41f→3d10bbaf；token guard/取消receipt/SendCommitted，HTTP null仅admission。真实core15、实际host gated链4、NodeGUI3、terminal2、native参数2均过并独立审。旧host未commit上传只能退役attachment后显式重连；不谎称已commit取消。 |
| Q033 GUI | 3f2b8b18/8da0d063/7746378f→e56eef64/87674beb/bd58566a。owner/reader/token关联恢复原稿/附件，未知不自动重放；独立复审先抓4类过期retry真红、又抓旧cancel撤新notice真红，均独立复验绿。精确集成85a78fe9生成成功、Chrome/WebKit56/56、types/unit/gates101脚本94条、旧poll22/22通过且指纹匹配。门/CI85a78fe9；真实GUI/native链仍待。 |
| Q034 | 轮询失败退避、前台恢复、gone停止d5ff206c，门4822e64b；作者22/22与独立6场景过；最新Q033集成poll22也过。 |
| Q035 | 迟到page/borrow按最新owner释放、tab串行操作与8活页上限35e12364；selector与CI channel跟进37f8c54f/cc3a3295。module10、两引擎组件8通过，真正native资源句柄计数仍待。 |
| Q038 | 有预算event reader与固定stderr drain作者29fcad93→c5ab273d；std7/7、独立11/11、真实agent25+2专用worker、8MiB真实sidecar通过。不代表全RSS/取消期限已解决。 |
| Q037 Pi部分 | Windows作者b51a7d9c→873a6304；root7fa059ba→32e227f1。独立监督首Stop10s、串行stdin、可取消pipe、WNOWAIT归属anchor、确认wait再终态、Execution RAII。真实Configure/Prompt/Stop两种不合作sidecar及下一轮成功、重复Stop/后代/尾缓冲/panic/早停等最终agent33过0失败6专用workers（父实际执行），固定source；Windows公共层+叶+harness交叉check过，8组Windows runtime尚未运行，CI f186d340已提供。完整Q037仍有MCP/远端子任务同步阻塞与普通ACP取消未解决，不能标done。 |

旧完整整合检查点ea0e2c57的types/unit/lock20/mobile/static/std11/overwrite26/scroll34等保留在证据，不能当后续改动头已全通过。单次setup错误、缺browser cache、缺本地agent依赖和source变化运行均保留并不冒充业务绿；具体见单卡。

## 当前所有者与下一步

| 所有者 | 工作副本 / 当前任务 | 下一步 |
| --- | --- | --- |
| root | wave1 c371集成；native-io fix/quality-safari-compose-range；agent-protocol fix/quality-acp-lifecycle；overwrite-recovery fix/quality-native-event-ownership | 接管Safari/native QA与ACP followup，当前精确版本验证后分批集成；更新总账。 |
| native_acceptance | 执行额度中断，已停止；此前run11来源与日志完整保留 | root接管，不再假设其仍运行。 |
| store_owner | Q043作者 d475 clean冻结；Q041待确认复用路径 | 只准备Q041 a+b改动范围与本地fixture，不启动Cargo/native或真实外部写入。 |

Q039研究fcd分支保留 | Q037 ACP真实host收到cancel后15s仍不退出已真红；获准阶段1Node重定位共享进程+generation/永久期限分离+bounded async桥+ACP窄lifecycle接口，不先接host新行为。阶段1提交不能把尚红的test-only host路径带入集成；Cargo等run11释放。Q039生产协议仍未改。 |
| store_owner | Q033 GUI作者内盘store-owner-recovery HEAD7746378f clean；Q037额外测试已移交root | Q041构建/安装/镜像审计交付；当前只定a+b immutable release/checksum/纯fetch-verify-place契约与夹具方案，尚不改生产或执行install/ship/同步。 |

Q039研究fcd0ad29：真实25.4MB合法差集超过4MiB，重复/重启不收敛，反向小更新与epoch正常；研究红不登记生产绿。分片完整blob、能力确认、VV/flush/ACK、spool/并发预算方案在卡；chunk固定不代表全内存有界，生产实现待接口/资源决策。

Q040只回收互斥资源：全局弱表在同一std表锁下retain/upgrade/create，Tokio queued future/owned guard持强Arc。必须先验证不能同token双活锁；不清持久receipt，不用LRU破坏未知重试。

Q041确认代码条件缺陷：镜像第二次API失败可被空while吞掉仍写VERSION；两次latest/逐文件推送混代；install/join未消费checksum且镜像忽略明确版本；onset默认build可能漏Pi bundle。完整卡给出a安装完整性、b不可变release descriptor事务、c构建前置/manifest、d工具固定、e native来源、f结构维护；当前尚未修复，未声称生产已损坏。

## 真实App与资源

- 证据根 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`；Q010-native-20261025/ACCEPTANCE.md是实际App运行账，旧单卡前半暂停记录仅历史。模拟器FE04AC7C-B375-4F2E-AE7E-B6F8F5608947与独立bundle io.github.colehank.khor.quality，当前停止；不得覆盖用户App。
- 下一App须用当前同源静态lib+独立生成工程，保留原c3/lib/App；pure-origin23840 fixture、实际UIKit keyboard/XCUI touch与原生能力单独计，WebKit浏览器不冒充原生。物理iPhone/Chinese IME/硬件Escape未完成。
- 一个Cargo/native槽，当前root持有；不并发Cargo，jobs=2，quality-target复用。主发布khor-target不写不清。
- 外置盒风扇坏、USB SMART温度不可读；用户明确继续。第二次审计清16310个闭合单链接对象，保留25个archive/executable并核元数据；外盘7,373,307,904→11,322,572,800B（约10.5GiB），内盘约10.4GiB（每次启动前核对）。监测掉盘/连续3I/O错误/任一盘<5GiB只停本构建PGID。曾依manifest清旧single-link first-party rcgu约5.25GiB，16现存rlib/测试二进制保留并核inode/size/mtime。
- WebKit cache外盘khor-playwright已存在；系统Chrome默认；CI空channel用内盘既有Chromium验证过。无headless shell的外盘cache那次不计功能失败，未下载浏览器。

## 仍未完成与恢复

全仓剩余模块深审、普通ACP/同步大差集/once及相邻owner、工程与发布链重构、工具与制品来源、真正原生/物理设备/辅助技术及最终整合用户流程都未结束。完成标准不缩水。每次压缩后先当前用户指令→STATE/PLAN/任务卡→实际branch/HEAD/diff/进程→固定source证据；图谱旧索引/历史摘要不能代替现场。

本次重写前状态保留history/STATE-before-process-integration-20261025.md；更早目录/暂停历史见history索引及单任务卡。root维护此文件；任务作者只写自己的任务卡。
