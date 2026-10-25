# Q-024 非作者审查：执行器、检查分层与 CI

状态：review complete（所查实现无未处理阻断项；Q-025、最终交互与真实 CI 结果另跟踪）
审查者：validation_audit；作者：root。
固定范围：861652db → 409f5817（含 f3fb5254、b3142d33、409f5817）；审查开始实际 HEAD e89da2e6 仅追加 identity，未改变本次所查文件。
只读实现目录：/Volumes/disk0/khor-code-quality-wave1；本任务仅写此中央卡和外部证据，不改实现。

## 结论与发现

netlab 编译/运行失败、空跑的错误传播、mobile 静态与编译分离、原默认集合保持、release 等待 quality 的依赖链均成立。发现一项低影响参数保真问题和一项 Linux 驱动适用范围风险，作者已经补丁处理；其中参数问题已由审查者独立回归确认关闭。尚未运行 GitHub Actions，不能把本机检查或 YAML 解析称为远端 CI 通过。

### R1：远程 filter 的尾部换行被裁掉（低影响，已修复复验）

原审查版本 scripts/netlab.sh:39 用命令替换接收 sed 输出，POSIX shell 会移除末尾换行。非 Linux 分支的独立 mock SSH 探针证明空格、单引号、`$()`、分号都保真，但 `ends-with-newline\n` 到远端变为 `ends-with-newline`；SSH 退出29正确传播。此项不涉及真实执行注入，影响是过滤条件发生变化。

证据：`Q024/review-remote.json`。作者在工作树加终止哨兵并去除哨兵后，审查者复验8种参数，包括单/双尾换行与空串，全部保真且仍返回29，见 `review-remote-fixed.json`；对应 netlab SHA256 = 02b21cf1ee79cce61adb08477827ce21a04e9ff322c73a8bd866acced7f975d5。该补丁后5个既有 netlab runner 回归及 sh语法再次通过，见 `review-netlab-fixed.log`。建议保留远程 quoting 探针为长期用例，既有5项均只覆盖 mock Linux 分支。

### R2：键盘驱动将 macOS 修饰键泛化到全部 WebKit（适用范围风险，代码已收窄）

check-scroll-lifecycle.mjs:130 的注释只针对 macOS，而原条件对 Ubuntu WebKit 同样发送 Alt+Tab。此次未启动额外 Linux browser，因此不把它写成已复现的 CI 失败。作者已改为 `process.platform === 'darwin' && browserType === 'webkit'` 才使用 Alt+Tab，其余使用 Tab；只读复核与 node语法检查通过。最终 Linux/两 engine 运行证据仍需要 CI或相应环境完成。

## 实现核对

