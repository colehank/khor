# 第二轮当前状态

更新2026-10-31。目标 **完成第二轮 active：29反馈＋5遗留＝34项**，未完成。本轮无main合并/ship授权；main e7c335c8保持发布基线。逐项要求与待验看[验收总账](ROUND-2-ACCEPTANCE.md)，执行边界看PLAN/ROUND-2-EXECUTION/ENGINEERING/STORAGE。完整前检查点与历史失败在[归档](history/STATE-before-9463793e-checkpoint-20261031.md)，不能按旧ref覆盖新源码。

## 当前源码、协作与运行

Root `integrate/round2@9463793ef10d0677b21e43af6fa3a1b046b7104c` clean。两作者＋独立reviewer；Root负责集成/实际验证/协调，不开第三产品实现线。作者在责任域内可最小适配必要调用点，跨域/共享语义改变先协调；新独立问题仅NEXT，不扩大本轮34项。

- input独占Node/ACP/必要只读Store；最后生产no-Meta/Ref abad＋154→Root309/cc92，64观察ID路径a92→Root946，均独审。现只补原Resume fixture的能力事实期望：wire始终load=true/resume=true，getter正确，原新增{false,true}期望混淆能力与选择；不得改wire成load=false或弱化Resume1/Load0/New0/partial origin/Close/reap。待固定与独审，未运行新批。generic close独立旧问题仅NEXT。
- H独占GUI/Core/shared DTO。生产4baf链已独审整合；Recovery17三测试片743→Root801独审并实际通过；单TS Extract收窄c0→Root4392独审/运行JS字节同，actual fulltypes/Vite通过。H现source-only准备既定Core3exact＋tables4/两integration仅compile的窄driver，不改产品或运行；7selected包含3自动bin，仅两实际运行程序私有copy，原guard/原V/唯一槽不改。
- reviewer只读源审与真实尾核，无自主runtime。三方责任卡dirty保留，不全局stage。

当前Native/browser/App/maintenance均 **FREE**。最新native actual14794已终态false（详情下）；browser13057/fulltypes48300已终态绿且独审；旧句柄不poll。真实状态与后继运行以内部 `current-runtime.json`/实际tool handle优先，不因观察超时重启。没有当前统一App产物/全范围验收通过声明。

## 最新实际验证

