# Q041e：bindings 输出目录最小治理方案

2026-10-27，owner `node_test_review`，只读定界完成，待 root 授实现。基线 wave1 `integrate/code-quality-wave1@686f2e42eb634ff5ba0edc34ab822fe8764f13f2`，开始/结束均 clean。已读 STATE、PLAN、Q041、Q041-fresh-build、Q041-generated-inventory。未 Cargo/native/gen、未下载、未改生产/依赖/输出、未清理任何目录，仅写本卡。

## 当前事实与风险

graph-first minimal-context + 两次 semantic search 已先行：图 build 在 `1d2640d2`，84 files，明确与686f不匹配，gen_bindings无节点；此树无 `.codegraph`。下述依据现场固定源，不把旧图0当覆盖。采用 explore-codebase 技能，图缺口后读实际 generator/调用方及锁定 ts-rs10.1.0 本地源码。

当前 `apps/gui/src/gen` **158 files = catalog.ts 1 + bindings 157**；其中已知旧3仍原样存在：

| 旧输出 | SHA256 | mtime ns |
| --- | --- | ---: |
| PlaceRow.ts | `84a3831296e6cd6b0c1c5f7e4f2b7a85283f23908b95aa86fa7365a9642e3c72` | 1792909225954475541 |
| QuotaWindowKind.ts | `9ccca4c6ab991fa971b511f91ba459e74d3e99987cafa29f76318ea3f03b44dd` | 1792909225966641583 |
| Wheel.ts | `ba33fe4eab59f2724960734ebeddf6c9fd4c260bd06eac6eb6d02ae4e30fd75d` | 1792909225952638999 |

三者均 regular file，gen树无symlink。158个 `{relative_path:content_sha256}` sorted compact JSON 的摘要为 `0ba9c209c59cc7b9862b08083b202094effcf7e2280d1ecc3373fff0069ef083`。未重新运行 producer；“当前正常生成应155总文件/154 bindings”沿用已审 fresh155 原证据与同源对照，不冒充本次生成结果。

真实链路：

1. `apps/gui/package.json:7` 的 gen 先 `mkdir -p src/gen`，`gen_ts > src/gen/catalog.ts`，成功后才 `cargo run --locked … gen_bindings`。predev/prebuild/pretest 调同一 gen；build 随后 tsc/Vite，test 随后 unit/types。直接 `test:types` 不自动生成。
2. generator 依 runtime `CARGO_MANIFEST_DIR` 定默认 `../src/gen/bindings`，显式首参数可任意选输出；当前71个 root `export_all_to(&dir)` 逐个执行、每个递归走 TS dependencies。这个 runtime 路径修复必须保留，不能退回 compile-time checkout。直接 binary 无runtime env仍有 env! fallback，是后续选路径时应明确拒绝的歧义。
3. 锁定 ts-rs10.1.0 `export.rs`：每进程某路径首次调用 `File::create` 覆盖/截断后 write/sync；后续同路径合并当前类型，缓存仅进程内 Mutex。它不列旧目录、不清过期文件、没有跨进程事务。任一后续 export 失败，已覆盖文件留在 live 目录，expect panic 非零也不会还原。显式路径/文件symlink可被 create 跟随。
4. `src/api.ts` 等真实GUI模块从 `./gen/bindings/*` 导入，tsconfig include src。当前源码没有三旧类型的 import/type use，唯一独立 `Wheel` 匹配为term-scroller注释。当前不是已观察到用户功能损坏；条件风险是 reused checkout 中旧模块可满足后来误留 import，而 fresh checkout失败，或中途失败留下混代。
5. quality GUI job `npm test`、release frontend job `npm run build`、Tauri beforeBuildCommand/beforeDevCommand、build-on的export→npm build均进入上述链。build-on共享target并在临时export运行，所以必须维持 runtime crate路径。onset缺fresh GUI producer另归Q041d，不能称已共用。web/build.rs消费的是dist，不直接消费bindings；bindings问题须经tsc/Vite才影响其输入。

## 建议最小落地

不改71根类型/TS形状/crypto/协议，不引入另一份generator或通用事务框架。保一个生产导出函数 `export_bindings(dir) -> Result`，现 root 列表原样移入；在同binary旁加小型managed-output helper负责以下固定流程，依赖用已有std/serde_json/blake3即可，不需新crate/锁文件。

