# 第二轮当前状态

更新2026-10-31。目标 **完成第二轮 active：29反馈＋5遗留＝34项**，尚未完成。无本轮main合并/ship授权，main仍e7c335c8 clean。逐项以[验收总账](ROUND-2-ACCEPTANCE.md)为准；规则读PLAN/ROUND-2-EXECUTION/ENGINEERING/STORAGE。旧状态和原失败完整保在[前检查点](history/STATE-before-864c4500-checkpoint-20261031.md)，不要按旧ref恢复新源码。

## 当前源码、分工与运行

Root `integrate/round2@864c4500f25302deb6d86553cc84ddf69f568f9e` clean。两作者＋独立reviewer；Root只集成/验证/协调，不开第三实现线。

- input独占Node/ACP恢复：known2f已Root整合e313并实际验证；当前在author tree补真实Initialize capability持久事实与caller-stable token窄片，其后同一owner串行做公开home路由/native动作/新Shell。已授权Node lib/live/recovery_attempt/gui_host/必要session_recovery.rs、ACP accessor，后继link/ipc/proto/shell_start/Meta关系；不写Core/TS。能力不提前改旧Meta，绑定同epoch/full native/agent/effective launch；offer不探provider。后继同一≤16KiB文档须区分current execution owner与latest action/result各一份，Join不撤live生产者key，同token结束后仍不重新spawn。generic close旧问题仅NEXT。
- H独占Core/API/GUI消费：ac953共享serde DTO、e122贡献锚点、e211当前turn夹具均已独审整合。现在并行接Core facade/API/Tauri/TS/DetailPane与私有Recovery动作hook，不新wizard、不改model配置；core recovery.rs及SessionId仅加现ts-rs10元数据（依赖原有，不改锁/版本），GUI只映射home.hex避免平行enum。另一个极小原H测试夹具修正（见下）独立提交，不混产品。Node接口未固定前不真实gen/compile，生成文件不手写。
- reviewer非作者只读审源码/真实结果/Clean Code，无自主runtime；idle用followup_task。源码授权不等于构建授权。

当前browser槽仅Root **actual38247**（完整ACP，head864）运行；Native/App/maintenance FREE。以内部 `current-runtime.json` 与实际tool handle为准，不能因观察超时重启，不poll历史terminal句柄。

## 已确认结果与明确限制

| 范围 | 实际证据 |
| --- | --- |
| known-native生产接线组件 | Roote313 / actual14117 terminal0 /92.501s，full locked/offline metadata＋同次no-prior Node libtest V＋原8exact父各1/0/0＋2真Python SDK helper。result `ebc0052d64963266ee37af3b7f16fc7d88695cb6e4aa2ecbe4eaa96a662e1206`，已独审；2334源/env稳，owner/footprint空、TMPabsent，单私有binary核最终SHA/lsof后退役无gzip。原实际target SHA `250ca852161e71c1dc7730367c2cb1b792298e824416f794e7d800f0f13cb4f8` /288677480B。证明受控New/Load/Resume、production prepare/Ready validator与失败结算；不冒真实CLI parent、公有token/remote/Core/App/Shell、所有OS事实Known或generic close。 |
| 新建New生产链 | 005b actual54318 terminal0/92.141s，原8父＋2helper全过，result6a9938c7…51fa。是前一实际来源，原target9feb已历史，不能冒当前250c来源。 |
| 当前Reader | Root864 actual75722真实脚本exit0、28PASS（新贡献assert加在原case内），2335源/env/tools/owned/TMP净。Root caller误期待29使原组合result `15ee848b00ca25b425d0afe17c27fd166cffae72fbcdc588b8f969190d18e729` false保留；adjudication `07e68de04f2f1d3c4e4dedefbed6ce398fdc94a5c68f4723ee022895af97d335` 独审确认真实28已过，不改原false、不重复跑Reader。import准备错误也保且未启动Node。 |
| 当前完整Find | Root864 actual55298，完整32PASS/exit0/3.278s含真实Worker，result `fef9c1516c0ec94a777cf3a125ca7ec7a8603397d2806663aae11194bacc68cd`，已独审；2335源/env/tools/owned/TMP净。共享spans retirement未改变搜索/copy内容；不是DOM/WK/types证明。 |
| 文字Range红基线 | Root298 actual55767 terminal1/6.564s，真实512-entry窗prepend128淘汰尾部，同entry200/offset49/sample仍在，文字top0→10240；result376b7236…78c3，已独审业务红、非timeout/forced，清理净。e122修复已整合，但当前尚无浏览器Range绿。 |
| 当前完整H消费者尝试 | Root864 actual59083 terminal1/7.848s；前4组搜旧DOM/源copy/giant拒截断/raw export过，原第5组held-copy顺序断言失败，未到Range。result `dae945ff8a231b354767b0a8f0e02f7b61c45ef151318cc98586549bf6a47b08`，log22acde4c…57e8e，已独审；2335源/env/gen/tools/strict清理全净。持有的是空tail poll lease72/f:A1300，而Copy lease73/f:A20已完成，any-newer测试flag被无关IO消费，不能据此归产品顺序错。 |
| ACP完整18947/9255 | 两次原false保。9255在b7完整跑121.040s，result `963806b5e50c6c1560e0e9e0edca98d7b1f599aff6bf1f903697c1b061b8753d`，严格清理/源稳且独审。真实lucide SVG selector正确；default fixture仅Notes无Turning→ChatView inTurn=false，Loader正确不渲染，不是产品CSS缺口。e211只补合法turning事实，原SDKpayload/4主题动静/所有现assert与nonvisual保；38247当前一次重验中，不预称绿。 |

