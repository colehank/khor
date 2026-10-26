# Q033 当前GUI协议候选独立审查

2026-10-27，reviewer `node_test_review`，非本片作者；此前只写Q-033-current-protocol-plan.md。本审只读源码/证据并写本独占卡，未Cargo/gen/native/browser、未改源码或提交。自身Q041e三源保持冻结，不能作为其独审人。

## 固定范围与结论

实现树 `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-store-owner-recovery`，`fix/quality-current-gui-protocol@686f2e42eb634ff5ba0edc34ab822fe8764f13f2`，candidate01恰7 tracked改动+1新测试源。external `Q033/current-protocol-candidate01` 11文件与internal recovery同名逐SHA一致，8源全副本与初次审查现场逐字一致。SHA清单见该source-freeze.json。

**生产/协议源码未发现本片新增阻断。candidate01整体有一项明确测试编译阻断，必须解除并实际跑host3后才可称本片定向验证通过。** 不能把已完成normalCore+16普通父当作host3或完整GUI→Rust验收。root已确认只修下述mut并另冻candidate02，待delta与最后收据复核。

已读权威STATE/PLAN与当前协议定界卡。按review-changes先minimal→detect/affected/tests图：此树无.codegraph；CRG图build `a723bbba`、46 files、不匹配686f；只认导航提示。图7 changed漏新gui_greeting.rs、0flow/0tests不代表覆盖；转当前完整diff、8源及实际local/far/rejoin/Stop/host reassembly调用链。没有为审查更新图。

## 阻断与最小修复

**B1（测试setup/编译，不是产品红）**：candidate01 `apps/gui/core/tests/chat_upload_cancel.rs:180–181` 把cleanup连接换成 `gui_host::connect_at` 后，`attached` 未声明mut却借用 `&mut attached.stream`。`current-protocol-host-green-02/host.log` 实际E0596，build-finished false，尚未进入任一host业务case；11.291s退出101、无资源guard。最小修复只加 `if let Ok(mut attached)`，保原Close/host.join与所有业务断言。建议只续相同3普通父，保编译失败记录；不必重跑未变normal/greeting/io16。

## 当前行为与oracle核查

- `crates/node/src/gui_host.rs:178–192` 只接受完整相等 `khor-gui/1`；空、陌生、terminal及其他GUI版本都明确catalog `msg.host-gui-protocol`。先走shared `host::connect_welcomed:768–791`，仍发送原cookie/0×0 Hello、先保Welcome.ok=false原why、15s deadline；不额外写terminal Version。host自身`:970`仍发布同一当前greeting。初连local/far（chat.rs:521–556）及far rejoin（:484–492）均调用此严格入口。
- `Chat.upload_cancel`及三初始化/赋值删除，生产 `legacy_upload` 和 `StopAction::Disconnected` 消失。静态全apps/crates Rust检索无这些剩余分支。`chat_send.rs`与baseline仅删除一个Disconnected variant，`chat_io_tests.rs`与baseline仅删除一个fixture字段；逆替换后逐字相同，12原oracle未弱化。
- `chat.rs:733–765,833–884` 保current Stop→上传终止Parts、Queued本地取消、Committed原agent Stop，控制/turn双FIFO和synchronous admission不变。Stop写失败或末Parts写失败仍产生reconnect=true并retire原attachment；不能因用户拒绝旧Khor兼容删除这些current IO错误恢复。StopSignal/StopWriter Drop仍唤醒取消发送者。
- token仍optional/bounded，nullable取消仍保持；host GuiOp无本地token。SendCommitted仅完整写commit后由claim_receipt一次发出；失败/取消互斥receipt、旧Arc attachment隔离及same-text新token12case均保留。native_io/API/ChatView/draft registry未改，不能借本片声称已验真实Tauri上传。
- `Reassembly:268–296` 完全未改：Stop清held并置over，残Part不能恢复，Parts仅清discard不执行旧payload。改名的residual test保split与完整已收两形态、真实Stop/Parts后原socket继续、两新prompt总数恰2；名字带old不是删除该当前防线的依据。
- current host取消父保首Part实际forward后门控、Stop等待/turn失败、精确draft-1/reconnect=false、gone=false、无commit/Unsent、独立ACP JSONL prompt0，再同attachment下一turn/真实Ask+Answer/EndTurn、prompt1和accepted1；没有以EOF或重连代替取消。已commit父保Ask后Stop→Turn(Cancelled)、非SendCancelled、attachment仍活和prompt1。settings/mode/option以及独立ACP能力适配未删。撤掉的只是legacy Welcome改写及旧host矩阵，不另计作绿。
- 新gui_greeting四普通父核真正TCP Hello、当前成功清socket timeout且无terminal帧、明确拒绝键、原拒绝why、silent实际15s。red→candidate测试仅把is_err进一步加强成具体catalog键，原“unsupported greeting accepted”失败断言保留；未把任意dial/IO Err当通过。实际错误cookie服务校验未改，本新fakepeer拒绝case只是保host refusal原why，不单独声称真host错cookie认证矩阵。