**1. 管理归属先于清理。** `bindings/` 是这个producer的专属目录；`catalog.ts` 在外部，绝不碰 `src/gen` 整树。默认路径须从本次runtime crate定位并规范到已知GUI生成子目录，缺env且无明确arg应报错。显式arg是输出位置，不是删除权限：只接受新/空目录或带本producer有效inventory的既有目录；任意已有非空未管理目录拒绝，不能因basename叫bindings或有.ts就清。检查目标与控制项本身的symlink/特殊文件；默认生成路径下的相对祖先须真实目录，拒绝让src/gen或bindings链接指向其它树。允许用户选择的scratch根先canonicalize，不要因macOS `/var` 等系统路径别名误拒整个TMP。

已有默认canonical目录可一次接管，但必须先核只含已知ts-rs头的regular flat `.ts`，无未知文件/子目录/symlink，保存完整旧name/hash inventory与旧树backup；命中未知项则保留全部并报明确错误。这个特例只归默认专属generated路径，不能扩到显式任意目录。header本身不是通用删除授权。未来清旧backup也只能按本次记录的确切目录/inventory，不扫相似前缀。

**2. 成功生成前完全不改live。** 目标父目录下创建唯一 sibling stage，所有71根和完整依赖只写stage。结束后验证stage仅预期regular `.ts`、相对路径不escape、无重复冲突/未知项；写排序的managed inventory（schema/producer + relative names/content hashes，不写绝对路径/mtime，使跨目录结果可比）。导出或校验失败只清本次stage，live旧TS bytes保持。保当前71根作为唯一root名单，测试不能另抄名单；必要的type输出路径检查从同一名单/TS dependency visitor推导。当前无 `#[ts(export_to=…)]`；未来若支持它，应在export前拒绝absolute/`..`，不能等写出stage后才发现越界。

**3. 最小可恢复发布，不虚称跨平台原子替换非空目录。** 用固定目标旁OS文件锁串行本producer（仓库已使用std `File::lock`），不要只用易遗留的mkdir锁；控制文件不进入bindings TS inventory。stage完整后才发布：旧live rename到确切backup，stage rename到live，确认新live inventory后才把本次成功报告给调用方，再按精确old inventory回收backup。发布第二次rename失败，立即还原backup，返回原错误；rollback失败时保全部目录、报路径/恢复要求，绝不顺手删旧有效输出。

为支持真实中断，只需极小固定transaction记录（target/stage/backup的确切本次标识与old/new inventory，发布前落盘）；下次同锁下先恢复：live缺失且有精确旧backup则恢复旧；完整新live存在则确认或收尾；状态/内容不匹配时保全部并失败。不要按mtime猜最新、无记录扫目录或搭无限generation/GC系统。可将record作为helper本地实现细节，不扩产品接口。SIGKILL中间可能短暂live路径缺失，但旧有效数据在backup，下一gen先恢复；这不是连续读者/断电原子性保证。现npm编排在gen成功后才启动tsc/Vite，可接受这一明确边界。若要求运行中的Vite同时观察始终完整快照，应另行决定平台原子exchange/pointer方案，不能把普通两次rename当已有原子保证。

提交后cleanup失败须显式诊断，保完整新live与old backup并下次可恢复，不得误回滚已发布新代或清未知文件。当前3旧文件的治理效果是新live inventory没有它们；绝不先 `rm -rf bindings` 再跑导出。

这三步是一个最小完整切片：单一root export + stage + 明确owned inventory + 很小publish/recover helper。若首片只做“stage后逐file覆盖+unlink stale”，必须降称生成前失败保护，不能交付为目录失败安全；不建议以此半方案关闭Q041e。

## 适用验证与范围

先在owned scratch取得真实旧行为失败，再改；不对当前wave1的158文件做探针。测试执行实际producer/helper，不复制export算法或grep源码制造绿。