| 范围 | 独立检查结论 |
|---|---|
| scripts/netlab.sh:85–113 | 编译日志完整落盘，失败保持原退出码并不启动 scenarios；运行失败先输出汇总和全文再保持状态；零场景/无结果marker返回2，避免过滤空跑假绿。`grep|sed|sort`汇总不是最终通过依据。 |
| scripts/netlab.sh:33–42 | rsync/SSH 使用独立argv，filter逐项单引号保护；mock验证 shell元字符未执行、状态29传播。尾换行问题已按R1关闭。 |
| scripts/tests/test_netlab.py | 真实shell置于独立scratch，PATH优先使用nft/tc/cargo替身、NETLAB_TOOLS也指替身；未调用真实Cargo/网络/namespace。覆盖编译37、场景42、空结果、缺report和成功日志/filter；没有把被测状态机重新写成断言。 |
| check-mobile.mjs:9–47 | 参数冲突返回2；static-only在工具链检测前结束；静态失败先返回1；非mac编译拒绝而非假通过；rustup缺失/失败、dist缺失、cargo失败/信号都非零。compile-only不读静态契约。 |
| check-mobile-static.mjs | 原entry point、Info.plist文案/ATS、同步命令普查、主题、参数命名契约保留；仅fs/path，模块本身无编译副作用。iOS Info.plist在git中，不依赖本机未跟踪生成文件。regex普查仍是源码契约检查，不证明实际耗时。 |
| mobile名单变化 | 新增5项确认工作队列注册/读/取消或平台能力；移除已async的reveal/show_menu。chat_control/chat_respond/term_ready保持红，未靠加入QUICK归零；Q-025负责生产修复和进一步socket语义复核。 |
| package.json | 自动展开旧test与新test:unit+test:types后，31条命令逐项顺序完全相同（30脚本+tsc）；pretest仍npm run gen，gen增加--locked。interaction显式串行lock-state、AI Elements、scroll-lifecycle；gates登记门自身不冒充执行。 |
| quality.yml | scripts/agent/gui三个实际作业；Node22.19.0；GUI先npm ci与默认检查，再gates/mobile static，再安装锁定playwright-core所需浏览器与依赖，最后交互。失败不被continue-on-error吞掉；证据always上传。 |
| release.yml | quality用job-level `uses`调用同仓workflow_call；frontend needs quality，build needs frontend，publish needs build，无绕过发布路径；quality将权限降到contents:read。 |
| Browser channel | 三个交互入口均读取PLAYWRIGHT_CHROMIUM_CHANNEL；空串转undefined，匹配CI下载的Chromium；本机未设时保持系统Chrome。QUALITY_HEADLESS=1和两个engine设置正确。 |

## 独立运行与版本

图谱先调用 get_minimal_context/detect_changes；图处于旧HEAD且只有17个文件，提示的risk/test gap不作通过依据，补实际固定diff和当前源文件。固定审查指纹在 `Q024/review-manifest.json`；作者补丁另以上述R1的源码哈希记录。

本任务运行：

- Python unittest discover：11用例，本机macOS 10通过/1非mac分支按平台skip；只使用现有工具替身。
- `node apps/gui/scripts/check-mobile.mjs --static-only`：退出1，准确打印 chat_control/chat_respond/term_ready 三项。没有Cargo调用。
- 两个mobile脚本 `node --check`、netlab `sh -n`通过。
- 默认test命令序列和pretest自动对比一致。
- 独立非Linux remote探针：uname/rsync/SSH均替身，不执行生成的remote命令；按shlex解码SSH参数验证保真并验证退出状态。R1修后再次成功。
- 未启动浏览器、Cargo、发布或真实远端操作，未改作者文件。

作者另报告test:unit、types、agent 42通过/1真实release binary相关skip。root最后一次交互运行在24/26处工具handle异常终止、结果无finished，不能记作全量通过；17项lock hook与4组AI Elements已有通过输出，完整串行重跑由root进行。本任务不把这些作者通报算入独立执行计数。

## 外部契约核对

Node22.19.0官方文档记录22.18开始默认类型擦除，可支持此处`node --test test/*.test.ts`的可擦除类型；依赖lock中最高最低要求含>=22.19.0，CI选定版本满足。仍不宣称本机Node26结果等于Node22完整运行。[Node22.19 TypeScript](https://nodejs.org/download/release/v22.19.0/docs/api/typescript.html)

同仓相对路径的reusable workflow按调用方同一commit读取，且嵌套权限只能保持/降低；当前workflow_call和release调用形式符合该契约。[GitHub reusable workflows](https://docs.github.com/en/actions/how-tos/reuse-automations/reuse-workflows)

CI在npm ci后安装浏览器及系统依赖，再串行执行交互，符合Playwright官方安装顺序。未指定系统Chrome的channel时会使用已下载的对应Chromium。[Playwright CI](https://playwright.dev/docs/ci)

## 通过边界与下一步

Q-024实现审查可进入集成后验证；R1修复和R2收窄需由作者提交并记录最终SHA。Q-025三项静态红是明确依赖，必须修生产语义后复验，不应放宽门禁。GitHub实际运行、最终完整26场景及真实原生平台不在本次通过结论中；Ruby YAML语法检查也不等同GitHub schema/actionlint或真实执行。
