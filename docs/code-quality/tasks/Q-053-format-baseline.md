# Q053 格式规范基线与独立迁移批次

状态：baseline recorded，未实施格式迁移；root，2026-10-27。用户原目标P4包含统一可重复的代码风格，格式迁移与功能变化分批。

在治理集成670347d7 clean上实际执行 `cargo fmt --all -- --check`，只读，8.075s，exit1：205个第一方Rust文件有差异（crates175、apps30；无vendor），完整原日志1,935,425B。1923 tracked输入前后SHA一致，worktree仍clean；未格式化、未编译、未改任何源。实际工具 `rustfmt 1.9.0-stable (59807616e1 2026-04-14)`。双盘证据 `P4/format-baseline-670347d7` 含raw/result/source。这是原有风格差异，不当功能回归或已通过CI。

查已读Cargo.toml、GUI package.json及quality workflow未找到统一fmt/lint入口；全仓非vendor/非node_modules专用rustfmt/prettier/biome/eslint配置检索无命中。这个检索范围不意味着设计token/行为门不存在；后续不得以普通formatter替代它们。

后续在N1/R1/Q052功能改动收口集成后刷新清单，以Rust官方fmt/default edition workspace为统一机械入口；纯格式单独commit，不与逻辑修复混批，不动vendor/生成器输出归属。格式检查固定入口/实际工具版本进入工程指南和适用CI；不能直接开门让现205个既有差异阻塞无关任务，也不能用忽略全部源解决。前端格式按既有编码风格核清实际工具后定界，新增工具/锁变更另记来源，不凭关键词直接塞依赖。

完成标准：当前完整第一方格式检查通过；门与文档命令一致；生成代码从producer格式策略解决；适用类型/构建/既有行为门在最终整合版本通过。函数长度、注释数量不是成绩；保必要平台/协议/并发约束，失效历史说明迁决策记录。


## 既定P4收口前只读准备（source3a0104a9）

2026-10-27，非作者shutdown_owner graph-first后核实际源码，未运行Cargo/rustfmt/native、未写源。14个workspace成员中ACP为2021，其余继承2024；格式迁移和检查须保持manifest edition。为严格排vendor，使用14个显式包选择，不将根--all当全部第一方或排path补丁的证明。独立Windows harness另有workspace；还有terminal-color-answers、editor-history/native-menu两个probe lib模板、native-io/native-paste两个fixture共五份standalone/template，须在实施时验证模板处理，避免递归找不存在模块。native-events/tauri-2.11.5/event-mod.rs为第三方原文SHA契约，明确排除。

Rust OUT_DIR输出归catalog/detect/web build.rs，TS归catalog gen_ts与gui-core gen_bindings；只格式化producer，不手改生成结果。最小接线复用quality.yml scripts job和现README/KHOR规范入口，无新job/matrix/依赖。当前447份tracked非vendor Rust含第三方原文，N2集成后刷新，旧205差异不冒当前数量。先N2集成、再纯格式批、共同检查、最终一次整合验收。模板处理尚未实测；不借此开新功能专项。稳定工程约定已提取到[ENGINEERING](../ENGINEERING.md)，PLAN仅指向同一出处。


scope只读准备产物已独审后保存双镜像：外`P4/Q053-format-scope-preparation/manifest.json`/内recovery/P4同名，150402B，SHA3e161d686cc418331346c97dd887f959eab1921538e8d601fed6fc6f39f8ea66，root核双盘逐字节一致。基于integration3a0104a9：838 tracked Rust=root包440+独立harness1+五standalone/template+vendor391+Tauri第三方SHA快照1，第一方446，未归类0。清单含14实际package/edition、逐源SHA/模板前置/producer；只是准备，未执行formatter/Cargo/SDK产物，N2新文件集成后刷新，不能把此数量直接冒当前最终覆盖。