上述原件在 `/Volumes/disk0/Khor/evidence/round2`；内部recovery只紧凑元数据/当前源码，不镜像大日志、截图或程序。旧R1 actual57370五exact、disclosure92761 short/long真实IO/暂停恢复、AI63835四场景、Store95235真实67,174,400B、Node60432/ACP63174/Recovery98901/Read62914各自原范围保持；无关通过未因新TS全重跑。旧失败75482/22391/36944/AI多次false/74275/18947/9255均不回填，详情可追前检查点和原证据。

## 眼前闭环与后续顺序

1. H第5组夹具窄修：仅A/newer且DTO含固定目标source contribution key50000000:0才holdCopy，空tail不消耗flag；不得按clipboard事件arm自证。保原激活/顺序/退休不交付，并限定本次writes+1与本次order起点以免旧copy假过；finally保≤32条order尾/heldKey/heldcursor。独立单script/source审后Root跑当前完整原7+Range prefix/late-input/cap，不重红基线、不重未变pure闭包。e122已保≤512贡献spans、实际DOM48字sample/entry-offset、Worker retire和newer-input ticket；复杂Markdown不可证明时诚实fallback，不冒exact保证。
2. S3三async Node接口已冻结给H并行source消费：`session_recovery_on(home_hex,&SessionId)->SessionRecovery`；`recover_session_on(home_hex,&SessionId,expected_run:Option<&str>,RecoveryToken)->RecoveryResult`；`new_shell_run_on`同参数→ShellRunResult，Shell要求Some(oldSID)。真实DeviceId peer/resident和本机复核，无显示名/本地回退，offer无写无probe，actions非0RTT。公开稳定token/typed Join/Pending/Unknown/Refused及Shell新SID/default shell/空command/previous-root仍在实现；没有接口完成声明。
3. H真实入口是DetailPane（SessionContent/MobileSessionContent是文件/浏览器附栏）；复用现onSession/reload/rejoin路径，同SID Opened/Join要重解canonical/control而不是捏新GUI ID，新Shell才开新SID。history先显示，offer迟到按home/id/run拒，动作显式、Pending/Unknown不自动重启。
4. Node/Core接口固定后**一次**实际gen/catalog/noEmit/Vite；现179gen/278dist仍16ee旧来源，新HistoryEntry字段和Core恢复DTO未真实生成。之后受影响C/Core原生及同源CLI/GUI生产构建。旧GUI48383资产来源门false保，最终仅精确Tauri OUT核SHA/devino/lsof可逆retain后真实cold，不mtime/fakeprior/放宽V（详R2-V）。
5. 完整S的真实host正常Quit重挂/死后显式恢复/断线/新Shell；E真实Mac IME/路径拖选/全入口草稿/保存失败取消/最后键→正常OSQuit；长历史系统Save/Copy；桌面W/C/G/Q/B组合和真手机viewport/候选/键行/滑删仍未完。私有fixture，不动真实用户会话/模型API；真人前重新确认方便，旧回答已失效。
6. 旧卡死R2-001和原网址WK连接拒绝R2-016未证修复；不能以其它绿替代，若最后仍无法证明须用户明确接受延期。新独立问题只NEXT，保34项不缩目标。此前3–5工作日是粗估，不再说仅剩验收。

## 存储与恢复

定向清理已释放外104.715GiB、内4.004+1.775GiB；当前外约112GiB、系统13GiB/recovery2.2GiB，发布缓存117GiB/quality40GiB。源码/会话/正式发布与保护target未删，部分khor*只是兼容链接。每批严格外20/内10GiB保留量＋显式峰值，e70 preflight原件随批保存；原5GiB只紧急停止线。同quality target/jobs1/incremental0/offline，私有程序批末退役、不gzip和双盘大payload，日志截图外盘唯一。quality50GiB/recovery5GiB先复核再开下一批，不扩空间治理（见STORAGE）。

先本页/PLAN/STORAGE，再实际Git/dirty/runtime；作者任务卡dirty不得全局stage，不按摘要覆盖。当前小bundle链各经gitverify0并双盘；最新b7相对298 bundle `fd8855d68e4d562953f084dcf05e35dc563056dc054d235c9d15582bd627f8a3`/11973B，后续864源码也须镜像小bundle。内部 `/Users/zgh/Library/Caches/khor-code-quality/recovery` 的RESUME/refs/docs/runtime按逐文件事实同步，不称全局原子。历史binary/source SHA仅历史，退役payload不冒仍在。