| 范围 | 已核事实与边界 |
| --- | --- |
| 真实当前生成 | Rootcc92 actual28762：metadata、catalog build/stage、bindings build/stage/pub六phase0；两原no-prior prepare/verify/require成功，prior=[]/reusefalse、actual CargoJSON selected实同。186bindings stage/pub全dict同，含catalog/manifest共188gen。整体false因后继fulltypes单TS2345，不能改绿；result `5b69534015cc74d0042df7016f66b7889c64e2d52b9ca53e2687ba84e35147ce`。2339源/env/tools稳定，owner空/TMP absent/清理空，独审。 |
| 当前fulltypes/Vite | Root4392 actual48300 terminal0/14.370s，复用上述真实生成，仅exact3test＋1type-only源差，19prior pins/两V/六phase/旧false逐SHA核对；types→Vite全0。2340源稳定、188gen/278dist逐dict实同，清理净，独审。result `4e2ea0c661cbce6a03eb37cf112f56dd37eb74658a0185487eacebfbd700f879`；frontend receipt `a4314eb0b854049e7ba94bffebc4549ef32b8104eedfae1ad550c1ccb393fb42`。generation cc92与frontend4392分列，不冒新Rust producer或App。946仅两Node路径改变，GUI/types/assets源字节不变，不重gen。 |
| 当前Recovery17 | Root4392 actual13057 terminal0；原13完整assert保＋4明确S3组，exact17全ok/finished、errors/console/network0。真实DetailPane/hook/同context-origin IDB与Terminal mounted owner；同target5次payload不变、peer同payload/different-target并行、cwd null epoch4→chosen epoch5新nonce、同SID重绑、busy drop/ABA/另一kept原实例全部到断言。2340源/188gen/env/browsertools/fixtures稳定、owned/TMP全净，独审。result `ea117908f1f6808e87864035a8066f5befce8b4fa6582f7232e67a34f6d1a0dd` /report `7084ee576fef2b53ee10c26c8fc57c59296aa9a07b6df4f0e8b8a2dd0629a5ed`。仅controlled Chrome真实组件与外部IO sink，不证Node peer/provider/OS/WK/App。 |
| 最新Node组件14794 | Root946 actual14794 terminal2/86.825s；metadata/compile0、原no-prior V真过，run1–7各唯一1/0/0＋journal真Python helper1/0/0；R7观察64/原token/fence/只读offer/正资格模型分支实际通过，不把注入退出当OS。最后run8 resume helper在能力事实assert真实失败：wire load/resume都true，原新增期望load=false。没有后续Resume/capture/close通过声明。result `76b47012f8483bf5f45b06123605c1a6314909f3992addbee904d5d3550de078`；源/env稳、真实group无剩余，privatecopy04d69已核SHA/lsof退役；panic留一自有eventstore目录及part在TMP原件，不称全净。独审已核。 |
| Node观察路径红89125 | Root4392 actual89125 terminal2/97.875s：compile/V及run1–4过，R7在2022真实observe64 ID被旧dir24cap拒而None panic，后run6–8/helpers未跑。产品a92只observe64lowerhex保原完整映射，其他kind原限制/全部原assert保；已独审并在14794实际闭合。result `09d7704fa4eeda29e0941f973ceffa87e034b04556702644a02fc3469e3836f3`原false保。 |
| known-native历史绿 | Roote313 actual14117 terminal0/92.501s，原8exact父＋2真PythonSDK helper，新建/Load/Resume和prepare/Ready组件通过；result `ebc0052d64963266ee37af3b7f16fc7d88695cb6e4aa2ecbe4eaa96a662e1206`。当时原target250ca为历史，最新14794实际target04d69；不能用旧绿替代后继public/能力全批。 |
| 当前H/Find/ACP | actual28700完整H原7＋Range前缀重排/新wheel胜旧A/512窗尾淘汰保留同字点≤2px全部绿，result925385c4…e86f0；Find完整32 actual55298绿；Reader实际28 actual75722绿，但caller误计29原false与独立adjudication保；ACP actual38247完整4theme×motion/配置附件/导入等绿。均独审。c0只TS返回类型、JS字节同，不重未变Find/H/ACP。真实WK/系统出口不在这些范围。 |

原件在 `/Volumes/disk0/Khor/evidence/round2`。完整原失败与其它既有R1/Store67,174,400B/Node/ACP/Recovery旧13/read/实际L02/L05等证据逐包保留，详归档与总账；未受影响结果按源映射复用，不重全仓矩阵。45451为wrapper漏原catalog compile-only `--test gate`的来源覆盖准备失败（非产品红），已恢复原参数再真实生成；原false `25941d42…4a20`完整保。28762整体typesfalse亦不回填。

## 恢复边界与后继顺序

源码已接三async Node公有方法（execution-home DeviceId路由、resident/peer、本地身份复核）：session_recovery_on、recover_session_on、new_shell_run_on；Shell尾cwd可选且expected_run必须Some(oldSID)。Core/API/Tauri/GUI薄透传，SessionRow.execution_home来自home.hex，display home不得作路由。真实公有IPC/CLI parent/远端/完整App仍待验证，不能因组件绿宣全部可用。

关键契约保持：offer无provider probe/写入；Unknown执行不授spawn，prior Unsupported可显式重新检查；ObservationRef是数据，目标home重验真实来源，完整原逻辑SID不变；同physical JournalFile→有限3alias→Ref保存→Reserved，child仅豁免自身exact claim；正式published才切canonical/control，真实GUI2/input owner仍必须认证；删除引用持同Journal/row锁，任意Meta/Attempt（含Reserved/Unknown）阻删除。现同16KiB文档分execution owner/latest action，不另造registry；Join不撤live producer key，Pending/Unknown/transport不确定复用同payload。只读身份/正文与恢复资格分开。

