# Q055 共享构建缓存的内容来源校验

状态：working，候选实现及纯控已完成，真实编译对照已暴露并在修正准备逻辑；尚未集成或用于整合App。Owner：root（P4）；用户已授权全仓功能与构建治理。main/ship边界不变。

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


## 2026-10-27 root独审与真实对照（进行中）

root实际复现01盲点：包外shared-input内容改变并保mtime，producer key不变、旧生成物仍被接受。02将generated rerun及普通rustc dep-info输入限制为对应producer key闭包/common/extras已覆盖路径；未知输入拒绝，不为README变化全仓失效。root重跑17纯控过。

首次真实小型std两包实测 `Q055/native-provenance-real01`：实际Cargo A build0，verify拒绝hyphen published bin，因为rustc hashed名使用underscore。03仅补Cargo target.name的crate名映射，保完整SHA/size/mtime/唯一配对/rustc.d强校验，新增回归后18纯控过。旧01失败未回写。

`Q055/native-provenance-real02`：A、B的实际编译及原生输出/嵌入root均正确；保旧mtime的新module/API/生成输入变化触发重建；故意把旧A消息标fresh仍被拒绝。同源B refresh=[]、实际3个firstparty artifact均fresh=true且原生输出正确，旧顶层summary.d未代替配对rustc.d。回A实际Cargo0但verify正确拒绝旧B生成输出：prepare只刷新app/src/main.rs，script fresh=true、dep/app fresh=false；prepare误以包内任一buildscript artifact仍匹配旧A就够，未强制生成器再运行。root授权逐target及实际generated-files匹配的最小修复，不放宽verify/不改fixture名/不重跑伪绿。原02结果保留，relocation阶段尚未执行。

两次actual controller已终止、所记录各owned PGID空、TMP清、moduleSHA稳定，总耗时分别1.975/2.792秒。独审另外发现controller的forced-drain、最终PGID/TMP、最终moduleSHA/TOTAL门缺口；下次独立03收紧，历史01/02不改。两次都不是全Q055绿，更不是Khor正常CLI/原生入口或最终App验证。唯一共享target继续使用，不造新target/worktree。


## 真实real03闭环与实现提交

候选04逐target判断对应当前artifact的匹配，custom-build另核其producer key的真实generated inventory/content与output marker内容。脚本字节相同不能代替生成物来源；verify完全未放宽。19纯Python回归全部过，root再次运行确认。

root实际 `Q055/native-provenance-real03` 以收紧后的控制运行，exit0/5.542s：A→B→sameB→returnA→relocatedA 共5次真实Cargo build及5次真实native输出均正确，所有changed producer实际fresh=false，sameB的3个artifact全fresh=true且refresh=[]。保旧mtime的新module/API/生成输入变化没有错复用；旧A消息在B状态被拒；故意陈旧顶层summary.d未代替配对rustc.d；回A触发脚本重新生成old值；relocation实际env!(CARGO_MANIFEST_DIR)为新root。root逐SHA核66dual并独读全部receipt/actualstdout。moduleSHA bef186f76c5184e8a0bf57784b417610817be32acb4a7d673eb75da22c22a9b5稳定，所有owned groups gone/leaderreaped，无forced/cleanup error，TMP清，最终module/TOTAL/镜像门均通过。原01/02失败保留。

作者exact5提交859b9f7d5d962d117f516184dce45050b1ff1896，root集成d49d88c3ec5be3ade695138308f716402d89fed6。这建立可用的第一方来源校验入口，并验证真实小型workspace的正反对照；Khor自身正常CLI/原生入口、CI实际运行及最终App同源接线仍要验，不能把本段标全Q055/P5 done。下一批R1先接当前正常CLI+Node build及9个必要业务父，复用现有process controls而不是复制runner。


## 真正Khor入口接线

R1 run02仅metadata离线解析要求未缓存Android依赖而exit101，0build/0业务父；改用实际rustc host过滤，保resolve/locked/offline后run03 metadata过、CLI build0/73.08s，verify保守拒绝Node未配置的包外compile-time输入。root扫描实际所有workspace rustc.d一次列齐仅providers.json与scripts/install.sh。candidate05只在khor-node extras显式纳这两文件tree(file)及Node/dependents key，verify和未知输入拒绝不放宽，20纯控过。CI/doc host过滤9229→16a86已集成；05 exact2 fc592→0a888已集成。

run04实际normalCLI0/69.34s、Node0/66.30s，并通过完整verify/require来源；zero/slow+close6合计8个业务父过，但R0第一run进度红，整run非绿。源码/entry/module/dist稳、owned组/TMP清，28dual由root独读逐SHA核。证明来源模块已接上真正Khor入口；R1行为红不被误计成来源工具失败，也不标全R1/P5完成。原各setup/verify/产品失败都独立保留。
