# 第二轮当前状态

更新2026-10-31。目标 **完成第二轮 active：29反馈＋5遗留＝34项**，尚未完成。无本轮main合并/ship授权，main仍e7c335c8 clean。逐项以[验收总账](ROUND-2-ACCEPTANCE.md)为准；规则读PLAN/ROUND-2-EXECUTION/ENGINEERING/STORAGE。旧状态和原失败完整保在[前检查点](history/STATE-before-864c4500-checkpoint-20261031.md)，不要按旧ref恢复新源码。

## 当前源码、分工与运行

Root `integrate/round2@cc92d47f04cf4aaac2b1a43109633fdbb6a2a15e` clean。两作者＋独立reviewer；Root只集成/验证/协调，不开第三实现线。

- input独占Node/ACP恢复：known2f已Root整合e313并实际验证；5da capability/private stable-token与ad30 execution/action分离已独审并整合c820/4bf（只有源码，未编译/运行）。3da＋1c11＋914 Meta-path公开3动作与typed纯拒绝已独审整合284/34b/2a（无编译/运行）；7313＋0799 logical-ID/有限3alias生产组件已独审整合473/92d。ed947＋c4已非作者源审并整合8bf/992：精确完整身份/正式published才投影canonical和控制；两个readonly删除入口持同physical journal与row guard完成删除，任意Meta/Attempt（含Reserved/Unknown）阻删除。没有当前编译/运行证明。最后abad＋154已独审并集成309f/cc92：权威Observation只读快照、现两条私有offer/native Ref数据转发、目标home真实来源重验、同逻辑SID admission与同File保存Ref已接；原R7内增真实owned-files零写offer/Unknown只读/注入正退出事实检查（未执行），不冒真实OS退出。当前作者clean，无已知生产后继，仍不宣整体恢复已可用。已授权Node lib/live/recovery_attempt/gui_host/必要session_recovery.rs、ACP accessor，后继link/ipc/proto/shell_start/Meta关系；不写Core/TS。能力不提前改旧Meta，绑定同epoch/full native/agent/effective launch；offer不探provider。后继同一≤16KiB文档须区分current execution owner与latest action/result各一份，Join不撤live生产者key，同token结束后仍不重新spawn。generic close旧问题仅NEXT。
- H独占Core/API/GUI消费：ac953共享serde DTO、e122贡献锚点、e211当前turn夹具均已独审整合。d549薄Core facade已独审整合74076（尚依赖未接齐Node接口，无编译证明）；b8/4c传输与cwd、74目录DTO及3f Core观察adapter均已独审整合；UI7ed→3ad→f617→3d→a8→1452→4b已源审并按精确小片集成至d197（没有React/IDB/类型/运行证明）。H正在复用98901原Recovery13 harness并明确增加4组当前真实DetailPane/RecoveryActions/真实IDB/Terminal owner验证，不改已固定生产源，不新wizard、不改model配置；core recovery.rs及SessionId仅加现ts-rs10元数据（依赖原有，不改锁/版本），GUI只映射home.hex避免平行enum。另一个极小原H测试夹具修正（见下）独立提交，不混产品。Node接口未固定前不真实gen/compile，生成文件不手写。
- reviewer非作者只读审源码/真实结果/Clean Code，无自主runtime；idle用followup_task。源码授权不等于构建授权。

当前 **Native槽RUNNING：Root actual28762**，生产源冻结cc92；browser/App/maintenance均FREE。最近browser actual28700完整H已terminal0/独审，38247 ACP完整绿亦保。旧句柄不重poll。以内部 `current-runtime.json` 与实际tool handle为准，不能因观察超时重启，不poll历史terminal句柄。

## 已确认结果与明确限制