UI真实IDB只保存caller payload：(home,id)每target一条、总32、每条8KiB；事务外RPC、不同target并行、无WebLocks安全上下文硬门。Terminal mounted(home,id,generation) owner一次退休，home/ABA等待真实IO/Save/ACK/keys，不伪全局SID reload。DirectoryRequired同token无spawn；用户选cwd另取offer/epoch/nonce，不改原A cwd；新Shell B/default shell/空command/previous-root在spawn前持久。

下一步顺序：
1. 固定并源审Resume能力事实期望窄修后，仍原8exact＋2helper/4045driver/300s，不加suite，实际重新编译与验证；保14794原false和part原件，不手改getter或削弱Resume优先级。
2. Core既定3exact＋tables4；terminal_current/chat_upload_cancel与3自动bin仅compile。H准备driver，Root唯一准入，按最新head/真实frontend188gen278dist/两Node源差固定；不重gen或已绿UI。
3. C原hermetic shim/gui_refused；实际同源CLI/默认custom-protocol GUI生产产物。Tauri旧48383资产门false保；只允许精确OUT核身份/SHA/lsof可逆retain后真实cold，不fakeprior/mtime/弱V。当前GUI/Core代码源审与Chrome不是实际App编译/运行结论。
4. 完整S私有真实App：同host正常OSQuit重挂、确认已退后显式Load/Resume/unsupported、精确home offline不回本机、新Shell B!=A/previous-root/空command/同token单B/目录新意图；真实public方法/CLI parent不由model/fixture替代。只私有fixture/控制协议，无真实用户会话/模型API；generic close独立旧问题NEXT，不宣所有close已安全。
5. 当前App桌面W/C/G/Q/B组合，真实WK进度/主题首帧/原URL拒绝；长历史真实系统Save/Copy；Mac IME/路径拖选/所有文件入口草稿/保存失败取消/最后键→正常OSQuit；真手机同页viewport、候选跟手与键行/滑删。准备好对应当前实例后重新确认真人方便，旧回答无效。devicectl只读inventory：iPhone15ProMax/iOS27paired+developer，但tunnel unavailable/DDI false；未下载/安装/启动，不把SDK当唯一原因。
6. 34项逐项完成审计＋非作者Clean Code审查。R2-001旧卡死/R2-016原网址仍未证修复；不能以别的绿替代，若无法闭合须用户明确接受延期。目标仍不只是验收；先前粗估Lab6–10小时/完整3–5工作日，非完成承诺。

## 存储、压缩与恢复

读STORAGE。当前外约112GiB/系统约13GiB、quality约40GiB/recovery2.2GiB；已定向释放外104.715GiB、内4.004＋1.775GiB。每批e70 preflight严格外20/内10GiB保留＋显式峰值；原5GiB仅紧急停止。唯一quality target/jobs1/incremental0/offline；quality50GiB/recovery5GiB先复核。运行私有binary批末严格退役、不gzip/双盘大payload；日志截图外盘唯一，内部仅当前源/文档/紧凑结果索引。用户会话/发布产物/源码未删。

当前小source bundle链已verify0且实际双盘：recovery-consumers992相对b735（0eb3264c…c3c7/27989B）、observed-publiccc92相对992（325e9283…a854/9700B）、recovery-ui-types4392相对cc92（eb398508…582c/10337B）。最新946后继需补小bundle；作者dirty snapshot为逐文件事实，不是全局原子，禁止覆盖后来源。先STATE/PLAN/STORAGE，再核真实Git/diff/运行handle；内部 `/Users/zgh/Library/Caches/khor-code-quality/recovery` RESUME/refs/docs/runtime按最新逐项更新，不因旧摘要覆盖新实现。
