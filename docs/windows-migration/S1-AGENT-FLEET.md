# 第一阶段（S1）：Windows 上的 Agent 大同世界

> 目标与取舍见本文 §1；已完成与未完成见 §2、§7。每一项都留了命令与输出，
> 没做到的写「未做」，不把没验证的写成通过。

## 1. 定下的方向（用户决策）

1. **默认 Khor 对话**，原生终端可选；两条栏位并存。
   这里说的是**形态**：默认创建 khor 拥有的对话（`kind='agent'`，手机可直接用），
   具体由哪个 agent 回答**跟随清单**（本机是 Codex，因为它是唯一内置可用的）；
   Khor 自有 agent（Pi/DeepSeek）需要账户，不是默认。
2. **两个都做**：Khor 自建对话（`_ghost` + ACP/Pi）与「原生应用拥有对话」。
3. **原生会话在手机上接受按键中转**（`term_key`），并在界面上标注为远程终端。
4. **npm 可选、自动探测、绝不强制**：主路径（内置 Codex、Pi/DeepSeek、原生接管）零依赖；
   只有注册表里的第三方 ACP 适配器需要包运行器。
5. DeepSeek 走 Khor 自有 agent（Pi）；ACP 注册表里没有 DeepSeek 适配器。

## 2. 已完成（本阶段）

### 2.1 窗口形态：Windows 恒为 GUI 子系统（提交 `40093bd7`）

**问题**：`#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]` 让 debug 构建成为
控制台程序，而 release/安装包是在标题栏修复之前打的（release 二进制 21:11、安装包 20:41，
修复提交在 21:39）。于是这台机器上仅有的两个可运行产物各错一半：**一个带控制台，一个有双层标题栏**，
轮换使用就成了「窗口在两种形态间横跳」。

**改法**：Windows 恒为 GUI 子系统；原本由控制台承担的诊断改由 `diagnostics` 模块写日志文件
（`%LOCALAPPDATA%\Khor\logs\gui.log`，`KHOR_LOG_FILE` 可覆盖；8 MiB 上限**在启动时检查一次**，
不是运行中的硬上限），`KHOR_GUI_CONSOLE` 分配真实控制台给开发用（`1` 开，空/`0`/`false` 关）。

**只在"没有任何标准流"时才重定向**：`main` 里调用但先问 `GetStdHandle` —— 同一个二进制还会被
再执行为 `_host`/`_ghost`/`_cagent`/`_codexagent`/MCP worker，它们拿到的是**父进程给的管道**，
重定向会把协议握写进日志、让父进程一直等。这一条是独立验收（见 §8）抓住的阻断项，
修复见 `8089527c`。

**证据**：

| 项 | 结果 |
| --- | --- |
| debug 二进制 PE 子系统 | 3 → **2（WINDOWS_GUI）** |
| 启动后可见控制台窗口 | **0** |
| 日志文件 | 已创建，首行为 `[gui] 2026-11-17 00:19:53 start 0.2.0 (…\khor-gui.exe)`，随后是 `khor: window transport on 127.0.0.1:1432 (4 workers)` 与页面自身流量 |
| `KHOR_GUI_CONSOLE=1` | 出现 conhost 子进程 → 逃生门有效 |

### 2.2 子进程不再闪控制台（提交 `992f2ebd`）

GUI 进程没有控制台，Windows 会给每个控制台子进程**新开一个可见控制台**。`CREATE_NO_WINDOW`
此前只有一处（`crates/node/src/khor_agent_process_windows.rs:199`，自有进程适配器），
**本轮新增 10 处调用点**。新增 `console::hidden`（返回带 `CREATE_NO_WINDOW` 的 `Command`，
而不是接收 `Command`——`creation_flags` 是**替换**语义，见模块注释），覆盖 doctor 的两处
（`khor.exe version`、`khor serve`）、编辑器的两处（`code --version`、`code serve-web`）、
以及添加机器时走的 6 处 ssh/ssh-keygen。

