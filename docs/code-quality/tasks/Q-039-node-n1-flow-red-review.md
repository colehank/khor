# Q039 N1 F1 red候选05：独立源码审

Reviewer：node_test_review，非作者，2026-10-27。仅只读source/证据，未Cargo、测试、进程操作或改源；只写本卡。Q052候选02/plan04维持冻结候槽。本卡不冒称真实业务红或编译绿。

**当前授权的逐exact独立进程四父验证未见确定源码阻断，可供root审runner后授权尝试。quiet应为绿控制，其余三个必须各自越过指定setup/control/cleanup门后命中自己的目标断言才算独立产品红。30秒内未到真实预算窗口属于准备失败，不能减量/种budget或算红。** 有一项执行约束：四个普通父需各自exact、test-threads=1隔离运行；默认全lib并发尚不适用，见末段。

## 来源、范围和生产等价

先读中央STATE/PLAN与validation-plan/production-equivalence；CRG先行明示main@8704a503，flow_tests符号not_found，不能表示无影响/已覆盖。树无.codegraph，实际新源为准。

树 `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery`，HEAD `5f2d5cf77ad05bd7e8ce2e11d0b90c36485c7ef3`。`Q039/node-transfer-n1-flow-red-candidate-05` 与internal recovery同名28文件逐SHA一致；14源与完整1084输入逐SHA=current。manifest `2a61eca866440a4f88536b296355b6844cf2d5d6bb135b2e9a3bcc7c44f5d429`；fullpatch `dc0ac8d9f5bf23d293d72026051d95dfe433c34e87da6fa364f57cfd7a0d663d`；04delta `70be24d4ae21434d036e2d8297e65ff3bedafb549320f121da0e3ff618b83fbe`。新flow源 `1bc7551cdc995599f9c9902ce960826a63a6a7da1cc9cb36bdf57602e6f952e0`。

完整读1154行新flow及五路径delta。仅新测试模块/lib self alias/State真实gate窄helper、wire frame tag、step/receipt barrier/quantum observer。四份normal-inverse04与04原full source独立逐字核等；另逐段读实际delta，正常编译分支仍一次dispatch_inner/一次transfer_step await+同Ok-only计量，response同encode/write/finish。新观察代码只cfg(test)，没有产物环境故障开关、伪造result/body计数、跳过admit、复制blob协议或提高预算。04三typed Busy/原已确认计量和部分Exchange缺陷刻意保留，正常源码等价结论是静态判断，非normal编译证据。旧N0、Pair04及旧11unit源未变。

## 观察与真实网络归因

Fixture使用private canonical scratch双home+existing util::hermetic relay，A为公开one-shot Node，B是真nonhosting resumable resident；公开pair和最多6次真实小doc sync先settle11Doc epoch/VV与双方pending receipts，scope之后才测。A endpoint.json必须不存在且B保存的A roads为空，防旧A resident替测试完成工作。

FrameTag在实际接受stream处跨await显式传递；真实question/共享admit成功之后admitted记录请求，reply只在实际send.write_all成功后记。内部readings dispatch明确None，不伪装wire。peer精确过滤，只合A foreground Quantum与B接受A的真实帧；B自己的泵事件不充本visit。doc/body manifest+offset/长度入账，不存payload。Quiet的early opening同样经过真实FrameTag，不只数普通exchange_on。

Probe根≤4，events/root≤2048、manifest/root≤64、active/waiting barrier≤8，BY90秒。overflow/invalid manifest/barrier timeout都置issue并由healthy在最终target前失败，不静默丢事件后算红。Ticket Drop释放、Scope Drop退注册并release所有pause，规则/锁不跨网络await。私有gate/Sink都在实际After checkpoint确认Busy后drop，先解除故障再让pump收尾。

## 四个父分别判定

