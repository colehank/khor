# Q041c 非作者审查

状态：B1已由root修复，固定候选独立17/17通过；本批受审范围无剩余阻断；root唯一生产所有者，审查者不改4源、不恢复Q041权限。

## 固定范围与来源

作者树 `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-store-owner-recovery`，HEAD `a723bbba67ac8ed3d8b0fbd12bbc69089d5f7871`。tracked dirty为scripts/onset.sh与apps/agent/build.mjs；两个新测试为untracked。开始与独立复跑后SHA一致：

| 文件 | SHA256 |
| --- | --- |
| scripts/onset.sh | 705631328a79232ad3dc6567c67e8f242cba7f333b81f731a4179526747875ab |
| apps/agent/build.mjs | 56d83b008f55d54e50bd8d320b5a8727492040adb16453adfbb3da4f69c7bf1f |
| scripts/tests/test_onset_build.py | 69f2e96193ce0a4df5a495ff69360504c84e1cf478028f7d649f242b2341be55 |
| scripts/tests/test_agent_bundle_build.py | 654aa4d6ca113415a9a62f649ed26e448dbfa21351aca547d6d411383d6cb5a2 |

已读中央Q041c/PLAN开发期当前协议决定。按review-changes技能图优先：minimal context/detect_changes/affected_flows工具实际索引main8704，仅docs/SHIP.md而非作者候选，无权用其低风险结论证明本diff，因此回退实际diff+调用方。未改STATE/全局门/包/锁。

## 初步结论