| 范围 | 实际证据 |
| --- | --- |
| known-native生产接线组件 | Roote313 / actual14117 terminal0 /92.501s，full locked/offline metadata＋同次no-prior Node libtest V＋原8exact父各1/0/0＋2真Python SDK helper。result `ebc0052d64963266ee37af3b7f16fc7d88695cb6e4aa2ecbe4eaa96a662e1206`，已独审；2334源/env稳，owner/footprint空、TMPabsent，单私有binary核最终SHA/lsof后退役无gzip。原实际target SHA `250ca852161e71c1dc7730367c2cb1b792298e824416f794e7d800f0f13cb4f8` /288677480B。证明受控New/Load/Resume、production prepare/Ready validator与失败结算；不冒真实CLI parent、公有token/remote/Core/App/Shell、所有OS事实Known或generic close。 |
| 新建New生产链 | 005b actual54318 terminal0/92.141s，原8父＋2helper全过，result6a9938c7…51fa。是前一实际来源，原target9feb已历史，不能冒当前250c来源。 |
| 当前Reader | Root864 actual75722真实脚本exit0、28PASS（新贡献assert加在原case内），2335源/env/tools/owned/TMP净。Root caller误期待29使原组合result `15ee848b00ca25b425d0afe17c27fd166cffae72fbcdc588b8f969190d18e729` false保留；adjudication `07e68de04f2f1d3c4e4dedefbed6ce398fdc94a5c68f4723ee022895af97d335` 独审确认真实28已过，不改原false、不重复跑Reader。import准备错误也保且未启动Node。 |
| 当前完整Find | Root864 actual55298，完整32PASS/exit0/3.278s含真实Worker，result `fef9c1516c0ec94a777cf3a125ca7ec7a8603397d2806663aae11194bacc68cd`，已独审；2335源/env/tools/owned/TMP净。共享spans retirement未改变搜索/copy内容；不是DOM/WK/types证明。 |
| 文字Range红基线 | Root298 actual55767 terminal1/6.564s，真实512-entry窗prepend128淘汰尾部，同entry200/offset49/sample仍在，文字top0→10240；result376b7236…78c3，已独审业务红、非timeout/forced，清理净。e122修复后完整H actual28700已通过全部Range子场景；红基线保留。 |
| 当前完整H消费者尝试 | Root864 actual59083 terminal1/7.848s；前4组搜旧DOM/源copy/giant拒截断/raw export过，原第5组held-copy顺序断言失败，未到Range。result `dae945ff8a231b354767b0a8f0e02f7b61c45ef151318cc98586549bf6a47b08`，log22acde4c…57e8e，已独审；2335源/env/gen/tools/strict清理全净。持有的是空tail poll lease72/f:A1300，而Copy lease73/f:A20已完成，any-newer测试flag被无关IO消费，不能据此归产品顺序错。 |
| ACP完整18947/9255 | 两次原false保。9255在b7完整跑121.040s，result `963806b5e50c6c1560e0e9e0edca98d7b1f599aff6bf1f903697c1b061b8753d`，严格清理/源稳且独审。真实lucide SVG selector正确；default fixture仅Notes无Turning→ChatView inTurn=false，Loader正确不渲染，不是产品CSS缺口。e211只补合法turning事实，原SDKpayload/4主题动静/所有现assert与nonvisual保；38247在864实际完整通过：terminal0/120.284s，result `0cf68f0cf2b694b5afa188ba54823da76009d9801966a247001312df540626c8` / loga5a924f7…7fba，4theme×motion/附件/模式配置/目录导入/registry/idle/press均实际到达，2335源/env/tools/gen及strict清理净，已独审。后326仅8个TS derive元数据，不重复该未变消费闭包；不冒WK/Native/fulltypes。 |

上述原件在 `/Volumes/disk0/Khor/evidence/round2`；内部recovery只紧凑元数据/当前源码，不镜像大日志、截图或程序。旧R1 actual57370五exact、disclosure92761 short/long真实IO/暂停恢复、AI63835四场景、Store95235真实67,174,400B、Node60432/ACP63174/Recovery98901/Read62914各自原范围保持；无关通过未因新TS全重跑。旧失败75482/22391/36944/AI多次false/74275/18947/9255均不回填，详情可追前检查点和原证据。

当前完整H已实际闭合：Roote9 actual28700 terminal0/16.253s，默认原7＋全部Range子场景的8组全过，result `925385c43a3324e79d9e36ddc4cb36716b5218182e7d14cc1f210d77a0be86f0` /report `d6ac8e8688accde4fd602e80a058324022597629e6ba122b579f10b9dfca7a31` /loga4b33f79…2a39c，独审。prefix重排、真实wheel新B胜旧A、512窗尾淘汰后同字点≤2px/原RO+六doubleRAF+120ms全到；当前gesture Clipboard次数/激活/顺序与A→B退休原要求实过，终态OWNER_BARRIER/writes3/rejected2、held null/leases{}。2335源/env/gen/tools与strict清理净，13compact metadata无大镜像。仅受控Chrome生产UI/Worker与API/clipboard内存sink，不冒WK/OSSave/OSClipboard/当前Rustgen/fulltypes；后续只改Core/Node/胶水，不重复未变Reader/Find/ACP/H。