1. `a_stable_public_sync_uses_one_actual_versions_frame`：真实settle后public sync，B收到的实际请求精确 `[Versions]`、moved=0，A/B11Doc fresh epoch/VV前后相同。未调用内部dispatch假数。此父是绿控制；失败不能借其它三个红解释。原日志须有实际pair/settle/cleanup成功。
2. `bidirectional_wire_bytes_remain_charged_when_a_real_sink_is_busy`：两边真实Device地址历史+AgentSpec合法历史预备fixed manifests，Device/Agent两个方向均>2canonical chunks。首Exchange已真实创建B Receiving/stage后，在A下一次Write之前held实际B core Sink owner，After必须typed Busy，然后立即drop owner。实际失败请求payload仍由B的admitted帧计入actual，而04仅Ok加charged；独立Agent反向full chunk、Seen123/Pintrue且Device fixed target未达必须先成立。每个实际frame请求/成功write的reply各1、双向同quantum≤4chunks/1MiB与轮转次序也必须先过。清理后目标`charged==actual`失败才是本父独立计量红；IO/owner未拿到/收不到Busy/独立小doc未推进/trace不全均为setup失败。实际body≤32MiB是额外断言，不把其它父结果代替它。
3. `a_real_last_single_chunk_budget_defers_a_bidirectional_exchange`：两方向Device/Agent>8MiB，wide/remaining两方向均>1chunk固定真实目标。针对wide每次初Exchange先持真实本地metadata gate制造pre-IO Busy，直到B实收/实写body量实际满足`CHUNK<=32MiB-used<2CHUNK`；near时让Device/Agent的下一步真实gate Busy并释放，将wide的真实双向Exchange放行。没有修改moved、constants/30秒或返回synthetic step。必须真的到窗口、发生真实Busy、SeenPin持久、wire完整/量子门过；清理后只有`actual<=VISIT_BYTES && !target_started`目标断言失败才算独立边界红。未到窗口、30秒性能不足、wide未入选择集等都不是红；不应为获取红先减canonical chunk/扩大visit。
4. `a_pure_pull_stays_pending_when_the_first_receipt_step_hits_a_real_gate`：只给B写大agents；A无push，B实际完整反向首块reply已记录后，在A receipts前持真实metadata gate，After必须Busy，drop gate后等真实首visit结束。首fixed target确实未达；释放A scope、确认无A resident，fresh Node同root public retry必须到相同epoch/VV+精确Spec(`/usr/bin/true`,fixed-flow-b)，B至少两次Exchange offer均同一manifest id且第二次agents非Pending。cleanup先成功，再断言首visitagents Pending。若重开未恢复、manifest变换、误认证或首target已完成，都是control失败，不能把它当B2红。它验证fresh Node重开，不伪称已经做postflush/prereceipt kill。

## 清理、执行限制及待实证

每个主要业务target之前显式f.stop：实际deafen B endpoint→abort/await resident→release probes→等peer RoundWatch真实结束→remove private Node根并assert不存在，打印cleanup marker。Scope/Ticket/Running/Resident Drop负责异常释放/abort；setup panic退出可能只有RAII，runner仍需后验PID/PGID、TMP，不用target文案覆盖cleanup失败。flow本身未另起系统host进程，relay/Node任务在本测试进程内；不能把内部任务当额外普通父。

四父均多线程Tokio，但设计运行方式是每父一个独立actual libtest进程，exact/test-threads1；同进程若并行≥3个flow，2root/fixture会超过MAX_ROOTS4，且真实metadata io::LOCAL是全局try-lock，fixture准备的State::open().unwrap也可能互相碰撞。当前四exact计划避免该问题，因此不是本次定向跑的阻断；不能因此宣称默认全lib并发可绿。后续若纳入普通全库并发，应与既有metadata TEST_SERIAL协调串行，或增加最小进程隔离，不能只抬MAX_ROOTS遮盖跨fixture gate干扰。

runner尚未独审/运行。最低要求：只构建当前Node actual libtest once，14源/1084前后同，actual fresh=false+SHA来源；四exact各自独立记录/分类/有限fuse+guard；quiet与三个目标分别判断（原N0/Pair不充数），负面结果要求所有列出control成立和cleanup先过。source等价、真实公平、30秒窗口是否可达和重启恢复都要由实际raw证明；本卡未取得任何红/绿。postflush/prereceipt真kill仍独立下一片，不因本次fresh reopen关闭。

## Root：06实际运行独立核对（2026-10-27）

本卡原审限定source05/候选阶段的结论保留；本段不是reviewer自行运行。Root已核06仅两处cfgtest clone、生产仍04；真实control03的83files与purepull runtime01的42files双盘SHA一致、每组1084 frozen inputs前后一致、四份actual raw summaries、原group/TMP/无forced收据及实际99b3c42b…6ab7二进制保留件。证据 `Q039/root-flow06-runtime-audit`；binary before-Q052 copy receipt `Q039/root-n1-flow06-pre-q052`。

quiet1 Versions绿；Busy6次实际6,377,994B、charged4,805,130B漏6整块，为目标真红；第三父30s内只有7,602,386B未达预算边界，是setup失败、无budget红；原第四purepull独立补跑首pending=false/target absent，实际fresh reopen按同manifest/epoch/VV/Spec完成后目标断言真红。3个业务红并未全部取得，整个N1没有绿。随后07另few-op合法Chat预算fixture已获授权，限额不变，原Device/Agent压力、N0/Pair/11保持。

最终纳入全库必须显式ignored并由版本化入口逐actual exe --ignored --exact隔离运行；ignore/list/0tests不能计绿。postflush/prereceipt实际kill仍未做。
