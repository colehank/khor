# Q-001C：验证、工具链与发布审计报告

审计日期：2026-10-25。负责人：validation_audit。固定源码基线：`8704a503a27b36ad58f922690de50e766f8bd485`，`/Users/zgh/Desktop/working/khor` main，检查前后 `git status --short` 为空。中央文档为本报告和 Q-001C.md；尚未审查 root 的修复差异。

## 审查边界与方法

先读中央 STATE / PLAN / Q-001C 和 AGENTS；get_minimal_context 确认图与基线一致（741 files / 12481 nodes）；脚本关键词及 CommitPage、useChatScroll tests_for 未命中，随后用 rg、行号阅读与清单扫描补齐。根目录没有 `.codegraph/`，未建立索引。

“结构审查”表示核对入口、环境、调用边界和代表性断言，不表示每个文件或每条测试完整深审。数量来自基线 git ls-files；不会把测试属性数当作覆盖率。没有执行卸载、部署、发布、网络 Lab、原生 App 或用户服务。

| 模块 | 清单与审查状态 | 核实的入口 / 关键流程 | 尚缺范围 |
|---|---|---|---|
| GUI 检查脚本 | 133 个 tracked 文件，97 个 `.mjs`（90 个顶层）；结构审查完成，重点场景深读 | package.json、check-gates、face commit/save/keyboard、ai-elements、native/mobile/wk、editor-history、scroll-surfaces、session-navigation-ui | 其余领域门仅入口/语法及代表性抽查；不能称全部断言已深审 |
| GUI Lab | 131 个 tracked 文件，18 个 `.mjs`；清单/结构审查完成 | commit.tsx生产组件；gui-session-layout、lab-scroll、session-navigation；默认 tsconfig不纳入lab | 大部分实验原型未逐页运行；新旧API兼容及可访问清单需后续维护 |
| 根 scripts | 7 个 shell；语法检查完成，安装/远端构建/netlab主要分支已读 | build-on/onset/onset-lib/install/mirror-sync/netlab；relay-https入口与对应Rust测试 | relay-https全部运维分支未深审；未做真实远端操作 |
| CI | 2 个workflow，完整入口审查 | codex-adaptor（限定路径PR）；release（tag/dispatch，frontend→build→publish） | GitHub仓库外部branch protection、人工审批与外部CI不在本地证据中 |
| 构建/格式 | GUI/agent/site package、TS config、Cargo workspace、Tauri config、node/web build入口审查 | 锁文件存在；GUI gen→tsc→vite；Pi Bun compile；CLI内嵌资源 | 未重建完整产物，未验证跨机器字节一致；无已跟踪通用formatter/linter配置 |
| 全仓测试入口 | 14 个Cargo workspace member；agent 8个test文件；site没有test入口 | GUI29个默认脚本；agent tsc+node --test；Cargo各crate单测/集成；CLI serve_up/relay_https/netlab | Rust测试具体语义归Q-001B；本审计没有运行cargo workspace test |

## 既有验证实际覆盖

