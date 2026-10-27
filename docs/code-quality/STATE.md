# 当前代码治理状态

更新2026-10-28。**目标active，未完成。** 本轮源码实现及Clean Code结构收口已集成；P5真实App验收仍有阻断/未执行项。详细旧记录见[前一检查点](history/STATE-before-q035-q043-closeout-20261028.md)，旧handle都不可重启。继续先读本页与[PLAN](PLAN.md)，再核实际refs/diff/PID。

## 用户边界与唯一执行者

- Clean Code贯穿命名、职责、共享规则、状态/资源owner、副作用与失败语义，[工程约定](ENGINEERING.md)；格式仅机械小项。新问题不能靠改断言或重试称修复。
- 用户明确“开始了的做完，后面只记录不新增；再有问题下轮”。不扩产品实现、模块、工具专项、测试矩阵。已约定验收的最小接线继续，发现独立缺陷只记录。
- 未上线，统一当前协议，不投入旧Khor兼容；认证/epoch/完整性/实际ACP能力保。
- **无main合并/ship授权**，用户App/服务/release不动。
- root独占native/Cargo/App/服务槽。当前所有受控执行均terminal；只有root可启动下一计划。三个agent只静态准备或只读审查，不自行执行。

## 当前源码、产物与环境

集成：`/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，`integrate/code-quality-wave1@495d096d54ea083304031423ca052f87dd9bc75e` trackedclean。full2388输入frame SHA **36f15241f5524d1e900d9b160087eb26b512a5db3f217c0713a869500efaf07e**（含gen157/dist277/模块锁）。生产与21b1一致，56bde仅原030失败diagnostic，495d仅Peer fixture精确Busy→Refused；原testbody不变。

main `/Users/zgh/Desktop/working/khor@8704a503a27b36ad58f922690de50e766f8bd485`不动。协调目录即本页所在plan树，root写中央docs。

**不用编译或重跑115。** 正常dev GUI **f76302af7b61b59c1d01172ee4209c1c9cb6c80bd6381ee760468527624c865d** /374932072B，实际producer `P5/Q033-quality-gui-build-run02` result ec56eb347c058b1a167e5fd8807fdd7cf1a57891cc93cae9e6d3f1019385e2bf；build/artifact/source/cleanup通过，自动Q055 proof因TAURI_CONFIG未覆盖仍nongreen，不能伪造receipt。core-helper-run02完整provenance true：8115c4d2b35760839818aa80e8b0f9b1a8629954689d7423455126e195b12d2b；`debug/deps/chat_upload_cancel-a98ef2c294b70f0c` SHA fa77eac90f78bc7df46506d24a982bda54c42d9aa3051756cf25cd924d8d0552。

五必需path/SHA固定双盘`P5/Q033-real-app-actual-producers01.json`的origins；其旧status/launcher是历史，不作当前状态。normal CLI retained `debug/khor` SHA24b1ce4685b560c6e930ddf1dc974925bda45c061a0a027d549d41aa433e3e91，原receipt22fd...28df7，1953源条目与当前frame匹配；GUI合法重编共享deps后只映射实际exe/stat/源码，不称旧CLI full require仍真。

环境五pin/PATH取`Q039/integration-bootstrap-guard-run01/toolchain-pins.json` SHA6582c4471645016dfe900e561685bf29c3d605d8d9e0f52be8797fbd0cf79533。外层prepend一次，父PATH只去掉恰首个toolbin:；恢复真实RUSTC/RUSTDOC/RUSTUP_TOOLCHAIN/AUTO_INSTALL0，unset TAURI_CONFIG/REMOVE_UNUSED_COMMANDS/KHOR_AGENT_BUNDLE/SIDECAR/test变量。outer原设置CARGO_*。唯一target `/Volumes/disk0/Khor/caches/rust/khor-quality-target`，jobs1/inc0/lockedoffline；原300step720total/双盘>=5368709120B/512/有限owned-PID-PGID/TMP身份与lsof/dual门保，f7/fcb不改。工具链事务已complete，不rustup升级/重做恢复。

## 完成项及当前真实证据

### 代码与既定组合

18c0a9d1明确Pair请求类型/Issuer–Joiner、删废分支共享纯base规则；3e5e25a4共享精确input/sticky与setup终态规则；357c7070让UploadRelay拥有线程/socket/gate并有限清理。均独审，不把fixture整理称整个产品架构重构。d26e608f仅216 Rust机械格式，33c8bc12共同检查main/显式root/默认check/显式write；450第一方check与392第三方SHA稳定。21b1同一root fence覆盖recover/register/bootstrap，保认证/布局/副作用顺序。详AUDIT Clean Code表。

原115父**全部已执行：113过、040 deadline与062 NodeBusy两失败下一轮、0未执行**。40旧结果带独审版本/成功路径映射复用，不冒称在新helper重跑。run08最后50父/53summary全过、85dual、source/owned/TMP清，resultb5043df74c80a22fa4e2be0049939ad891df89590a12ffd3348d33644e825bd5。33c8另外4原大数据父与21b1入口原4父的确切版本/cleanup边界保历史，不重跑充数。

### Q033当前协议真实App：完成，原harness非绿补验单列

run05/source495d/f763真实CUA四PNG共base64 23,048,528B：A首Part→真实Stop/explicit release→原稿四图恢复/capture-a(prompt0)，同稿同图B新token/commit→Allow/EndTurn/capture-b(prompt1)，无附件C→Ask/Stop/新Cancelled(prompt2)，mode bold/model two→原完整raw oracle ack→CUA CmdQ。App45305 return0、helper0/hoststoptrue/connections1/181608ms，未reopen/leave。原result64428402c1b965c35f9dc1b2400063b12040d0adf8e4ed95b1b453cecf38c806/189.108s因Vite受控TERM143被旧launcher0/-15断言漏认而nongreen，原件不改。

独立`P5/Q033-real-app-run05-postreview01/report.json` SHAe6c297f0eebd31d713ceb6e85bf6b9fcff68a7c2152cda0b3cb2dab1c6fd075b，42项原raw/末尾检查实过，132PID/44group全退、TMP清。三主卡已追加最终事实。两旧270s UI操作超时run03/04保历史，勿重跑Q033。

### Q035四原native程序化资源场景：全部已执行，2过2失败

- `Q035-native-owned-run01`/82454，9.481s，result3d01dba87041a73934313f10c9d4585b4571bcfe6859511e2d90fb761364f745：旧Vite cold React/菜单重复入口harness失败，非case。plan03只改server真实main入口守卫/原deps bootstrap，原TSX/断言不改。
- run02/99848，7.734s，result2e65ecd9731e9a8e8af418f19cd90ad1e59c04e2ce6aec27e22088ab285b08c1：late-close真实Some→close→最终null，但父报channel closed，保失败。
- run03/36238，10.988s，result3d13d2221548d86a114d65f6787b195cb69e1da1d8ff09210571bdcbe721cf17：strict-late-close明确过；close-reopen两次open/old-new landing/新owner Some均在，末尾仍channel closed失败，不能回填父绿。
- run04/5115已exit0，43.111s，result**d9f28df0a39f1d887cd6a9b8876ed19dbb5e58a8060c264d36d17f0e1e9e66f7**：只运行最后原native-eight-cap，真实10logical/8native/最旧2null/最终10known全null；root CUA CmdQ App73816 return0/server143、全原source/helper/owned/TMP/dual门绿。**仅该selected run绿，original_suite_green=false**。没有重复已过/已失败case，所有4原case体/时限完整保。

两个channel closed归新page_steps关闭/with_webview oneshot竞态候选下一轮，已有静态源链但raw缺唯一失败invoke/调度时序，不能说已定位唯一race或已修。独审P5/Q035-native-four-case-review01/report.json SHA46be305a9827f2de9972db4aeb7b8a652d5dc5751fe48bab80597243ce98808a，65项证据检查（非业务case）通过；Q035主卡已追加2pass2fail事实。程序化不是生产UI/借网/WebKit heap全清证明。

### Q035生产UI双owner/真实私有双peer：首次setup失败，明确Resume接线待执行

plan01→`Q035-borrow-owned-run01`/54361，11.464s，resultd1d338aaae1214333ee0f7d1ee893a06b6fca1d364494a7a548a94281704abf9：两真实root/私有relay就绪、invite0、pair1精确FinishPending，未进入UI，source/owned/TMP全清。readiness-audit01报告f1a29d71343bd40655ee11e066d4bd5ad8ae42470a1156639b8dcb044184bbe6：该消息来自Pair Finish，非初始Node bootstrap。无外部Finish-ready状态，不伪造轮询。

**下一步root独审已冻结plan02，再唯一槽执行。** `P5/Q035-borrow-owned-plan02` run6dee9203...4c7df逐字同01；launcher e0ababff53483f25964ee590d52a30aca2c25f5ea28c69a659e1c484081351e5；data1516718b9ddc9acfaf5a5155066d092d2f14b97a032b3ed6442322801632be30。仅精确首次FinishPending允许同0600ticket/同两Root一次现有Resume，与首pair共享原70s，原source证明复用saved operation+nonce、首消息Resume、产品原完整journal验证保持；外部只存bytes/SHA/stat，不写状态/新grant/新invite/循环/睡眠/新解析或加密工具。一次Resume再失败就停。完整5phase/native/proxy/Quit/cleanup不变。

两freshRoot首次open前须非空实际relay only_mine配置；App只持APP_ROOT、CLI只serve EXIT_ROOT。新plan不请求web随机口，启动前只读核已有5467 listener且两实例实际busy降级，不能碰用户94762或其他owner。原normalCLI quit仅EXIT_ROOT、keeper143；relay正式TERM0；observer0/-15/143。原285/300/720/5GiB不增。

### Embedded Pi实际UI：受Busy阻塞，未验证

`Q043-pi-owned-run01`/30904，163.913s，resultad3f9fcff3af1017eb65831976eb211b978891ab091401ef8e3c238c87ae5417 nongreen。root CUA开khor配置model disabled；Quota Register三次重开均Credential home无候选/Connect disabled/document transfer state busy。raw read1–10 rejected，catalog11/16成功、12–15 rejected；**零manage/login/secret/reply/OAuth，runtime目录和khor-agent进程均无**，不能把本地catalog称运行。root记录root-ui-abort并CUA CmdQ App65305 return0，业务未达、observer failed-path forced cleanup，原normal_app_exitfalse不改。

独审`P5/Q043-pi-run01-review01` SHA2bb4e6d654dfbacee2981f30aa25216a060902c51d4a01ff9b58dfde7dba190d，33项证据检查（非33业务测试），106PID/34group/TMP清、来源/双盘齐。Busy只据root CUA，不从redacted reject冒称精确诊断。归已有Node打开准入/GUI目录候选下一轮，不新实现或重复尝试。实际embedded startup仍unverified。

### Q043 OS：已静态独审，尚未执行

`P5/Q043-real-app-owned-plan01/os` runner187864963f7d4121558c47d89f90979f78d24a75338b58af30264191fb6e79dd，launcher81eddb6e3becedb12a42d1fc94fa07b50298717943e580783ef791904669d3fb，observera645375c8b81cd031697a3f226e047459a6c39e87ca534f35258f025102c00aa，data61c76112cd4900290bc32797bf997d3fa05efe7fc9d10c4e79cea9f4ae572c96。原helper270/inner285/outer300720/5GiB与所有门保，零新编译。真实Finder→ChatView/terminal drop、false→true→false fullscreen、私有不可写文件修改→窗口Close prevented/Keep→仅修fixture权限/真实save→正常Quit。不能用DOM动作/伪IPC/native事件替代OS动作。旧七注册撤销/title及38组只映射复用，不重跑矩阵。

## UI执行提示（避免再次误launch）

CUA压缩后先rewriteDocumentation。**getApp前必须ps确认ready.app_pid及精确executable仍live**；原harness可能几秒内失败，inventory可能false。getApp会launch，禁止访问已死旧handle。完成后注销仅历史quality App的LaunchServices注册；必要reset CUA，然后绑定新owned精确.app路径，不靠重复bundle id。原run04/035run01工具曾重唤旧private App，均CUA正常退出，不能算原流程正常Quit。

当前CUA只持已退出qcap和任务Finder；新Finder窗口P5（root通过CmdN新建）已用系统Window→Move&Resize→Right放右半屏，截图1280×1410；用户原Finder窗口未关。OS可以把quality App放左半屏，再实际跨窗drag，末尾只关这个任务Finder窗口。别重置用户原窗口/偏好。

Q035生产UI最快：Sessions→khor→再次点当前药丸（Click to focus this session’s tabs），首次自动内容picker；radio Browser→Choose machine→q035-exit→Machine and URL输入readyURL Return，持久owner为content。Machines→q035-exit→Browse→同URL得第二owner；probe-both。切Sessions使MachineWeb卸载→probe-one。**关闭Session content组内URL药丸× aria-label Close或右键Close tab**才真正删持久tab，内容工具条Close tab仅隐藏不可当释放；probe-none。New session tab→Browser/remote/URL→probe-reopen，真关内容tab→probe-final。各marker必须等匹配ack，之后Quit intent/ack/CUA CmdQ/10s内accepted。别关外层khor session。

## 证据、归档与恢复

E `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence`；I `/Users/zgh/Library/Caches/khor-code-quality/recovery`。原source/tool/result/raw证据不回填。所有历史case失败与新独立问题见AUDIT，物理iPhone/IME、Windows runtime/SSH/master/remote lease/Linux等不新增矩阵。

原cache-offload01/02/03恢复ZIP在I根，原stat/SHA见双盘P5/cache-offload0N/compressed-backup.json；旧local/iOS静态档/完成旧test exe路径不可直接用。normal GUI/core/CLI与源码/正常deps保留，禁global clean。

历史quality staged GUI都是f763重复，逐全manifest/实际binarySHA/无lsof后可逆归档到I/q033-historical-staged-gui-f76302af.zip，102641293B/SHA8a9bf6443392b6a2e71faf473c069bcab8d00211349f997faf2ccaba62f56c30，唯一entry khor-gui解压374932072B/f763。精确恢复bytes/mode/mtime/原manifestSHA索引：P5/Q033-historical-staged-gui-archive01(run03/04)、Q033-Q035-staged-gui-archive02(Q033run05/Q035native01)、Q035-staged-gui-archive03(native02/borrow01)、Q043-staged-gui-archive04(Pi01)、Q035-staged-gui-archive05(native03)。恢复exe前原全树manifest不再成立；独审全树检查是归档前事实，inode不恢复。native-run04重复stage已在独审/root核106条全manifest后归档，索引P5/Q035-staged-gui-archive06；原cap结果不变。当前外盘约6.20GB，内盘约8.68GB，每次启动重新核5GiB门。不要凭旧floor盲启动。

内盘恢复RESUME与current-code-quality-docs每次关键事实后镜像；掉盘先读RESUME，逐SHA核再恢复，不能按摘要覆盖未提交源。