顺带修掉一个真 bug：`khor_cli_binary` 的**邻接查找**在 Windows 上找的是没有 `.exe` 的 `khor`，
所以只有 PATH 回退能命中；「诊断」在 CLI 不在 PATH 的机器上就会报「没有 CLI」。现在邻接查找也带
平台后缀（PATH 回退仍在）。
证据：`doctor_fix(serve)` 返回「a resident was started」（修复前该调用在本机定位不到二进制）。

测试：`doctor` 1、`code` 9、`satellite` 26 全过。node 全量 lib 首轮 8 失败、复跑 4 失败（1053 / 1057 passed），
逐个隔离跑全部通过（35 + 15 + 2）。**只能说明这几条与本次改动没有可见关联，不能证明全量并发下的失败
是"既有且无关"**——独立验收指出这个推断越界，这里收窄为"未证实与本轮有关，也未证实无关"。

### 2.3 Agent 清单：这台机器实际有什么（提交 `678c25c7`、`d277c83d`）

**为什么需要**：`agents known` 回答的是「khor 知不知道启动它的 argv」，是注册表快照；
而人先问的是「**这台机器上有什么用、在哪、khor 能拿它做什么**」。两件事不能并成一张表：
本机装着的 DSH 从不在 ACP 注册表里，注册表里十几个条目本机一个都没装。

`agent_fleet` 用文件系统能回答的事实构建清单（PATH / 安装目录 / 配置目录是否存在）。
**"绝不启动任何东西"只在 Windows 成立**：Windows 分支不碰登录 shell；macOS/Linux 上
`fleet → user_path → asked_shell` 的首次调用仍可能起一次登录 shell（进程内 `OnceLock`，只一次）。
路线的优先级写死在数据里：`builtin`（khor 自带 shim，零 npm）→ `native-acp`（工具自带 ACP，零 npm）
→ `registry-acp`（包运行器适配器，唯一需要 runner 的一档，行上直接标 `needs-runner`）
→ `terminal`（地板，任何 agent 都能被敲进终端）。

**两条自我限制**（独立验收后收紧，2026-11-17）：

- **只认 `npx` 作 runner**。`Node::agent_launch` 把注册表 argv 原样交给启动器，而那些 argv 都以
  `npx` 开头；本机只有 `pnpm` 时若标 `registry-acp` 就会出现「清单说可用、启动找不到 npx」。
  `pnpm dlx` 是下一步（要先教启动器改写第一个词），不是现在的承诺。
- **生产表里的 `program_dirs` 目前全空**：安装目录查找这条路径由测试里的构造条目证明，
  真正生效的是 PATH 与配置目录。文档不再声称"发现了安装目录"。


**证据（CLI）**：

```
$ khor agents fleet
9 known, 2 installed, package runner: none
codex   Codex              installed  C:\Users\admin\AppData\Local\OpenAI\Codex\bin\…\codex.exe  builtin   acp run list read resume send terminal
claude  Claude Code        not installed  -  -  terminal
dsh     DeepSeek Harness   installed  F:\DeepSeek Harness\…\dsh.cmd  terminal  run resume send terminal
gemini  Gemini CLI         not installed  -  -  terminal
…
```

（上面的 `package runner` 在本机是 `none`：只有 `pnpm`、没有 `npx`，按上面那条自我限制不算数。
最初版本这里印的是 `pnpm dlx`，独立验收指出与实际启动器矛盾后改正。）

**证据（界面）**：诊断 → 会话关联 → 「这台机器上的智能体」。
Codex 显示 `已安装 builtin` + 路径 + `acp · run · list · read · resume · send · terminal`；
DeepSeek Harness `已安装 terminal` + `run · resume · send · terminal`；**未安装的七个**仍在表上。
`POST /agent_fleet` 经窗口传输返回 200、`agents: 9`（截图见
`F:\Temp\khor-try\shot-fleet2.png`；截图中那行 runner 是收紧之前的构建）。

