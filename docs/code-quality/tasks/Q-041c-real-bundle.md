# Q041c 真实 Pi 制品验证：准备与授权边界

状态：root授权后真实锁定安装、Bun Darwin arm64制品与无凭据smoke已通过；槽已立即归还。下方准备方案为执行前记录，实际结果见末尾；不把计划段落当未执行结论。未运行Cargo/真实provider/main合并发布。

## 现场与来源事实

实际集成树 `/Volumes/disk0/Khor/worktrees/current/khor-code-quality-wave1`，HEAD `76db834bb75aaeb727df571a9dfc580ab65ee31a`，tracked/untracked clean。图优先minimal context仍main8704索引，不覆盖当前候选，回退实际锁/build/providers/host源码。

- package-lock.json SHA256 `9293fb43673dcb0176ccda7d9e20f73361fa4ba4e5876aa3250204d30f353e58`，与main逐SHA相同，v3锁123个包，全部registry.npmjs.org来源，Pi4包精确1.4.0/typebox1.3.7。
- 现集成agent/node_modules是指向main的旧链接，不单凭该链接认定locked。实际hidden .package-lock.json123包与锁version/integrity/resolved全一致、无缺失/多出；123个实际package.json版本无差异。只读npm ls --all --json --offline --logs-max=0 --update-notifier=false实际exit0、无problems，共267展开节点。这证明结构/元数据一致，**不是123包内容逐字节的锁来源证明**。
- main现有node_modules约189MiB，旧dist/khor-agent约73MiB；后者是旧产物，只可作资源估计，不用于候选通过。
- 已有Khor npm-cache约394MiB，命中锁tarball URL仅7/123；只读HOME npm cache命中76/123。不写HOME缓存/配置，不为省空间把未验main内容冒称已认证依赖。
- Node实际v26.4.0，Bun实际1.3.14，本机arm64/16GiB内存。外盘可用6.3GiB，内盘10GiB；本机当前自由页较少。真正构建前再量测，内/外任一低于5GiB不启动；释放唯一槽后串行执行，掉盘/连续3次I/O错误只停自有过程。

源SHA：build.mjs `56d83b008f55d54e50bd8d320b5a8727492040adb16453adfbb3da4f69c7bf1f`；scripts/providers.ts `0fd760085bce0ba84aa3aeea25569055b7c5603363c6493bbaecb7403376faa3`；providers.json `491ef2eb84786d38dbc924f31c906a2623a002e092cb913e38e0a3e3a73b751a`；host.ts `a9f768ba719f564c225011ecc39eda60c58abcab2268fa12ec1e83d36702493d`。

## 建议最省且能证明锁来源的依赖步骤（未执行）

在现Khor caches/dependencies下建立由完整lockSHA标识的私有依赖项目，只复制本树package.json和package-lock.json，npm ci对锁integrity校验registry tarballs。使用已有Khor npm-cache，7份可命中，其余按固定resolved与integrity下载；若仅允许offline则明确失败，不静默改用main modules。新的deps项目约190MiB、tarball增加预计几十MiB。完整manifest记录pkg/lock/Node/npm版本、npm ci命令/exit和source before/after；npm ls再次确认。不得凭目录名跳过fullSHA核验。

锁中仅两个hasInstallScript：@google/genai preinstall为echo no-op，protobufjs postinstall只检查/警告version scheme；源码已读。二者不生成运行代码/二进制，因此此批可用--ignore-scripts，避免执行第三方安装脚本；均registry包，未用git依赖prepare。真实Bun仍需实际编译验证此判断，不把ignore脚本当充分功能证明。

待授权命令草案（占位路径最终在执行收据记录）：

```sh
AGENT_DEPS_DIR=/Volumes/disk0/Khor/caches/dependencies/khor-agent-9293fb43673dcb0176ccda7d9e20f73361fa4ba4e5876aa3250204d30f353e58
mkdir -p "$AGENT_DEPS_DIR"
cp apps/agent/package.json apps/agent/package-lock.json "$AGENT_DEPS_DIR/"
npm ci --prefix "$AGENT_DEPS_DIR" --ignore-scripts --no-audit --no-fund \
 --prefer-offline --cache=/Volumes/disk0/Khor/caches/dependencies/khor-npm-cache \
 --logs-max=0 --update-notifier=false
npm ls --prefix "$AGENT_DEPS_DIR" --all --json --offline --logs-max=0 --update-notifier=false
```

不要覆盖已存在未明来源的依赖目录：若目录存在，先确认owned manifest/fullSHA，决定复用或另行协调。npm ci对该私有node_modules有删除/重建语义，不能将prefix指向主树。root需明确授权后才写cache/网络下载/替换ignored链接。