- 每次onset的mktemp私有STAGE内单独生成Pi组件，非空后以绝对KHOR_AGENT_BUNDLE传给cargo zigbuild --locked。crates/node/build.rs canonicalize、rerun-if-env/changed、include_bytes调用链完整，源码未见丢env或共享dist/khor-agent串target。
- CARGO_TARGET_DIR绝对路径按原义；相对路径按cargo实际cd的REPO解析，BIN定位对应标准Rust target，不把gnu.2.31的glibc selector当产物目录名。
- build.mjs可选outfile按调用cwd resolve，默认仍项目dist。provider失败阻止Bun，Bun非零透传；GNU/musl目标对应传入Rust target。固定官方[Bun1.3.14类型来源](https://raw.githubusercontent.com/oven-sh/bun/bun-v1.3.14/packages/bun-types/bun.d.ts#L2467)明确允许Linux architecture/SIMD/libc顺序；未用最新版docs推断固定版本，不把类型契约当真实交叉编译已验。

## 阻断 B1：serve-up artifact 未进入写入前门（已实际复现）

scripts/onset.sh:299先remote mkdir，309先scp binary（ontarget312先mv），316–317才提取/验证khor-serve-up非空；dry-run291–297先返回，跳过此本地artifact验证。源码条件：install.sh的SERVEUP边界不存在/错误时，live可以先动远端再失败、dryrun成功。本项已报root，root明确接收。以下隔离反例已执行，无作者4源修改，不执行任何SSH remote脚本。

## 独立已执行

```
PYTHONDONTWRITEBYTECODE=1 python3 scripts/tests/test_onset_build.py -v
PYTHONDONTWRITEBYTECODE=1 python3 scripts/tests/test_agent_bundle_build.py -v
```

实际8/8与5/5，共13/13通过，四源SHA前后稳定。测试安全核验：onset的ssh/scp/node/cargo/file全截获在TemporaryDirectory私有repo；真实build.mjs只用当前Node执行stub providers和stub Bun subprocess，未编译实际Pi、未下载、未Cargo、未SSH、未执行installer、未改用户HOME。原有测试不覆盖缺失serve-up或onto-target的实际文件存在性，仅已给出路径字符串约束。

作者原红绿证据已澄清为Q041/c-onset-build-red/test.log、c-onset-build-green-02.log、c-agent-entry-green-01.log（后两者为.log文件）。中央Q041c卡开头尚旧，后续统一由root更新；本review不代写作者任务卡。

本审查不声称真实Bun编译/Rust嵌入执行、目标设备最低glibc运行或真实远端部署验收。完整支持target构建/运行仍后续独立槽。


## B1 实际反例与交还

root已明确授权只在 `Q041/c-review-probes` 写probe/快照，作者4源仍只读。使用实际原 `OnsetBuildTest` 临时repo/tool隔离，不复制onset实现；仅将临时repo的install.sh分别替为缺SERVEUP模板和非空语法错误`if`模板。运行真实onset.sh入口：

| 模板 | dry-run | 实际exit | fake远端写入顺序 |
| --- | --- | --- | --- |
| missing | yes | 0 | 无；错误被漏过 |
| missing | no | 1 | mkdir → binary scp → 才发现缺serve-up |
| malformed | yes | 0 | 无；语法错误被漏过 |
| malformed | no | 79 | mkdir → binary scp → serve-up scp → sh-s（fixture拒绝，不执行） |

这4项是精确的前置缺口复现，不冒充真实SSH发生写入；所有动作由fake工具记账，未执行remote body。root已收到确定阻断与源码写权交还。建议只提取一次到owned STAGE，非空+sh-n都通过后再dry-run返回/remote mkdir，后续传同一份artifact，避免二次读模板改变结果。

证据 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q041/c-review-probes/{probe.py,report.json,source/}`；report有HEAD、4源before/after SHA与4条完整调用日志，固定源稳定。probe脚本只加载实际测试、运行隔离入口，不依赖真实编译/网络。源快照机械复制保留，source目录不是另一个实现分支。

复现命令：

```sh
PYTHONDONTWRITEBYTECODE=1 python3 \
 /Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q041/c-review-probes/probe.py \
 /Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-store-owner-recovery
```

总体建议：B1修复及正式negative回归通过后再交付Q041c；其余受审构建链未发现新的确定阻断。root可增加正式测试写权给审查者，但本轮目前只写本review与获批证据目录，未修改原测试/生产。


## 正式回归准备（尚未执行新测试）

root接收B1后单独授予 `scripts/tests/test_onset_build.py` 写权；其余3源仍root所有。已补missing/malformed × dry/live四个独立negative case，必须本地非零且无mkdir/scp/mv/sh-s，原read-only discovery和mux -O exit可通过；calls在无日志文件时返回空。仅修改临时repo的install模板，从不执行remote body。该测试源固定SHA `07c9dbd6dbae9d194b8814aa3caa39752e625b760976431719f66d985e936291`，已报root冻结。按root要求本阶段未运行/pycompile测试；等待root固定onset候选通知后跑原13+新增4=17全组，不把准备源码当通过。


## B1 固定修复后的独立收口

root冻结新onset SHA `7611d688127fb192705ddfddaadccf9087045221e71281d670f605980f34455b` 后，审查者实际核生产：uninstall独立分支退出后、ARCH discovery前唯一次提取至owned STAGE；非空与sh-n通过才开始discovery/build/dryrun/remote mkdir。后续scp同一份STAGE文件，不再读模板。初次B1已封闭，未通过忽略错误或放宽negative断言遮盖。

获新冻结通知后实际执行完整onset12/12（原8+新增4）与actual build.mjs5/5，共17/17，exit均0；原命令见上，工具执行收据有完整每case结果。4源前后SHA全部稳定：onset7611d688、Mjs56d83b00、onset-test07c9dbd6、entry-test654aa4d6。最终确认与机械source快照在 `Q041/c-review-probes/final/{confirmation.json,source/}`。不伪造未重定向的原始stdout日志，confirmation说明原始执行输出在review工具收据中。

新测试是审查者写的，不冒充它们通过独立作者以外的测试代码复审；root仍应审这4个简单negative的oracle。生产修复与其他既有构建链由本审查独立源审并运行完整组。测试写权现交还root，4源冻结，不再修改。B1旧red probe报告/脚本与旧源快照保留原样，不将其改为green后追写历史；4个新正式negative已确认同样两个dry/two live行为现均本地拒绝无远端写。

推荐：本批可交付/集成，保留真实Pi/Bun编译、Rust嵌入执行与远端端到端部署仍未验的明确限制；不等于Q041/P4所有目标完成。不新增旧端兼容，也不main merge/ship。