测试：`agent_fleet` 5 个单元测试（含「按安装目录查找」——第一版在 Windows 上漏了 `.exe` 后缀，
正是这条测试抓出来的）；`khor-gui-core` 的 `command_tables` 4 过（桥接与 Tauri 两张表一致）；
gui-core `agents` 12 过；前端 `tsc --noEmit` 干净。

### 2.4 列表默认按项目分组（提交 `586043a7`）

Windows 上列表打开时按 `place` 分组（"哪个项目在忙"是这台机器上先要回答的问题），
macOS/Linux 仍是 `recent`。只改**没有偏好时的默认**：`storedArrange` 仍返回人选过的值。
证据：同一批行在改动前是平铺的，改动后列表出现 `∨ F:\#Gong\#AI4S\khor`（8 条）与 `∨ 没有目录` 两个分组头。

### 2.5 产物与安装包

构建产物与提交的对应关系（避免"当前 HEAD 的构建"这种无法核对的说法）：

| 产物 | 构建时间 | 含哪些提交 |
| --- | --- | --- |
| 早期 03:41 版本 | 03:41 | 到 `c74a3cc77` 前后，**不含** `1dbf9f4b` 及其后的验收修正 |
| 05:37 版本 | 05:37 | 到 `1dbf9f4b`，**不含** `8089527c`/`004262e9`（验收修正） |
| 最终版本 | 见 [HANDOFF-20261117-0600](HANDOFF-20261117-0600.md) §2 | 含全部验收修正 |

`Khor_0.2.0_x64-setup.exe` 约 36.1 MiB；release 二进制 PE 子系统 = **2（GUI）**，启动后无控制台、
单标题栏、列表按项目分组，日志文件收到 `khor: window transport on 127.0.0.1:1432 (4 workers)`。
`scripts/windows-installer-check.ps1` 在 04:32 那个产物上是 **6/6 PASS**
（install 6059 ms、layout 89、start 2100、stop 711、uninstall 2222、removed 57；既有 `logs`、`runtime` 保留）。
⚠️ 那次检查**不覆盖**其后的验收修正；最终产物需要再跑一次，见交接文档。

### 2.6 DSH 通道：已用实验验证（未提交代码，结论先行）

在隔离 `DSH_HOME` 下实跑 `dsh --profile headless --json`：

```
{"type":"session","sessionId":"session-8e39627a-…","cwd":"F:\\Temp\\dsh-exp\\ws"}
{"type":"status","phase":"turn_start","turn":1}
{"type":"final","sessionId":"session-…","reason":{"kind":"error","error":{"code":"MISSING_CREDENTIAL"}},"text":""}
```

- 第一条事件就带 **sessionId + cwd**，会话按项目目录持久化到 `<home>/sessions/<编码路径>/session-<uuid>/`；
- 用 `--session-id <同一个 id>` 再跑：回同一个 id，且 **`"turn":2`** —— 证明接着上一轮历史继续；
- 不存在的 id → 结构化 `{"type":"error","message":"session … does not exist"}`，退出码 1；
- 失败（隔离 home 无凭据）也是结构化事件，不是崩溃。

**未测**：带真实凭据的完整一轮；接管 DSH 桌面创建的会话；与运行中的 DSH 桌面并发写同一会话。

## 3. 调研结论：接入面分档（"无论什么架构"的答案）

| 层级 | 形态 | 例子 | 需要 npm |
| --- | --- | --- | --- |
| 1 | 原生 ACP 模式 | `gemini --acp`、`opencode acp`、Copilot CLI、Goose、Cline | ❌ |
| 2 | ACP 适配器（注册表 22 个可启动） | `npx -y @agentclientprotocol/codex-acp` | ✅ |
| 3 | Khor 内置 shim | `_codexagent` → `codex app-server`、`_cagent` → `claude --print` | ❌ |
| 4 | 原生 CLI 结构化输出 | DSH `headless --json --session-id`、Codex `resume`/`queue` | ❌ |
| 5 | HTTP 服务器 | `opencode serve`（OpenAPI 3.1）、DSH web | ❌ |
| 6 | PTY 兜底 | 任何 CLI | ❌ |