## 真实构建与制品验收（未执行）

建议先实际匹配本机的aarch64-apple-darwin：能同时测真实Bun bundle和独立执行，不把Linux交叉制品仅file判断冒充运行验证。onset的Linux GNU/musl完整编译及目标机运行仍另批，不以本次Darwin关闭它。

root授权后：记录/保存现agent/node_modules symlink原目标，在无其他agent构建时仅将集成树ignored链接指向已独立npm ci的node_modules，主树及其modules完全不改；完成后恢复原链接或按root长期cache决定，不在恢复时删任何main内容。

build.mjs有argv3正式输出接口，不写共享dist。providers实际生成器仍会写集成树tracked providers.json，所以构建前保存SHA与字节快照、运行后记录SHA/git diff；若出现差异先保留报告给root，不静默以旧文件遮盖生成差异。理论上同锁确定性生成应相同，但目前尚未执行，不能预称一致。

```sh
AGENT_BUNDLE_OUT=/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q041/real-bundle-darwin/khor-agent
node apps/agent/build.mjs aarch64-apple-darwin "$AGENT_BUNDLE_OUT"
file "$AGENT_BUNDLE_OUT"
shasum -a 256 "$AGENT_BUNDLE_OUT"
otool -hv "$AGENT_BUNDLE_OUT"
```

还需记录Bun/Node可执行文件路径及SHA、pkg/lock/source输入清单与before/afterSHA、完整真实build输出/exit、artifact大小/mode/SHA、MachO64 little magic `cffaedfe`与CPU arm64，以及providers before/after。不能仅看进程exit0；产物必须新建在owned空目录、非空且exec。

资源预算为估计：新增deps约190MiB、真实bundle旧参照73MiB、tarballs/Bun临时runtime/诊断余量150–400MiB，总额外约0.4–0.7GiB；Bun峰值RSS先估1–2GiB但未实测。当前仅比5GiB线多1.3GiB，不与native/Cargo并行，必要时root先按manifest清闭合对象。首次执行采集实际峰值/最低free，不能以预算代替护栏。

### 无凭据独立协议smoke

host.ts不是参数型CLI，`--help`没有实现，会等stdin，故不选择它。使用实际MachO子进程、显式不含API/OAuth key的最小环境、owned cwd/TMPDIR，设置单例短fuse与输出上限；不重设HOME，不向真实服务发请求，不执行manage/models（它会refresh provider）。采用三条隔离子进程：

1. stdin EOF：应安静退出，确认无无限后台资源。
2. 输入非法JSON再EOF：应给 `failed/Invalid host protocol message`并exit1，实际所有Pi import与OAuth注册必须先成功。
3. 输入JSON start但protocol=0、缺凭据/空model/text/stateDir再EOF：应给 `failed/Invalid khor agent start request`并exit1；源码guard在mkdir/openModels/runtime/MCP之前，确认真实host协议入口且无模型调用/磁盘身份写入。

每个smoke记录PID/exit/wall time/stdout/stderr（不含任何用户凭据）、expected frame与scratch前后清单。仅杀/回收自有smoke子进程，fuse触发为失败，不把timeout当成功。可选用unsupported provider manage作为后续模块正例，但它会扩大初始化路径，本批无必要。独立bundle运行不依赖node/Bun PATH可用性；使用绝对产物路径且PATH仅指owned空目录/最小系统目录，避免实际退回用户Node脚本。不得用旧73MiB产物冒充candidate smoke。

本卡是准备方案，不是通过证明；依赖安装、真实build、smoke及真实Rust嵌入尚全未执行。root下一授权到来前只读/写本卡，不替换链接、不下载、不占编译槽。


## 实际执行与交付（2026-10-26）

root明确授予唯一Bun/native槽，仅真实Darwin bundle与无凭据smoke，不Cargo。执行前复核外盘6,777,081,856B、内盘11,020,136,448B，均高于5GiB。root随后声明其可能并行写Node link/proto；本次源manifest限定实际agent全部26个tracked输入，HEAD/status另记；实际开始/结束均76db且status空，未用无关全仓变更掩盖agent来源。

### run01 前置错误保留

`Q041/real-bundle-darwin-01` 在npm config阶段exit1：userconfig/globalconfig同指/dev/null被npm拒绝double-loading；未开始依赖安装或Bun compile，不是产品目标红。4源/全部agent输入稳定、链接未改变、无产物/无残留PGID。旧run.py/report/stderr完整保留，不追写为成功。