| 入口 | 实际验证 | 不代表什么 |
|---|---|---|
| `npm test`（apps/gui） | package.json:12–13：先cargo生成catalog/bindings，再执行29个脚本，末尾tsc；包含source约束、内存bundle模型、Chrome select/scroll-target两类浏览器检查 | 不运行face、ai-elements、gates、mobile、wk、native、session-navigation-ui；不能据此宣称完整交互/原生通过 |
| `npm run gates` | check-gates.mjs:61、75–80、99、123：顶层mjs与npm入口、同层双引号from导入、文档名称substring可达性 | 不执行门；不核对断言质量；不遍历lab或scripts子目录；不证明文档描述正确 |
| `npm run face` / `FACE_ONLY=ws npm run face` | check-face.mjs:99–115系统Chrome、production App、API route fixture；30391–31419工作区分组，30740–31040提交历史/提交diff/跳转/capped | 不是WKWebView/iOS；窄viewport并非真实触摸；不是真实后端/git协议端到端 |
| `npm run ai-elements` | check-ai-elements.mjs:113–115四种宽度/主题；277–309真实ChatView披露锚点、动画流式增高尾部、历史上方plan重排；还含权限/工具/思考/plan | 无commit场景；没有键盘或viewport高度收缩；API仍为fixture |
| face `bar k` | 32530–32606 fake visualViewport resize；量root、bottom bar、内容底边、xterm行数及PTY resize | 未量ChatView最后一条消息；浏览器事件模拟不是iOS键盘/焦点/pan |
| `lab/check-gui-session-layout.mjs` | 14–32带keyboard query重新加载到固定状态，检查composer与viewport边界；有多主题/宽度 | 不证明已有长聊天处于tail-follow时，键盘展开过程仍保留尾部 |
| `npm run mobile` | check-mobile.mjs:46–48非macOS退出0；65–73前置检查；221–250 sync census；312–325 cargo iOS target check | 不启动iOS App，不触发软键盘，编译通过不能证明滚动行为；本次中断且静态已失败 |
| `npm run wk` | check-wk.mjs:22–33裸WKWebView+shim probe，含PDF/排版/overscroll；62–64非macOSskip exit0 | 注释明确不是Tauri自己的webview；无本次ChatView/CommitPage操作 |
| `npm run native` | check-native.mjs:15–57 scratch KHOR_HOME/tmux，真实macOS binary，主题/窗口/menu/keys | 不验证iOS软键盘；86–103选择最新mtime binary，旧于dist只打印note，不能自动证明binary来自待审HEAD |
| `npm run smoke` | smoke.mjs:1–25真实隔离节点+dev bridge+系统Chrome，网络/会话/同步流程；9037行单文件 | 不是Tauri/WebKit/iOS；没有为本次两bug运行，不可沿用旧结论 |
| `npm test`（apps/agent） | package.json:9–10 tsc + node --test test/*.test.ts，8个文件 | GUI默认命令不会代跑；CI release只build，不调用test |
| CLI shell相关Rust测试 | crates/cli/tests/serve_up.rs:112–123提取onset --print-serve-up，152–162检测owner store；relay_https.rs:12–15 scratch root/mock acme+systemctl，真实loopback relay | 现有覆盖不能证明onset --uninstall/--purge/--dry-run组合；未发现这些组合回归 |

默认29项（来自package.json:13）：shim、boot、px、icons、replies、keys、dial、tokens、tap、links、select、snapshot、peek、iip、prompt-sp、indent、revert、type、words、session-cwd、scroll-target、launch-paths、session-read、terminal-colors、terminal-drop、scroll-health、deck-scrub、file-deck、field-interaction。不得把session-read/scroll-health的名称误读为聊天viewport回归：前者主要读回执/attention；后者是终端滚动监测时钟和监听生命周期。

## Q-012：两项用户故障漏检结论

### 提交文件跳转

已存在真实点击后的几何断言。check-face.mjs:30936–30954打开菜单、点击第三文件、等300ms，检查scrollTop非0、file bar在viewport内，顶部对齐或scroll到底。1400×900与390×844各一次（30855–30858）。不能将漏检解释为“旧测试只有静态检查”或“没有等菜单关闭”。

尚未运行证实的缺口：

1. 只在一个延迟点采样；没有等待菜单完全detach、焦点稳定、diff layout稳定后连续保持断言。若事件更晚发生，300ms读数仍可通过。
2. fixture只有三个文件，先展开第一文件尾部60行，再跳第三文件（30761–30803、30912–30939）；没有多文件大commit、不同目标、重复跳转、键盘选择或触摸选择。
3. face只启动系统Chrome（114）；390宽是在同page setViewportSize，不等于iOS WebKit。若焦点/scroll anchoring是引擎特有行为，Chrome fixture不足以检出。
4. npm test不调用face；单次“默认测试绿”压根未执行这条已有检查。

root正在独立复现。上述是覆盖缺口，不是已确认根因。新回归建议：复用lab/commit.tsx生产CommitPage，真实菜单鼠标/键盘/触摸入口；选择前确保目标在屏外；菜单退出与多轮layout完成后连续采样target位置/scrollTop/activeElement；覆盖前、中、末文件与连续来回；让原始基线以同断言失败。不要用测试locator.focus/scrollIntoViewIfNeeded作为最终断言前步骤，因为它可能替产品滚动。

### 键盘与最新消息

已覆盖ChatView内容增高尾部跟随和历史锚点；未找到同一活动ChatView在“已有长内容、尾部跟随状态→键盘逐步缩短viewport”的断言。face bar k虽模拟viewport，但观察的是终端行数与shell几何；Lab keyboard场景重载到最终几何。三条证据组合仍不能证明聊天阅读模式经过键盘动画的转换正确。

新回归应分别检查：tail-follow时逐帧缩短/恢复viewport；同动作下历史段落仍在原阅读位置；composer增高、streaming同时发生；关闭/再开键盘；切会话时恢复对应状态。记下浏览器模拟、Playwright WebKit与真实iOS三个不同层级。真实软键盘至少需要最终输入焦点、可视尾部、阅读锚点、连续展开收起操作的记录。

### 同类fixture契约问题（Q-001A协同发现）

GUI审计者已复现save-road内容digest循环：内容A→B→C→A后保存D出现虚假冲突。这里仅记录测试漏检证据，业务复现归Q-001A。

check-face.mjs:31429–31433声称fixture维持真实compare-and-write契约，确实31504校验was；但31479–31482 `digestOf()`是递增序号、31508每次write赋新值，内容回到A时不可能复用A的digest。因此save-c的“undo survives save”不能检出内容哈希环。check-editor-history.mjs:50用文本hex前64字符作digest，56写入不校验was；66–85验证undo/redo/remount，不断言循环后保存成功。建议对生产内容hash语义建一个真正一致的临时disk fixture，并加A→B→C→A→D行为回归；无需依赖源码里某个函数名。

## 风险清单

### V-01 高：共享home purge越过本机store归属（源码确认，未执行）

位置：scripts/onset.sh:141–168，尤其165；scripts/onset-lib.sh:39–61。

触发：两台机器共享HOME，HOME/.khor/owner属于A，在B运行`onset.sh B --uninstall --purge`。普通停止使用`khor_root`（149）选择B的独立store；purge却无条件删除HOME/.khor及HOME/.khor-hosts/B。后果：同时删除A的身份/数据，波及共享集群，无法靠二进制回退恢复。

建议：卸载前解析并固定本机root，purge只删除授权store路径，验证路径/owner；不要在移除binary后重新推断一个不同root。验证思路仅用scratch目录：构造A owner和B store两个哨兵，mock hostname和已安装khor root，截获或在scratch内执行生成的purge片段；断言B被删、A保持、未知owner安全失败。**不得调用真实卸载入口或SSH**。

### V-02 高：`--dry-run --uninstall`实际执行删除（源码确认，未执行）

位置：scripts/onset.sh:64–74、141–170、277–279。DRY=yes被接受，但uninstall分支在DRY判断之前调用remote off/purge并exit；所以加--dry-run不会抑制卸载/清除。

建议：在任何remote mutation之前统一处理动作计划/dry-run；组合冲突明确报错。scratch测试mock remote函数只记录脚本，覆盖install/uninstall/purge与dry组合，要求dry的删除/服务操作计数为0。本轮不实际调用卸载。

### V-03 高：网络Lab失败被wrapper吞掉，返回成功

位置：scripts/netlab.sh:76、80、85–87。cargo编译和测试均`|| true`；末尾grep也`|| true`。编译失败、测试FAILED甚至无结果时，只要执行到末尾，脚本可返回0。非Linux经ssh exec也会传播这个假成功。

建议：保留cargo退出状态，输出报告后返回原失败；对0个结果增加前置/后置检查。用fake cargo输出编译失败/部分测试失败/空结果/全绿四种日志，验证wrapper退出码与汇总，不启动namespace或远端节点。

### V-04 中高：默认/CI入口缺少GUI真实行为回归

位置：apps/gui/package.json:12–13、41、93；.github/workflows/codex-adaptor.yml:4–10、28；release.yml:57–60、125–135。

仅两个本地workflow：PR自动检查限定Codex adaptor路径与Cargo manifest/lock，测试只跑adaptor::codex；release构建frontend/agent/CLI，无GUI门或agent test。改变聊天/菜单等GUI业务后，仓库可见自动入口不会执行face/ai-elements；默认npm test同样漏过。此项是覆盖风险，不推断GitHub未提供的外部CI或branch protection。

建议：定义稳定且可选择的最小UI集合，与默认模型/类型检查分层；PR按影响范围调用；原生验收另出带版本证据。不要把所有40k行face塞进默认测试作为唯一解。

### V-05 中：测试夹具/时序和超大共享过程扩大误绿/误红

位置：check-face.mjs共40684行；99–115单browser/page，30391工作区组入口，30940固定300ms；31479序号digest。smoke.mjs共9037行。FACE_ONLY已有分组，不应重复发明完全新入口体系。

风险：共享route/localStorage/focus状态在组间残留；偶发时序失败归因成本高；固定fixture掩盖后端契约；大文件难以独立运行一个commit场景。建议从本轮commit/chat开始按功能拆成可单独调用模块，公共server/fixture/cleanup统一；保留生产组件与可观测行为断言；每个输出注明场景、版本、engine、结果及skip。

### V-06 中：mobile census已漂移，非默认门静态失败长期不可见

位置：check-mobile.mjs:157–218名单、221–250普查、312–325无条件继续cargo（只看dead，不看bad）。本次实际出现10条静态失败，详情见运行记录。不能由“未列QUICK”直接断言每个函数都阻塞主线程，需要Q-001A/B核对调用链。

建议：先拆/暴露静态与编译入口，static失败时减少不必要昂贵编译；名单增删按实际等待语义判断；针对聊天控制/文件handoff关键命令补时间与取消行为验证，而非只把名称塞进白名单。

### V-07 中：版本与依赖来源不足以复现构建/原生检查

位置：release.yml:48/101 stable Rust、54 Node22、109 Node22.19.0、112 Bun1.3.14、123–124无固定版本的ziglang/cargo-zigbuild；132–134 cargo无--locked。build-on.sh:105–123 git archive HEAD后symlink工作目录node_modules；135–137 agent也复用node_modules；162 cargo无--locked。check-native.mjs:86–103按mtime挑binary、旧于dist仅note。

已有积极约束：npm ci、Cargo/npm lock存在，release前端只构建一次并复用于矩阵，发布产物生成sha256；build-on确实导出HEAD并报告dirty，不会把未提交源码打进去。缺口是lockfile未验证实际复用node_modules、工具链会随时间漂移、native binary未绑定待审源码。因此相同HEAD仍可能用另一套依赖/编译器或旧binary。

建议：在隔离目录按lock安装依赖，固定/记录Rust与zig构建工具版本，Cargo使用--locked；证据manifest记录HEAD、dirty diff hash、lock hashes、node/rust/bun/browser、binary/dist hashes。原生测试在来源不符时明确失败或标注只能验旧产物，不能默认为本轮通过。

### V-08 中：下载完整性与镜像版本切换不一致

位置：install.sh:54–62、81–104只校验gzip及magic，未消费release .sha256；onset.sh:236–244已经校验sha256；mirror-sync.sh:20读取latest tag，26刷新main installer，32再次取latest资产，34–37逐文件替换最后写VERSION。

触发：镜像更新过程中客户端读取，或两次latest请求之间发布新tag；版本标识、installer、不同平台资产可能来自不同提交/发布。单文件tmp+mv避免半写文件，但不能原子切换整套release。建议下载同一release对象对应tag的资产并核checksum，版本目录完成后切换索引；install与onset共用核验契约。增加中途中断/换tag/错误checksum的scratch HTTP fixture。属于完整性/可复现风险，本次未观察真实损坏下载。

### V-09 中低：Lab与格式约定缺少维护边界

位置：apps/gui/tsconfig.json:20只include src；vite.config.ts:25–27明确Lab可引用退休API并从生产依赖优化排除；check-gates:61不遍历Lab；lab/check-gui-session-lab-scroll.mjs:4写死/Volumes/disk0输出目录；无已跟踪的rustfmt/prettier/eslint/biome/editorconfig/rust-toolchain文件或统一format/check脚本。

建议：标识production fixture与historical prototype，给前者独立tsconfig/可运行索引与env可覆写输出目录；已有四个Lab tsconfig可归一。格式规范先确定后单独迁移，不能把现有风格差异直接判为功能bug。不要删历史原型来“消灭类型错误”。

## 实际运行记录

环境：macOS本机；Node v26.4.0，npm11.17.0。所有命令针对固定基线，未执行npm pretest/gen或构建发布。

| 检查 | 结果 | 证据与限制 |
|---|---|---|
| git rev-parse / status | 基线匹配，clean | 检查前后主仓无源文件差异 |
| 115个GUI scripts/lab .mjs逐文件 `node --check` | 通过 | 只验证语法，不验证import可解析或行为 |
| 7个根script `sh -n` | 通过 | shell语法；relay-https另跑bash -n通过，尚未使用shellcheck |
| `node apps/gui/scripts/check-gates.mjs` | 通过 | 输出90 scripts /83 entries；仅其自声明入口范围 |
| `node apps/gui/scripts/check-mobile.mjs` | **静态失败，整体中断** | 审计者误判为静态脚本；输出10失败后触发cargo iOS check，随即终止自己进程树；不计编译通过/失败，不计原生验收 |

mobile的10条输出：

- sync但不在QUICK：chat_control、chat_respond、file_handoff_cancel、file_handoff_start、file_handoff_state、file_handoff_supported、file_preview_supported、term_ready。
- QUICK陈旧：reveal、show_menu已不是sync command。

已终止审计者自身进程：node check-mobile PID72262、cargo check PID72610及当时子树72654/76036/76037/76038/76039/76040/76055/76375/76485/77632/78067/78068；未接触用户服务。该检查短暂写了已有target目录的编译中间产物；未完成原生构建，没有源代码差异。后续执行脚本先核完整调用链，重编译交root串行调度。

## 首轮可执行检查组合（建议，未声称已运行）

在集成worktree `/Volumes/disk0/khor-code-quality-wave1`执行，由root统一记录确切diff、环境和产物：

1. 轻量入口与类型：`cd apps/gui && node scripts/check-gates.mjs && ./node_modules/.bin/tsc --noEmit`。如果generated TS缺失/过期，先由root调度`npm run gen`，不可在主仓偷偷补生成后声称只读检查。
2. 原有默认集合：`cd apps/gui && npm test`。含cargo gen，明确由root调度；保留每个已有失败，不通过删白名单或跳过归零。
3. 相关生产交互：`cd apps/gui && AI_ELEMENTS_PORT=23872 AI_ELEMENTS_OUT=/Volumes/disk0/khor-code-quality-wave1-proof/ai-elements npm run ai-elements`；`FACE_PORT=23873 FACE_ONLY=ws npm run face`。均自启独立Vite，选端口前确认空闲，串行运行，确保目录归本轮。
4. 新回归：`cd apps/gui && QUALITY_ENGINES=chrome,webkit npm run scroll-lifecycle`，独立端口默认为23821，可设QUALITY_OUT/QUALITY_PORT，WebKit需安装对应engine。本任务已用同一脚本完成原始业务文件红、第四版绿；实际命令与指纹见[Q-012.md](Q-012.md)。root已注册`npm run test:interaction`串行ai-elements与scroll-lifecycle，默认engine仍为Chrome。
5. 原生与集成：root先构建明确版本，再跑隔离native检查/真实iOS键盘与CommitPage验收；浏览器模拟不能替代这步。`npm run mobile`目前已有10项静态失败，编译单独调度并记录现存失败归属。
6. 扩展后端入口：若改动触达协议/原生，按Q-001A/B影响运行对应cargo tests；agent变化再`npm --prefix apps/agent test`。本轮前端最小修复无需为“全绿”自动执行所有网络/部署脚本。

实际可用的独立Lab：`apps/gui/lab/commit.html`直接渲染生产CommitPage；Chat用现有production App/ChatView fixture。新验收必须保留真实焦点、动画与异步布局，且按钮点击后不再由测试主动调整滚动。

## 交接

已确认：入口分层、两故障已有覆盖与缺口、上述脚本/CI/构建风险、语法/gates运行结果、mobile中断及10条现存静态失败。

后续更新：Q-010/Q-011原始重现、第四版修复效果与独立审查已在[Q-012.md](Q-012.md)完成。仍不由本审计确认的范围：完整默认npm test与所有非默认门、真实iOS/WKWebView/辅助技术行为、跨机器bit-for-bit构建；对应结果由root和后续任务分别记录。其他新发现仍为审计项，不扩张本轮实现范围。没有合并、提交、发布动作。


### root复现进展补充（作者通报，尚未独立验证）

root已捕获真实CommitPage第6文件点击后scrollTop3348，约125ms恢复focus到顶部trigger并scrollTop0（/Volumes/disk0/khor-code-quality-evidence/commit-baseline.json、.png）。这早于旧face300ms断言，故“等待不足”目前只能作为一般缺口，不能当作本次漏检定论；需要旧fixture运行结果确认是fixture未触发还是旧门根本未执行/已有失败。正式App长ChatView在visualViewport收缩也复现scrollTop3421→0，根因由root继续确认。

Q-001A未来回归建议（由GUI审计者提供，不重复算本审计已验证）：锁定状态后fetchLock失败必须保持锁定；标签关闭后借用资源late acquire必须释放；chatWait gone结束轮询，立即拒绝需要退避/取消。本次两修复范围外，详见Q-001A。


### 正式回归与独立审查补充

详见 [Q-012.md](Q-012.md)。原始业务文件哈希与固定基线相同，同一冻结脚本在 baseline-final 捕获三个 commit 选择方式回顶部，以及 chat 键盘第一步失尾。第四版由本任务在独立23821运行 Chrome/WebKit各13场景，26/26通过，changedDuringRun=[]、未捕获pageerror=0；无console监听，不宣称所有console零错误。比较红绿全部源指纹仅两业务文件变化。Lab跨iframe失焦、并发真实wheel+layout与WebKit迟到caret scroll另有开发红，均保留断言并最终通过。

现有ai-elements的mobile返回selector漂移已由root修为session bar内语义返回按钮；另一个工具展开超时的诊断补丁仅增加失败截图、focus/tools JSON与currentPage，不改断言。本任务只读复核无阻断。root随后报告ai-elements-final2串行四组通过，第一次失败日志仍保留，未证明争焦点是根因；默认npm test/tsc/gen/build与editor-history16组亦由root报告通过。这些作者结果与本任务独立26项证据分开记录。