**同一段对话能否两端同时读写**：手机能遥控桌面，是因为**桌面是唯一写者、手机只是它的一个"脸"**
（`chat_say` 在 `api::answer` 里，与 Windows 的 `?bridge=` 走同一张表；原生 TUI 走 `term_key` 按键中转）。
两个独立进程各自作为写者写同一个 Codex rollout 不行——Codex 对已装载 thread 持独占 OS 锁
（`~/.codex/thread-writer-locks/<id>.lock`），khor 只做共享探针并明确拒绝。DSH 则不同：它的 web face
是带认证的多客户端服务器，所以 khor 可以作为客户端接入——那是真正的"同时"。

## 4. 参考对象：CC Switch 做了什么、不做什么

CC Switch（MIT，桌面三平台）管 **13 个工具**，方式是**写各 CLI 的原生配置文件**
（`~/.claude/settings.json`、`~/.codex/config.toml`、`~/.gemini/.env`…），提供 90+ 供应商预设、
一键切换、本机路由（协议转换）、聚合模型列表、MCP/Skills/提示词管理。
它是**配置层**产品，不是会话层。Khor 的定位不同：**发现 + 会话 + 跨设备**。
因此第一阶段**不做**供应商配置切换；那张「发现表」的思路被借用到了 `agent_fleet` 的清单里。

## 5. 尚未开始（后续切片）

- **B 项目化会话**：**已按用户决定的形态完成**（`1037cfa9` 之后）。Windows 默认按 `place` 分组、
  新建向导默认跟随清单选已装 agent、观察到的 Codex 会话可「在终端里继续」（`codex resume <id>`）
  与「发一条消息」（`codex queue --thread <id> --message`，走 Codex 自己的守护进程，
  不是接管——所以已装载的 thread 也能说上一句话）。
  **DSH 已接入三样**（§2.8）：headless 驱动（`agents tell dsh`）、会话清单（`agents dsh`）、
  界面按项目列出并可「在终端里继续」。**正文也已读**（§2.9）：解析器 + `agents dsh <session>` + 界面「读一下」。仍未做：把 DSH 会话接进
  导入/聊天面板（`ImportCandidates`/`ContentSource` 管线）——现在是清单页里的独立读入口。
- **D 两机互联**：**已打通**，见 §8。
- **E 启动性能**：**达标**。本机 `ls` 不再建 iroh 端点（**1168 → 80 ms**，`3ce5c5e3`）；
  Windows 不再为 PATH 启动登录 shell（`5df68991`）；按独立验收排第一的项去掉 Windows 上
  不可能命中的 legacy 白名单整份哈希（`684e1d65`）。同口径 `khor open` **~5.0 s → 中位 742 ms**
  （6 次 728/742/746/779/648/692），**P95 ≤ 1.5 s 达成**。详见 [W7-PERF §2.2](W7-PERF.md)。
  仍可优化：缓存命中上的 publish 身份哈希；按验收意见只能以"不可变内容 + lease 复用"设计，
  不能用文件元数据替代信任证明。

## 6. 已知限制

- 「已安装」= 程序在 PATH 或已知目录里；**不代表**能跑（未登录、缺运行器等要起的时候才知道）。
- 清单里的路线与能力是**本阶段代码能做到的**，不是厂商能做的一切。
- DSH 的会话列表/读取尚无适配器，所以行上没有 `list`/`read`；不要把它读成"不能读"。
- 修复只保证今后会话不被误杀；`Failed` 的旧行不会自愈（截图里那些就是修复前的遗留）。

## 7. 复现方式

```powershell
# CLI（debug 构建，见 F:\Temp\khor-check\debug\khor.exe）
khor agents fleet

# 界面：release 或任意 GUI 构建
# 左上「…」→ 设置 → 诊断 → 会话关联 → 「这台机器上的智能体」

# 诊断日志（无控制台时）
Get-Content "$env:LOCALAPPDATA\Khor\logs\gui.log" -Tail 20
```

