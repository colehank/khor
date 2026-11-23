# W7 · 性能矩阵

按 [APPROVAL-PACKAGE](APPROVAL-PACKAGE.md) §5.2：同 HEAD、同依赖锁定、同工作负载，Linux 作为现有基线，记录中位数 / P95 / 峰值；预热 2 次、之后 5 次有效。

**两台是不同硬件，这里是跨机器对比，不是受控的平台对比**——记录硬件是结论的一部分，不能省略：

| | Windows | Linux |
| --- | --- | --- |
| CPU | Intel i9-10900（10 核 / 20 线程） | AMD Ryzen 9 9950X（16 核 / 32 线程） |
| 系统 | Windows 11（26100） | Linux 7.0.0-42 x86_64 |
| 二进制 | `khor.exe` 103.9 MB（release） | `khor` 97.6 MB（release） |
| HEAD | `3fe1dab0` | 同一提交（`git archive` 打包上传） |
| 工具链 | rustc 1.95.0 | rustc 1.95.0 |

测量脚本：Windows `scripts/windows-perf.ps1`；Linux `scripts/windows-perf.sh`（同一指标表、同样预热/次数）。

## 1. 矩阵

| 指标 | Windows 中位 / P95（ms） | Linux 中位 / P95（ms） | 比值 | 判定（§5.2：>30% 需解释，>100% 阻断） |
| --- | --- | --- | --- | --- |
| `cli_version` | 36.9 / 49.5 | 1 / 1 | 37× | **阻断项**，已归因：Windows 进程创建成本，见 §2.1 |
| `cli_doctor` | 5871.9 / 5917.8 | 2619 / 2619 | 2.2× | **阻断项**，已归因：探针含网络超时，见 §2.4 |
| `session_create` | 6405.8 / 6613.7 | 273 / 275 | 23.5× | **阻断项**，见 §2.2（已定位；两项已修，热路径 ~1.67 s） |
| `sessions_list` | 132.3 / 134.4 | 20 / 20 | 6.6× | **阻断项**，已归因：进程创建 + NTFS，见 §2.1 |
| `file_scan` | 1117.6 / 1150.7 | 28 / 33 | 40× | **阻断项**，见 §2.3（已定位并修复：本机 ls 白建端点） |
| `file_copy` | 49.0 / 49.6 | 2 / 2 | 24.5× | **口径问题，不是 khor 的指标**：脚本第 137 行量的是 PowerShell 自己的 `Remove-Item` + `Copy-Item`，完全不经过 khor。24.5× 是 cmdlet/NTFS 与 `cp` 的差别。应从矩阵移除或改标为"宿主基线" |
| `gui_cold_start` | **530.2 / 594.7** | 不适用 | — | Linux 无桌面会话，该行只在能显示窗口的平台测量 |

`gui_cold_start` 的定义是「窗口自己的传输开始应答」，不是「窗口出现」——一个能画但所有命令都挂着的窗口不算就绪。

## 2. 归因（哪些是平台常数，哪些是待查项）

### 2.1 Windows 的进程创建是 ~35 ms 的常数

实测同一台 Windows 上，一个什么都不做的 `khor` 调用中位 **58.9 ms**（含错误路径与 PATH 处理），`cmd.exe /c exit` 中位 **56.3 ms**。即：在这台机器上**启动一个进程本身就要几十毫秒**，与 Khor 的代码无关（PE 装载 + Defender 实时扫描）。

据此：
- `cli_version` 36.9 ms ≈ 纯进程创建；Linux 1 ms 是 `date +%s%3N` 的分辨率下限。
- `sessions_list` 132 ms ≈ 36 ms 启动 + ~96 ms 读存储；Linux 20 ms ≈ 1 ms + 19 ms。
- `file_copy` **不是 khor 的指标**：脚本量的是 PowerShell 自己的 `Remove-Item` + `Copy-Item`
  （`scripts/windows-perf.ps1:137`），不经过 khor 的任何代码。这一行应从矩阵移除或改标为宿主基线，
  它测的是 cmdlet/NTFS 与 Linux `cp` 的差别。（独立验收指出，2026-11-17 更正。）

**结论**：这几行相对 Linux 的倍数主要反映**平台常数**，不是 Windows 实现劣化。按 §5.2 仍需列为阻断项等待用户确认，但归因明确。

### 2.2 `session_create`：已定位，不是 Defender（2026-11-17）

原先的假设是「每次都要重新映射/扫描 104 MB 运行时镜像，Defender 每次执行都过一遍」。
**假设不成立**：原因是三处可测的代码开销，且宿主本身很快。

**测量方法先修正一次**：PowerShell 的 `& khor …` 会多等约 2 秒（控制台/管道收尾），
所以「`open` 用了 6.5 s」里有一截不是 khor。可靠数字取**进程自身生命周期**
（`Start-Process -PassThru` 后 `ExitTime - StartTime`）。

