# Q055 共享构建缓存的内容来源校验

状态：ready，未实现。Owner：root（P4）；用户已授权全仓功能与构建治理。main/ship边界不变。

## 已证实的问题

N1 F1 `node-transfer-n1-flow-red-05-control02` 首次编译失败，四个业务父均未运行。新cfgtest有1项已报告的移动错误；其余21项是当前源码已有的同步API被依赖rlib报告不存在。Cargo将khor_sync标为fresh=true、manifest显示当前worktree，但实际dep-info只有旧23个相对源，缺当前outbound/receive/version等模块。作者保存了实际artifact、depfile和诊断；不能把路径metadata或fresh=true作为内容同源证明，也不把本次编译失败计为业务红。

证据：Q039/node-transfer-n1-flow-red-05-control02/compile-failure-analysis.json、actual-sync-depfile.d、final-audit.json及raw.log。具体路径、SHA和双盘收据以这些原文件为准。尚未将底层原因定性为Cargo缺陷。

当前N1与R1 S22冻结输入的非Node第一方依赖差异已定位到sync生产源和catalog两个语言输入；其它crate源码/root Cargo/lock/config相同。短期只强制刷新这些已证实变化的依赖入口/生成输入，要求真实fresh=false、dep-info包含当前模块、生成物内容和源SHA对应；不扫空target，不全仓touch，不以另一棵树结果冒当前绿。

## 完成要求

1. 查清跨worktree共享target时实际fingerprint/输出复用机制，依据现有文件、实际命令与必要的primary文档，区分内容、时间戳、路径和生成输入；不能用推测补原因。
2. 为统一验证入口建立第一方实际产物的来源记录与复用判断，覆盖Rust模块、Cargo/build脚本、生成输入及影响运行的嵌入路径。只重建确有变化或无法证明同源的依赖，继续使用既有唯一target与串行槽。
3. 避免各任务复制修改大段runner控制。把资源门、确切源/产物、失败分类、进程回收和双盘证据维护为可复用入口；接口先审，不引新通用框架或第三方依赖。
4. 用同target的两个实际源状态验证：依赖API/行为不同及生成输入不同均不能错误复用；完全同源可复用。保存实际Cargo消息、产物SHA/dep-info及正反对照。
5. 与最终整合App验证入口对齐；旧收据不回填、缓存合法覆盖/退休单记，不把元数据/类型检查冒同源App实测。

本卡是P4真实新增构建缺口；不阻止已限定依赖重建后的N1功能修复，也不因此扩大重跑或降低当前业务标准。


## 2026-10-27接口与candidate01

node_test_review只读调查已交，root接受prepare_build/verify_build与现有run分离的窄接口，授权在复用terminal树的fix/quality-native-provenance@dd9c45ca隔离分支实现。第一方source名单/SHA、manifest/lock/config、变体/相关环境/嵌入root、实际artifact与配对rustc.d、生成输入及实际产物共同决定复用；变化/无法证明同源才精确刷新对应入口/build/geninputs，第三方与.fingerprint不动。原run/PGID/资源/清理沿既有控制，不复制框架。

primary实现资料：[Cargo fingerprint](https://github.com/rust-lang/cargo/blob/f2d3ce0bd/src/cargo/core/compiler/fingerprint/mod.rs)、[dep-info](https://github.com/rust-lang/cargo/blob/f2d3ce0bd/src/cargo/core/compiler/output_depinfo.rs)、[JSON build-script输出](https://doc.rust-lang.org/cargo/reference/external-tools.html#build-script-output)、[manifest目录环境](https://doc.rust-lang.org/cargo/reference/environment-variables.html)。workspace-relative路径与mtime机制符合现象，但历史缺fingerprint trace，不定性Cargo bug；build-script-executed可为缓存输出，不能单独证明刚执行。

candidate01已交root独审：外盘Q055/native-provenance-candidate01、内盘recovery同名，13文件双盘；5实现路径、1933完整inputs，patch07b1b8bf277b90c04f375d21d682c4abce994ac4a5b6ae5fc38a76dbb084e6e5。作者15纯Python父通过，现有native除main外11函数AST保留，CI同批prepare/verify normalCLI与Node并提供真实worker收据。root尚未源码独审/接受实现；未真实Cargo/native或共享target修改，未commit。

下一步仍按原完成要求：root独审、同一target真实A→B→同源B→A及生成输入/嵌入路径/旧顶层.d对照，再接实际CI/native/最终整合App验证；纯控结果不能替代这些。