## 8. 独立验收（2026-11-17，codex 进程 · ccs-kapestack/gpt-6-astra · reasoning max · read-only）

报告：`F:\Temp\khor-acceptance-report.md`（1.0 MB，含逐文件阅读记录与最终七节结论）。

**结论是"暂不建议合入 main"**，理由是三条：日志重定向会破坏 GUI 自复用角色的协议管道；
原生续接与远端默认选择没有闭环；新测试在 Unix 上有可执行权限缺陷。三条都已在本轮修掉：

| 验收发现 | 处理 | 提交 |
| --- | --- | --- |
| **P1 阻断**：`diagnostics::attach` 无条件替换 stdout/stderr，会吃掉 `_host`/`_ghost`/`_cagent`/`_codexagent`/MCP worker 继承的**管道**，握手写进日志、父进程一直等 | 改为"只在没有任何标准流时才重定向"（`GetStdHandle`），并修 `KHOR_GUI_CONSOLE=0` 被当成开的读法 | `8089527c` |
| **P2** 原生续接丢弃 `open_shell` 返回的新 id，终端不会打开（启动命令要等 `Ready`） | 新增 `khor:open-session` 事件 + App 自己的打开流程（`sessionTabs.open` + 详情页） | `004262e9` |
| **P2** 远端目标沿用本机清单，可能默认一个对方没有的 agent | 清单只对 `DeviceRow::me` 生效；远端保持出厂默认，直到节点能为那台机器回答 | `004262e9` |
| **P2** 清单宣称 `pnpm dlx` 可用，启动器却只会用 `npx` | runner 只认 `npx`（并注明 `pnpm` 需先教启动器改写） | `004262e9` |
| 测试在 Unix 上缺可执行位 | `write_program` 在 unix 设 `0o755` | `8089527c` |
| 文档事实错误十余条 | 逐条改正：`file_copy` 口径、trace 表算术与样本混用、`open` 两口径不可直接比、`CREATE_NO_WINDOW` 并非"从未设置"、CLI 查找不是"永远找不到"、未安装是七个不是九个、`program_dirs` 生产为空、Windows 分支才"不启动任何东西"、8 MiB 只在启动时检查、ROADMAP 状态过期 | `2f…`（见 git log） |

**验收没有做到的事**（照实记录）：构建、测试、安装、GUI 操作、安装包与 HEAD 的对应关系、
Linux 原始计时、macOS 动回归、Win10、双机、DSH 真实凭据——全部未复核，报告里明确列了。

**验收给下一轮的排序**：① ~~先筛掉 Windows 不适用的 legacy 镜像白名单检查~~——**本轮已做**
（`684e1d65`：debug 热路径 2.4 s → 0.87 s，release 中位 742 ms）；② 续接体验完善
（打开目标终端、幂等重试、已有 writer 的处理——**本轮已补上"打开目标终端"**，另两项未做）；
③ 按目标机器维护 fleet 并与 launcher 共用路线解析、清单加载不阻塞设备展示——**只做了"只对本机生效"
这一半**，按机器取清单与共用解析未做；④ 设计已发布 runtime generation 的复用（不可变内容 + lease），
**不能**用路径/大小/mtime/首块摘要直接替代完整信任证明。

### 2.7 给厂商的对话发一条消息（提交 `119ecfd8`、`1037cfa9`）

`khor send`/`tell` 驱动 khor 自己的会话；这一条是另一种情况：Codex 的 thread 在运行，
人想往里面说一句话，但**不接管**。Codex 自带这条通道——`codex queue --thread <id> --message <text>`，
写入走它自己的守护进程，所以没有第二个写者、不用抢 writer 锁；这正是"已装载的 thread
说不上话"的地方能说话的原因（`takeover` 在那里是被明确拒绝的）。