## 已核真实证据（尚未最终闭合）

- `current-greeting-red-01`：12文件双盘逐SHA相同，1921 before/after稳；旧生产真实编译成功后exact只跑1普通父，raw明确 `unsupported GUI greeting was accepted: ""`，1failed/3filtered，exit101；这是目标业务红，不把四loop输入或四case文件算四红。run/launcher AST及scriptSHA吻合300/420/5GiB，69.884s、PGID99121退出、scratch absent。
- `current-protocol-green-01`：normalCore check exit0、greeting4及coreio12 raw全绿，两个实际test binary记录稳定；1921 before/after稳。host phase尚在编译时外盘降至5353390080B，资源guard终止，非业务红、无host通过数。原group_absent=false采样须保原值，由后验receipt补足，不能覆盖原final。
- `current-protocol-host-green-02`：before1921与green01逐项相同且after稳定；唯一host续跑E0596如B1，无业务执行。AST/scriptSHA核齐300/420/5GiB，12.011s、PGID8506 group_absent=true、scratch absent。不能把其目录名green当结果。
- 初次审时green01/host02 external/internal尚未逐文件齐，因此未宣称最终镜像闭合。后验四PGID与storage-audit-10补充、candidate02 mut-only指纹及host3真实结果待末次核对追加。

## 适用验收与保留边界

本片最小源码方向符合未上线统一当前Khor决定；无需重新搭协议框架。B1解除后，优先完成当前host3，再核相同源/actualbinary/cleanup与双盘收据即可，不重复已稳16。整体Q033仍待方案中真正quality GUI production api.ts→Tauri native_io→current host→独立ACP log：上传中Stop prompt0、原socket下一turnprompt1、同文新token不串，并保当前权限/settings与已commit Stop。现core host测试没有WebView/Tauri，不能冒充最终链。相邻terminal旧版本协商与外部ACP能力适配分别保既定任务边界，后者不随旧Khor清理删除。

## Candidate02 delta与最终有限收据闭合

已独立核external `current-protocol-candidate02` 的8源、source-freeze与当前树逐SHA一致；相比01只有 `chat_upload_cancel.rs` 一处 `if let Ok(attached)`→`if let Ok(mut attached)`，逐字替换等价，无其它代码/断言变化。新该源SHA `ac3f66e14781e4fa3ddededd9d3eb120f544b52290aefc3db481b115dcde07d5`。B1解除。5生产源、io与greeting测试另2源保持01全字节，已稳normal+16适用于同一生产候选，不要求重复。

`current-protocol-host-green-03` raw编译成功，actual chat_upload_cancel新binary fresh=false，3普通父全部通过（committed Stop、upload Stop同socket继续、residual Parts），exit0/guardnull/PGID12892退出；测试3.25s，phase7.478s，总7.713s。1921输入before/after稳，与green01仅上述fixture mut一个差异。真实current host+独立Python ACP，而不是Promise或注入帧。合计normalCorecheck+19不同普通父（greeting4+io12+host3），不把此前编译失败或资源停当产品红/绿。

最终外内逐文件核齐：candidate02 13file、green01 18file、host02 11file、host03 12file；此前pending镜像现已闭合，原失败/资源记录仍保留。host03 launch AST与脚本SHA吻合STEP300/TOTAL420/MIN5368709120；min外6314811392B/内14754947072B，均≥5GiB。独立只读os.killpg(...,0)核3632/3851/4566/5260/8506/12892六实际phase group均ESRCH，原false采样未改写；三owned scratch实际absent、host03末process清单/cleanup清单均空。

三个实际test binary当前SHA与各原收据再核一致：greeting `08647ae7debcffbf080537a2efcd887a4b21192b4607c1140c8a595b9ffb9c07`，core io `82aba6a43d38e7e60be7be9c95ab0662b6e4cd5c0a96e764bf76c58219eb1450`，host `f5e7b35b456443b715cbb1d8798f50563bd1c5b945a5ae2830b23e7518718723`；manifest均为指定recovery树，非旧App/release或自身Q041e候选。

**本片current协议源码与有限19父证据无剩余阻断，可交root集成；Q033整体仍open，真正GUI→Rust→current host→ACP链和最终集成源验证未完成。** 本审没有运行任何Cargo/native/gen或改动Q033生产源，无main/ship/commit动作。Q041e candidate02独立由root审，本卡不含对自身Gen实现的独审结论。