当前S3实际源码/待验边界：Core/API/Tauri/TS/cwd与UI源已联成组件checkpoint，后台Meta-path三方法也已写；均没有当前编译/生成/运行结论。UI源码已审关闭自身row-status误退回复、sameSID重接/home资格、A→B→A单向退休、旧join/user-send门和presentation等待假IO卡死；busy传输/Save/键发送/ACK仍须实际验证，不能用有限predicate红绿替代React。frontend pending intent为真实IDB复合(home,id)键每target一条、总32/每条8KiB，支持不同target并行，RPC在事务外；仅caller payload不存执行事实/全史。缺Web Locks不能作为LAN HTTP拒执行门，现选IDB无需新依赖。HeaderSeat复用不新portal框架，重查文案明确“重新检查并恢复”。

纯Observation仍有必要后继：目标observe/<tuplehash>必须与provider nativeID独立，不能偷偷新tui/id_for；3alias恰原tui＋Native/Acp观察两ID，samephysical Journal File→有限alias fence→durableReserved，child只豁免自身exact claim，其他Unknown/Reserved/barrier阻新spawn，逐个rowgate不双持；New quarantine真实bound旧alias、保旧高水/result，genericclose原NEXT仍不宣全安全。远端owner可持execution home不曾保存的ObservationRef，私有现三request允许携其为数据，目标home重验hash/fullnative/本机qualified来源；offer临时只读不落盘，action持同reservation保存真实引用供child；不加RPC/adapter/parser/全扫。后继必须把sameidentity+正式pub资格接到Source::registered、selected_content、session_dir/inputowner、观察投影去重；H3f只复用现local/Reach实际权威，不从prefix/ref存在就授控制。旧CLI/New ID布局及所有既有数据保持。

组件完整参数回放边界不扩：同token且原full-effective-args未变才回放；目录/defaultshell/title/launcher派生参数改变可typed Refused，绝不新spawn或UI自动换nonce。IO/claim/ghost不确定不假拒绝，UI保旧payload。DirectoryRequired持久同token无spawn，明确选cwd另取新epoch/nonce。

协作自主规则：两作者在各自本轮责任域必要调用点可直接最小适配，不再逐函数等Root；H独占GUI/Core/shared DTO，input独占Node/ACP及必要只读Store。共享契约实质变化/跨作者写域先协调，独立新问题只NEXT；静态/有限纯逻辑可自做准确留证，Cargo/gen/Chrome/App/真实provider仍Root唯一重型槽。保持固定小片独审，不等巨段统一交付。

## 眼前闭环与后续顺序

1. H第5组夹具900b已独审整合e9并被上述28700实际通过，不再阻塞；原59083 false保。e122源码与原cap业务红保留因果链，复杂Markdown无法证明source映射时仍诚实fallback，不冒所有情况exact保证。下一步重点是公开S3接线，不扩大新问题。
2. S3三async Node接口已冻结给H并行source消费：`session_recovery_on(home_hex,&SessionId)->SessionRecovery`；`recover_session_on(home_hex,&SessionId,expected_run:Option<&str>,RecoveryToken)->RecoveryResult`；`new_shell_run_on`同参数再加尾`cwd:Option<&str>`→ShellRunResult，Shell要求Some(oldSID)。先前freeze漏cwd已按原DESIGN-v2目录选择要求纠正，不退成纯拒绝：None仅旧cwd、Some用户明确选新目录（空白不回退home）、raw/effectivecwd绑定token，不改旧A Meta.cwd。共享DirectoryRequired{id:oldSID}返回无spawn且同token持久结果；用户确认目录属于新intent，重新offer epoch/newnonce。74a6纯DTO与Facade/API/Tauri/TS cwd透传已独审集成（dcf/092）；尚无类型/运行证明。真实DeviceId peer/resident和本机复核，无显示名/本地回退，offer无写无probe，actions非0RTT。Meta-path公开稳定token/typed Join/Pending/Unknown/Refused与Shell新SID/default shell/空command/previous-root已有source组件，观察资格消费者与真实验证仍未完；prior Unsupported也有显式重新检查（新Init再guard），未知不背景probe；没有接口完成声明。offer现有agent()/devices()会走可能reseed的Store load，已授crates/sync/src/store.rs复用既有读体导出只读read_snapshot，无Lease/create/reseed/cache/flush，稳定性不足Err不内部loop，仅必要现测试，不扩同步治理或重跑64MiB。这是原zero-write offer的必要调用链适配。
3. H真实入口是DetailPane（SessionContent/MobileSessionContent是文件/浏览器附栏）；复用现onSession/reload/rejoin路径，同SID Opened/Join要重解canonical/control而不是捏新GUI ID，新Shell才开新SID。history先显示，offer迟到按home/id/run拒，动作显式、Pending/Unknown不自动重启。
4. Node/Core接口固定后**一次**实际gen/catalog/noEmit/Vite；现179gen/278dist仍16ee旧来源，新HistoryEntry字段和Core恢复DTO未真实生成。之后受影响C/Core原生及同源CLI/GUI生产构建。旧GUI48383资产来源门false保，最终仅精确Tauri OUT核SHA/devino/lsof可逆retain后真实cold，不mtime/fakeprior/放宽V（详R2-V）。
5. 完整S的真实host正常Quit重挂/死后显式恢复/断线/新Shell；E真实Mac IME/路径拖选/全入口草稿/保存失败取消/最后键→正常OSQuit；长历史系统Save/Copy；桌面W/C/G/Q/B组合和真手机viewport/候选/键行/滑删仍未完。私有fixture，不动真实用户会话/模型API；真人前重新确认方便，旧回答已失效。
6. 旧卡死R2-001和原网址WK连接拒绝R2-016未证修复；不能以其它绿替代，若最后仍无法证明须用户明确接受延期。新独立问题只NEXT，保34项不缩目标。用户本次询问后当前估计仍3–5工作日完成完整目标，下一可测Lab6–10小时；真人与手机环境等待另计，仍有实现工作。