- **核心回归**：同一个实际binary先生成fresh A；复制A到B并加三个真实旧generated fixture（或通用已管理obsolete文件），再次正常生成B。比较A/B的sorted文件名与每个内容hash严格相同，obsolete确实不可导入；catalog及旁邻sentinel bytes不变。预期bindings数量从输出推导，不把154永久硬编码。
- **生成失败保护**：真实export closure在stage写了首文件后返回确定IO/导出错误，live完整name/hash不变；外部public binary亦非零。最好用helper私有test注入一次错误，不加产品环境后门。
- **发布恢复**：在old→backup后、new→live前真实owned worker kill；下次真实gen先恢复且成功输出等于fresh。第二rename错误与rollback错误分别验证旧backup保留、不删第三方；成功后cleanup错误不谎称已回收。几项阶段针对固定两rename流程即可，不泛化成大型故障矩阵。
- **权限与并发**：显式非空用户目录、unknown file、target/child symlink及非regular项均拒绝且目标/链接外sentinel不变；两个真实producer同目标串行或明确Busy，最终只一完整inventory，没有混代/跨worker清stage。
- **正常链路**：root排唯一Cargo/gen槽后，标准 `cargo run --locked … gen_bindings -- <owned target>` 与正常 `npm run gen`/`npm run test:types`，必要一次标准frontend build；freeze generator/root-type/Cargo/锁及实际binary，fresh/reused inventory/source-before-after/cleanup双盘。跨worktree共享target的runtime路径另用两个scratch crate/export上下文验证，不能用env!旧树输出冒绿。

当前没有执行上述红/绿。Windows若未runtime验证就明确未知；不拿macOS成功宣称Windows目录rename/锁行为已验。普通frontend流程验证与Q041d同源producer编排可复用一次，别为本小切片反复全仓构建；本阶段没有占Node唯一Cargo槽。

建议允许修改：generator、小私有helper及对应定向测试；如果inventory文件使package/gate需适配再由root登记，不先碰catalog producer/package/CI/onset。catalog当前shell重定向失败可截断文件是邻近独立风险，本卡明确保留，不把bindings的成功生成承诺偷扩成catalog+bindings整代事务；Q041d后续可统一编排。

## 固定源指纹

| Path | SHA256 |
| --- | --- |
| apps/gui/core/src/bin/gen_bindings.rs | `777ddadb4f9b6ae3a345b056a45f7d5d678d27f21a7851bb526d9c6ddb43a534` |
| apps/gui/core/Cargo.toml | `c604d5a015e5d66b3abbad810f4d666875e974073cb9706501984fba5a7aecc3` |
| apps/gui/package.json | `0b72957dd717ff2943a460ddf64b076840c7878b83601030314838a260904bd4` |
| apps/gui/tsconfig.json | `323b68d69a33bdd18e18a1900ac1b1956660683885258eccb9e8e922d61df887` |
| apps/gui/.gitignore | `55c1fb5f854e906293f6acb44a00a1dd7724ce74bdd76ad803bad184a313e3dc` |
| apps/gui/src/api.ts | `477e50591996a6795fc173b81369494c270382ab2e2eb2252c62ae1b51658c43` |
| .github/workflows/quality.yml | `225c38f9b4b156788273c545293480328be34ad935fbb5e90ccc45cc90ab3a40` |
| .github/workflows/release.yml | `8ef0f61243531036460fae883b3972f91519f9174b538fd307d766dea26e1d08` |
| apps/gui/src-tauri/tauri.conf.json | `755e9fe30cfef0ff199416cd066245f7add9585b3432555ad66cb9199bdded1f` |
| scripts/build-on.sh | `85d5e54a3db2386c659bf5fdc7bf7be66bd7a8b80ac0b6f7d4bbbc24e538d539` |
| scripts/onset.sh | `7611d688127fb192705ddfddaadccf9087045221e71281d670f605980f34455b` |
| Cargo.lock | `4436a386ad74fdceec35dbfcb67ecbb32d593bfff56badfc92e9d5f5437bf9e6` |

本地锁定库 `ts-rs-10.1.0/src/export.rs` SHA `3c372c2fb0d3bc217053c3e777b5b5afe3d5a693f05087eb5ca780d27e6b0070`，`src/lib.rs` SHA `1fb682ddf29fc5b105bac97936d7b5791ee1b5e3dc52d1832e77a75bc6e87923`。不是从互联网新取API，不调用下载器。现有旧输出原bytes/mtime与审计证据全部保留；P4方案尚未实现/关闭。