**归因用 `KHOR_STARTUP_TRACE=<文件>`**（本轮新增的门，见 `crates/node/src/startup_trace.rs`）：
每个进程把 `pid / 相对进程启动的毫秒 / 标记` 追加到该文件。**下表全部取自同一次运行**
（`F:\Temp\khor-try\startup-trace-6.txt`，debug 构建、热路径；早先版本把两次运行的样本拼在一张表里，
增量算错，独立验收指出，2026-11-17 更正）：

| 标记 | ms | 增量 | 是什么 |
| --- | --- | --- | --- |
| begin → contracts-checked | 0 → 33 | 33 | 进程入口 + 运行时租约 + 契约自检 |
| image-current | 93 | 60 | 读本进程自己的 release 契约 |
| image-inspected | 873 | **780** | 对 104 MB 源二进制做 SHA-256，并起一个 `_runtime_info` 探测进程 |
| image-miss | 1371 | 498 | 缓存查找（按"这个程序自己是否是镜像"判断，命中不了自己的安装目录） |
| client-image | 2423 | 1052 | 缓存未命中路径上的 publish：身份哈希 + 清单 + 校验 |
| client-path | 2424 | **1** | 修复前同一位置是 **3099 ms**（登录 shell 探测；该数字取自另一次运行 `startup-trace-2.txt`） |
| client-spawned → client-ready | 2438 → 2692 | 254 | 宿主进程启动到 `host.json` 出现 |
| host-ready → roster-new → open-printed | 2692 → 2714 → 2715 | 23 | 名册登记 + 打印 id |
| node-dropped | 2716 | 1 | 释放 Node（不是退出耗时） |

**同一进程里的宿主自述**：`begin 0 → runtime 111 → contracts 127 → host-serving 145 → pty-open 167
→ program-found 171 → child-spawned 189 → listener-bind 191 → host-json 192`。
**宿主从进程入口到写 `host.json` 是 192 ms**（从 `host-serving` 算 47 ms）——不是「5 秒就位」。
早先那次「就绪 4.4 s」是把 `host.json` 的创建时间减去了 `meta.started_ms`，
而那个时钟在客户端的镜像工作之前就开始了。这个数字也不是"操作系统建进程"或"终端可交互"的时间。

**已修**（提交 `5df68991`）：`asked_shell` 在 Windows 上不再启动「登录 shell」。
`SHELL` 在这台机器上被设成 Windows PowerShell，于是每次建会话都起一个加载 profile 的
PowerShell，花 **3.1 s** 去回答一个 Windows 上不成立的 `-lic printf` 问题。
Windows 的 PATH 就是进程环境里那份，`KHOR_PATH` 仍可覆盖。

**再修一项（提交 `684e1d65`，独立验收排第一的优化）**：`inspect_supported` 先问
`legacy_info`，后者把**整个程序**（≤256 MB）SHA-256 一遍去比对四条已发布摘要——而四条全是
**Linux/macOS**。Windows 上永远不可能命中，这份哈希每次建会话白花。现在在文件检查之后、
哈希之前加平台门。**不是安全豁免**：manifest 路径（`inspect` → `publish_checked` 自带校验）
仍然为程序负责，Windows 上现在只剩这一条路径，与修复前行为一致。

| 时点 | `khor open -d -- cmd.exe /c exit`（进程生命周期） |
| --- | --- |
| 本轮仅 ls 修复的 release（**同口径基线**） | 6898（冷）/ **5010 / 5084 / 4989** ms |
| 修掉登录 shell 探测后 | 3680（冷）/ 1673 / 1670 / 1669 / 1679 ms |
| **加上 legacy 平台门后（最终）** | **728 / 742 / 746 / 779 / 648 / 692 ms**（中位 **742**，最大 779） |
| debug 对照（同一修复） | 878 / 832 / 896 / 1111 ms |

**同口径结论：~5.0 s → ~0.74 s（8.6×）**，目标 **P95 ≤ 1.5 s 达成**（6 次样本最大 779 ms）。
矩阵里的 6405.8 ms 是 `&` 口径（含 PowerShell 收尾），不能与这些数字直接比。

**仍可优化**（未做）：缓存命中路径上还有 publish 的身份哈希（debug 口径约 1.0 s，release 更小）。
独立验收明确反对用路径/大小/mtime/首块摘要替代完整信任证明；要做就按"不可变内容 + lease 复用"
单独设计。本轮达标后不再动它。


### 2.3 `file_scan`：已定位并修复——是本机 `ls` 白建了一个网络端点

`khor ls <本机> <40 个小文件目录>` 的 1118 ms 与目录、文件数、元数据都无关：
`ls_of` 先问常驻服务，问不到就**绑定一个 iroh 端点**（mDNS、套接字、关闭时还要等传输收尾），
而本机目录的答案根本不上网络。样本的双峰（1150/1133/1117 与 132/119）就是「绑了端点」与「没绑」两簇。

**修法**（提交 `3ce5c5e3`）：先解析机器，是本机就直接 `listed_with_arrivals`，不建端点。
release 二进制、无常驻、40 个文件、六次：**1354/1162/1168/1168/1186/1168 ms → 700/84/82/79/77/83 ms**
（首条是刚链接完的冷页缓存），其后复测 76–101 ms。约 **14×**。