只读真机准备核查：devicectl 2026-10-31 inventory exit0，iPhone15ProMax/iOS27.0 build24A5390f已paired、developer mode enabled，但tunnel unavailable/DDI false；本机Xcode26.6/17F113。未安装/启动App、未下载SDK/DDI，不能把SDK当唯一原因。原件platform/device-readiness-20261031/211411-*；真正验收前准备当前包并重新确认用户方便。

## 存储与恢复

定向清理已释放外104.715GiB、内4.004+1.775GiB；当前外约112GiB、系统13GiB/recovery2.2GiB，发布缓存117GiB/quality40GiB。源码/会话/正式发布与保护target未删，部分khor*只是兼容链接。每批严格外20/内10GiB保留量＋显式峰值，e70 preflight原件随批保存；原5GiB只紧急停止线。同quality target/jobs1/incremental0/offline，私有程序批末退役、不gzip和双盘大payload，日志截图外盘唯一。quality50GiB/recovery5GiB先复核再开下一批，不扩空间治理（见STORAGE）。

先本页/PLAN/STORAGE，再实际Git/dirty/runtime；作者任务卡dirty不得全局stage，不按摘要覆盖。当前小bundle链各经gitverify0并双盘；最新b7相对298 bundle `fd8855d68e4d562953f084dcf05e35dc563056dc054d235c9d15582bd627f8a3`/11973B，后326相对b7的source bundle `5d74efbafea607fb71a8ef1d44a38ed5c4c449bfbc454d1550e073c9a1fe920b`/6510B已verify0双盘。两作者当前dirty bytes逐文件保current-author-source/manifest，非整体原子，不覆盖后来源。内部 `/Users/zgh/Library/Caches/khor-code-quality/recovery` 的RESUME/refs/docs/runtime按逐文件事实同步，不称全局原子。历史binary/source SHA仅历史，退役payload不冒仍在。

## 当前统一生成批（2026-10-31）

- actual45451 terminal1/6.052s：metadata与catalog build均0，但wrapper移植漏原compile-only `--test gate`，prepared全图catalog＋core与实际仅catalog不符，被原V正确拒绝。result `25941d421d09ed05f4ae33284371634b02ce34a1c5a741eb5ebe919e7b0b4a20`；未stage/publish，源/env稳、owner空、TMP清，非产品业务红，原false保。
- 仅恢复原catalog编译目标（不执行gate），run2 SHA `2971fc8a388e8f274cf3036fe2ad739305a00ebf5149a02f7514dfbfc09b16fc`已独审，fresh e70 reserve20/10＋peak6/2后actual28762正在运行；catalog stage已到，bindings正编译，尚无本批通过声明。不要poll已终态45451；runtime以真实28762/current-runtime为准。
- H只改3测试源的Recovery17片743已fixed clean，原13assert保＋4明确S3组，正在非作者审。Root本批不受作者树变动影响；该片后继若仅测试delta，按精确源映射复用生产types/assets，不为script变更重gen。