- CLI：`khor agents tell codex 01a1de3a-… khor-send-probe` → 退出码 0，
  回执是厂商自己的句子：`Queued message 01a1e6a5-… for thread 01a1de3a-…`。
- GUI：只读 Codex 会话的行菜单多了「发一条消息」，弹层里显示**厂商回执**；拒绝时显示它的 stderr。
- 文本作为**一个 argv 元素**传给 `Command`，从不经过 shell；只支持 Codex（按名字拒绝其它厂商，
  不猜命令行）。新增的四条文案走 `[msg]` 段（`native-send-*`），`wont-start` 复用既有键
  ——catalog 自己的重复校验在发布前抓到了重复文案。

### 2.8 DSH：说话、清单、按项目列出（提交 `857c0b97`、`a8913afb`、`5abbc8a3`）

DSH 不是 ACP 注册表里的 agent，所以它没有 `session/list` 可问。Khor 走厂商自己的三样东西：

1. **headless 驱动**（`857c0b97`）：`dsh --profile headless --json --session-id <id> <一句话>`
   ——一段对话的"再问一轮"。`--json` 的帧里 `final.text` 是回答、`reason.error.message` 是拒绝
   （DSH 在模型调用失败时仍退出 0）。两个平台事实由实跑发现：`dsh` 是 `.cmd`，
   Windows 上要用显式 `cmd.exe /c`（不经 shell）；**DSH 拒绝从别的目录恢复会话**
   （`session "…" was recorded in "…", not "…"`），所以通道带 `cwd`，GUI 传行的 `place`。
   验证：隔离 `DSH_HOME`、在会话自己的目录里跑，退出码 1，拿到厂商原话
   `llm-deepseek: no API key for provider route "deepseek-official"…`。
   **带真实凭据的一轮没跑**（会花你的额度、写你的会话），留给你。
2. **清单**（`a8913afb`）：`khor agents dsh`。DSH 的会话文件首行就是头
   `{type:"session", id, createdAt, cwd, parentSession?, origin?}`，所以只需要解压**第一帧**，
   不用解全文件、也不用反解目录名编码。本机实测 **16 段**（子代理会话带 `↳ parent`）。
   依赖只有 `ruzstd`（纯 Rust 解压，不引入 C 工具链）。
3. **界面**（`5abbc8a3`）：设置 → 诊断 → 会话关联 → 「DeepSeek Harness 的对话」，
   按记录目录列出，每行可「在终端里继续」（`dsh tui --resume <id>`，在该会话自己的目录里跑）。
   行上有**标题**（`3b8b869a`）：DSH 的文件是**多帧 zstd**，标题是 `session/title` 事件里的
   `data.title`；扫到"provider"来源的标题就停，子代理用 `descriptor.label` 顶替，
   从没起名的会话诚实显示目录。本机 16 段全量扫描 **1.3 s**。消息事件（正文）仍未读。

**桥接层运行时证据**：`/dsh_sessions` → 200、16 行；`/agent_fleet` → 200、9 个 agent。
**NOT_TESTED**：这一节的像素呈现（WebView2 里 CDP 重载没生效），以及带凭据的真实一轮。

### 2.9 DSH 正文：一段对话读得出来（提交 `ff519450`、`c245f31b`、`5b8d31be`）

清单只解头帧；**正文**是另外三片：

1. **解析器**（`ff519450`）：DSH 的事件映射成仓库已有的 `Utterance`——`user/message` → User、
   `assistant/message` 的 reasoning → Thought、text → Agent、`tool-call` → Tool（**转成 claude 的
   `tool_use` 块**，参数从字符串解析成对象）、`tool/result` → ToolDone（claude 的 `tool_result`）。
   复用的是 `cagent::said`/`settled` 这条唯一翻译——验收期间发现过"每个厂商各写一份翻译"导致
   工具标题/状态丢失的旧 bug，这里不重犯。`tool/call` 事件跳过（它是 assistant 消息里那条的重复）。