### 2.3.1 打包发现：`cargo tauri build` 不重建 CLI

`release\khor.exe` 与 `release\khor-gui.exe` 的时间戳会相差很远：`tauri build` 只构建 app，
CLI 作为 `externalBin` 被原样打包进去。本轮因此出现过一次「安装包里的 CLI 是旧版」。
打包要么单独 `cargo build --release -p khor-cli`，要么走 `scripts/desktop_installer.py`
（它在打包前构建 CLI 并校验 app 与 CLI 的 build/contract 一致）。


### 2.4 `cli_doctor`：2.2×

`doctor` 含中继连通性探针，单条中继可耗到 2 s 级超时。两台机器的网络出口不同，
这行测的是「这台机器的网络」，不是 CPU 或代码。要可比，需固定探针目标或排除网络项。

## 3. Windows 10

**`NOT_TESTED`。** 本席位只有一台 Windows 11（26100）。归属：需要一台 Win10 x64 机器执行
`scripts/windows-smoke.ps1` 与 `scripts/windows-perf.ps1`。不把它写成 passed。

## 4. 本轮已做的优化（有前后对照）

| 项 | 前 | 后 | 证据 |
| --- | --- | --- | --- |
| 首屏 FCP（窗口内实测） | 1280 ms | **664–712 ms**（3 次预热后） | `first-contentful-paint`，见 [W5-GUI §6](W5-GUI.md) 与提交 `3056754b` |
| 首屏 JS 堆 | 51 MB | 32 MB | `performance.memory` |
| 慢命令阻塞轮询 | 单线程，一条 6.4 s 命令冻结所有轮询 | 4 worker：`doctor`（单跑 8.8 s）在飞时，`sessions` 轮询 312/190/176/174 ms | 传输层并发，提交 `7d067d49` |
| 会话轮询叠加 | 每 2 s 无在途保护，多次读数乱序落地 | 有在途保护（同 `fetchDevices` 的既有做法） | 同上 |

**冷启动方差**：重新链接后的第一次启动 FCP 读到 **2844 ms**（Defender 扫描新二进制 + 磁盘缓存冷），
随后三次为 712 / 664 / 672 ms。报告数字时用预热值，但要知道用户升级后的**第一次**启动更接近前者。
这也是 §5.5 那个方向（资源走 loopback）值得做的原因之一：它去掉 250 次预载请求与首屏的竞争。

## 5. 下一轮优化的量化清单（已由 bundle/性能分析测出，未实施）

启动图为 2.79 MB 入口，其中：应用代码 55.4%、markdown 栈 12.5%、React 10.9%、UI kit 9.5%、
xterm core 6.2%、shiki core 4.9%。可回收的部分（按可回收量排序）：

1. **详情区子树懒加载**（`ChatView` + `MachineChat` + `DelegatedSessionView` 一起放到一个 `lazy()` 边界），
   可把 markdown 栈 + shiki core 移出启动图，**约 18%**。⚠️ 本轮试过只把 `DetailPane` 的三处改懒：
   入口只从 2861 KB 降到 2857 KB，因为 `App.tsx:178` 仍静态引用 `MachineChat`，所以**必须先处理 App 的引用**
   （`retainKhorPromptTokens` 拆成小模块，`MachineChat` 在 App 里也懒加载）。已回退，未留半成品。
2. **设置/更多页懒加载**（`AppearancePage` 等），可移出 xterm core 与 react-simple-keyboard，**约 7.4%**。
3. **seti 图标 JSON 移出首图**（`FileKindIcon`），**约 2.9%**。
4. **`gen/catalog.ts` 按语言拆分**，**约 5.6%**（需改生成器，语言切换时再取另一份）。
5. **把资源也交给 loopback 服务**（`crates/web` 已有嵌入的 `ASSETS` 表）：页面与资源同源后
   可以完全恢复正常代码分割，并把 250 个 `modulepreload` 去掉。代价：页面 origin 变成
   `127.0.0.1`，需要 `capabilities` 的 `remote.urls`，且该 origin 的 localStorage 会重置一次。

## 6. 证据索引

| 项 | 命令 | 退出码 | 结果 |
| --- | --- | --- | --- |
| Windows 矩阵 | `pwsh scripts/windows-perf.ps1 -Khor <release khor.exe> -Gui <release khor-gui.exe> -Runs 5 -Warmup 2` | 0 | `F:\Temp\khor-perf\perf-windows.json` |
| Linux 矩阵 | `bash scripts/windows-perf.sh -b ./target/release/khor -s ~/khor-perf/perf-home` | 0 | `~/khor-perf/perf-linux.json`（LAB 192.168.3.6） |
| 首屏时序 | `.cache/w7-page-timing.ps1`（CDP `performance` + `paint`） | — | §4 表 |
| 并发 | 同上 §4；`doctor` 在飞时 4 次 `sessions` 轮询 | — | 312/190/176/174 ms |