run02只把npm user/global配置隔离到两个不同owned空文件，恢复使用run01创建且仅含正确pkg/lock两文件的owned SHA缓存；核full锁与package SHA才允许npm ci。未转旧modulesfallback、未写HOME npm配置/缓存。

### 真实 run02

证据根 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q041/real-bundle-darwin-02`。run.py为本次实际orchestrator；所有子进程start_new_session，自有PGID、300s安装/180s构建/5s smoke deadline、每250ms容量采样/5GiB门/3连续I/O错误停止、10MiB输出上限。stderr保留/usr/bin/time -l实际RSS。report有完整命令/exit/PGID/采样/26源SHA与恢复结果，final-observation额外确认所有已观察PID均不再存在。

- npm ci真实exit0、17.184s，123包按锁integrity安装在完整SHA命名私有cache；没有生命周期脚本/audit/fund，Khor npmcache固定，user/global配置为空且不同路径。npm-ls真实exit0，无problems。私有deps约191MiB，Khor npmcache增至420MiB，owned quality-manifest记录锁/pkg/tool来源。
- 仅临时替换集成树ignored agent/node_modules symlink到新locked cache；main及其modules未改。build结束/异常finally按receipt恢复精确原目标 `/Users/zgh/Desktop/working/khor/apps/agent/node_modules`，最终link_restored=true。
- 实际 `node apps/agent/build.mjs aarch64-apple-darwin <privateoutfile>` exit0、0.807s；Bun真实bundle 3084 modules与compile，不是stub。providers生成器也是真实Pi依赖，providers before/after字节完全一致；全部26 agent输入before/after相同，source_stable=true。
- 新候选 `khor-agent`：76,474,082 bytes、mode0755、SHA256 **af9555409de4af93cf166aeedc4b88f19196b5848dc89dd496ea0c650b79d646**。header `cffaedfe0c00000100000000`，file实际Mach-O 64-bit arm64，otool实际MH_MAGIC_64/ARM64/EXECUTE。artifact在新owned证据目录，不使用旧main dist。
- 真实产物三个独立子进程smoke：空stdin EOF exit0/无stdout；非法JSON exit1并输出准确failed/Invalid host protocol message；invalid-start exit1并输出准确failed/Invalid khor agent start request。每项约0.27s/5s fuse内，PATH仅/usr/bin:/bin（无Node/Bun目录），无用户API/OAuth环境、无manage/models刷新、owned cwd保持空。不是Node/Bun解释器回退，也不是模型/provider操作通过证明。

### 实测资源与回收

安装time最大RSS239,173,632B；Bun构建最大RSS786,186,240B（约750MiB）；smoke约133MiB。250ms采样group RSS与time maxRSS分别记录，不混成同一指标。运行循环采样最低外盘6,443,855,872B，纳入结束观测后最低6,443,843,584B；最低内盘11,009,245,184B，高于5GiB。最终新增deps/artifact/cache共约318MiB，低于最初0.4–0.7GiB估计；估计不是实际结果。

所有phase remaining_group_pids=[]，所有observed PIDs最后ps为空，time/Node/Bun/smoke均已结束。先向root立即归还槽，后续仅只读整理；没有Cargo/gen/下一轮Bun。已恢复链接、source/providers不变，已授权cache/artifact/证据保留，不删除主源/用户数据。

限制仍然明确：本次真实Darwin arm64验证已成立；Linux GNU/musl未真实编译或运行，Rust include_bytes嵌入/提取启动与onset全链未跑，不能以本产物关闭这些验收。未调用真实SSH/installer/服务/provider，未合并main或ship。

## 当前f763真实账户UI验收阻断（2026-10-28）

实际正常GUI已嵌入本卡真实bundle，但不能由build成功推出runtime已运行。P5/Q043-pi-owned-run01实际root CUA账户管理Credential home无候选/Connect disabled/document transfer state busy，未达到manage login/secret；native仅read/catalog，runtime目录及khor-agent进程不存在。catalog只是providers本地读取，不能当嵌入启动。原resultad3f9fcff3af1017eb65831976eb211b978891ab091401ef8e3c238c87ae5417/163.913s保nongreen；App65305实际CmdQ return0，业务未达与observer失败回收使原normal_app_exitfalse保持，不回填。独审P5/Q043-pi-run01-review01/report.json SHA2bb4e6d654dfbacee2981f30aa25216a060902c51d4a01ff9b58dfde7dba190d的33项为证据检查，非启动业务测试通过；106PID/34group/TMP与来源/双盘已清核。

实际embedded提取/启动仍unverified，归既有Node打开准入/目录候选下一轮，不为通过而注入manage调用/凭据/OAuth或改生产接口。