2. **整段读出**（`c245f31b`）：`conversation(id)` 按会话**自己的头**找到文件（不解目录名编码），
   逐帧解压到 4 MiB，`agents dsh <session>` 逐行打印：真实子代理会话 **12,388 行、1.4 s**。
3. **界面**（`5b8d31be`）：清单页每行多了「读一下」，弹层显示 `谁 / 说了什么`；工具显示
   `name {input}`，失败结果带 `failed:`。与 CLI 同一个 reader，所以两边不可能对不上话。

**NOT_TESTED**：弹层像素（WebView2 里 CDP 重载不生效的老问题）；`/dsh_transcript` 的桥接运行时
只做了 `command_tables` 与类型检查（上一次 `/dsh_sessions` 的 200 来自更早构建）。

### 2.10 统一驱动接口：六个动词有了一个归属（提交 `51a1c6f1`）

计划里的 `detect/list/open/send/read/launch` 之前是散件：名册负责 detect、`native_queue` 负责 send、
`dsh_sessions` 负责 list/read、表格里一格写 launch 路线，各面各取一件。更糟的是**能力徽标是手写的**
（`list: true` / `read: false`），所以"实现了但徽标还说没有"真的发生了：DSH 的 list/read 都是先把代码写完、
表格仍是 `false`。

现在 `agent_driver` 是唯一答案：own 对话类动词（`list/read/send/open`），逐条路由到真实实现，
没有路的 agent **按名字拒绝**；`open` 返回"终端该敲的 argv"（`codex resume …` / `dsh tui --resume …`），
detect 与 launch 留在 `agent_fleet`（它们是查找与 argv，不是对话）。

两个面都改走它：`agents tell` 与行菜单发消息走 `agent_driver::send`；列表的续接动作改问节点要 argv
（新命令 `native_resume_argv`），TypeScript 不再自己拼 `${agent} resume ${id}`。DSH 行现在如实显示 list/read。

**两条测试守住矩阵，双向**：行上每个徽标都必须是本模块实现了的动词；本模块实现的每个动词都必须在行上显示。
第二条正是能抓住那次 `false` 的测试。

### 2.11 DSH 会话进入导入与项目列表（提交 `d266014e`）

之前 DSH 只在清单页可见；它不在会话列表里，所以项目分组看不到、聊天面板也没有路。现在它们是
**普通的导入会话**，`place` 就是每段被记录时的目录——项目分组正是按它分的。三处：

1. **候选**：`import_candidates` 把 DSH 仓库里每段会话列为只读候选（`dsh:<session>`），已导入的跳过。
   DSH 不是注册 agent，没有 ACP `session/list`，所以来源就是它写的仓库。
2. **导入**：`import` 路由 `dsh:` 到 `import_dsh_candidate`，生成通常的 `Observation`（Native 命名空间、
   agent `dsh`），**标题/目录/起始时间从仓库读回**而不是信任候选字符串——旧面板不能导入错名字。
3. **阅读**：`Provider::Dsh` 加入 claude/codex。它的仓库是 zstd 帧不是 JSONL，所以
   `NativeTranscript::resolve` 给它一份 `{root}/.khor/cache/dsh/` 下的解压快照，每次 resolve 重写
   （刷新视图就能看到最新一轮）；之后 parser、协议块、binding 与任何厂商同一条路。定位先按
   "目录名=会话 id" 单路径探测，命名不同再回退到比对头帧。

**验证**（隔离 `KHOR_HOME`，真实 DSH 仓库）：
```
$ khor import            → 16 个 dsh 候选（标题/项目/年龄/`dsh:` id）
$ khor import dsh:session-146aa58b-… → observe/42a4d2c2…
$ khor sessions          → Probe request READY reply only（该行）
```
另有 `#[ignore]` 端到端测试（`KHOR_DSH_E2E=1 … --ignored`，0.54 s）：导入 → 行带标题/项目/category=dsh →
`session_content` 返回 Available → 快照解析出有说话人的行。`session_content` 全套 41 个测试仍过。