模板处理只读核实：已查Rust 1.95.0自带[rustfmt formatting.rs](https://github.com/rust-lang/rust/blob/1.95.0/src/tools/rustfmt/src/formatting.rs#L124)与[modules.rs](https://github.com/rust-lang/rust/blob/1.95.0/src/tools/rustfmt/src/modules.rs#L120)，stdin输入明确关闭out-of-line module递归；官方[配置说明](https://github.com/rust-lang/rustfmt#rusts-editions)区分parser edition和style edition。后续可以逐文件stdin/stdout按manifest明确两edition，覆盖第一方独立模板而不解析其尚未生成模块、不借nightly skip_children/ignore。这里只确认上游代码行为，尚未实测本地格式器或更改迁移入口；实施时以固定1.95真实工具做一次实际模板/完整清单检查，第三方SHA仍核前后不变。没有新增Rustup组件、依赖或源改动。


## 2026-10-28 实施与独立审查结果

N2已14931整合后，实际范围841 Rust=449第一方+392第三方。使用原1.95.0真实rustfmt1.9.0（显式RUSTFMT、禁代理安装）与draft02共同入口：check前216差异→write216→check后0，全部392第三方SHA稳；9文件dual P4/format-migration-14931。非作者node_test_review独立逐SHA核全部after、216before与14931 blob精确匹配、216路径即Git rs diff，未见混逻辑/输出。root只暂存216 rs，单独提交d26e608fbae7ec9915827fd3d0b906b5b22ffa3f（机械排版，不代替Clean Code）。

用户随后重申Clean Code关键；现有功能边界内部整理单独18c0a9d1/3e5e25a4/357c7070提交，详AUDIT收口表，不混纯格式。共同入口也由node_test_review按原执行序列仅改main/显式root，逆变换能byteexact恢复draft02；带--write的受限import探针禁所有工具/文件写仍成功、0副作用。脚本SHA bd9e0b8f84d2a401671373e8a218c527b9f8ecaef6c9673b0e09841c3afa893a。

新增private agent_setup使最终范围842 Rust=450第一方+392排除。所有writer交接后仅一次共同write(2处内部新增排版)→check0，dual P4/clean-code-final-format，第三方仍全稳，未运行Cargo代替业务验收。root将脚本/config/CI/两文档入口及统一ENGINEERING指南单独33c8bc1254d26335ddc5ea409d215a8b426bd30a提交；本地与现quality scripts job共用，job显式Rust1.95.0/rustfmt组件，无新增job/matrix/依赖。dtolnay @版本的选择语义已核其官方README。Windows只做关闭文件后replace/chmod路径兼容设计，未实跑不冒已测。

格式/入口本身已收口；整体P4仍按ENGINEERING结构和行为准则、最终组合/App证据核定，不以文件/行/注释数量计完成。当前集成33c8 clean，原npm构建已0（TMP Node缓存原门非绿另postcleanup），后续native正在一次最终网络边界，不因格式重复作者树LTO。

## 2026-10-28 当前收口补记：格式与结构成果分别记账

本节更新本卡开头的历史状态，不覆盖原始基线或失败记录。格式迁移d26e608f与共同入口33c8bc12已经实施、独审并集成；450第一方check0、392第三方SHA稳定及受限import零副作用的证据仍为上节确切版本结果。本次未重复执行formatter或CI，也不将这些结果改称495d头上新跑过。

Clean Code的独立结构成果是18c0a9d1的Pair请求/Issuer–Joiner角色、废分支删除与固定base纯规则，3e5e25a4的input/sticky及setup终态共享规则，357c7070的UploadRelay线程/socket/gate资源所有者；后者属于验收fixture。规则、状态/资源归属与失败语义的整理不以216文件排版替代，也不据此声称全仓完成深层重构。约束与具体边界见[ENGINEERING](../ENGINEERING.md)及[AUDIT](../AUDIT.md)。

当前集成495d096d的既定115父已全部执行或有明确复用映射：113通过、040/062两失败保留下一轮，未执行0；不是整体验收全绿。最后run08的50父/53成功summary、85双盘文件、2388输入before=after=current与owned/TMP清理已独核，完整记录见[STATE](../STATE.md)。正常GUI构建、真实App用户流程及退出仍在既定验收中；格式/入口本项完成不代替它们，也不将计划或旧二进制计作当前App实证。
