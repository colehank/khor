# Windows 迁移 W0 基线

本报告只描述当前 Host 分配的 `s2-windows` worktree，不复用其他 worktree 的执行结论。当前源码基线为 `d3781adce2e38bf966acd684233069e21c20a741`。本席位完成了仓库结构、manifest、Tauri 配置和 CI 的实读；没有通用 shell/exec 工具，因而没有把源码阅读写成产品构建或测试执行证据。所有缺少真实进程退出码的项目均明确标为 `NOT_RUN` 或 `BLOCKED_ENV`。

## 1. 隔离、Git 身份与修改边界

| 项目 | 当前观察 | 证据、状态与限制 |
| --- | --- | --- |
| 分配的工作树 | `C:\Users\admin\AppData\Local\Temp\steward-wt-315077e9\eb4776ab-59ab-55ed-93a7-cb0942bcef9a` | 当前 runtime context；工作目录为 Host 分配路径 |
| 分支 | `steward/eb4776ab-59ab-55ed-93a7-cb0942bcef9a` | `steward_status(task-1)` 返回的 task branch；本席位未创建或切换分支 |
| 起始 HEAD / base | `d3781adce2e38bf966acd684233069e21c20a741` | task contract、`baseline_observed` marker 和写入前 `steward_workspace diff` 一致 |
| 写入前 clean | `true` | Host `baseline_observed` marker；worktree fingerprint 为 `3290a7bf5b40585d3d0bf0b0787714122bb69389bb5120aee3164e868ddfe552`；写入前 diff 为空 |
| Git root | `NOT_RUN` | 未执行 `git rev-parse --show-toplevel`；角色工具没有 shell/exec，不能从路径命名推断 root |
| HEAD 的独立命令回执 | `NOT_RUN` | 未执行 `git rev-parse HEAD`；当前 hash 是 Host marker，不是本席位命令退出回执 |
| remote / remote HEAD / 默认分支 | `NOT_RUN` | 未执行 `git remote -v`、`git remote show origin` 或等价命令；CI 的 `main` 触发条件不代替 remote 观测 |
| 允许修改 | 仅本报告 | contract 的唯一 write scope 为 `docs/windows-migration/BASELINE.md`；不修改产品源码、Cargo manifest、锁文件或 CI |
| 隔离强度 | `partial` | contract 的 `minimum_sandbox_enforcement` 为 `partial`；角色工具提供 worktree 范围控制，但不是 OS 只读沙箱 |

可执行的初始 Git 身份闭环仍需 Host 在同一 worktree 中取得真实退出码：

```text
pwd
git rev-parse --show-toplevel
git rev-parse HEAD
git branch --show-current
git status --porcelain=v1
git remote -v
git remote show origin
```

上表中的 `git` 命令均为 `NOT_RUN`。没有将旧 worktree 的根目录、HEAD、branch、clean 或 remote 结论带入本报告。

## 2. 已读取的真实构建入口

### Rust workspace 与 CLI

根 [Cargo manifest](../../Cargo.toml) 声明 resolver 2，workspace 成员包含 `crates/cli`、`crates/web`、`apps/gui/core`、`apps/gui/src-tauri` 等，排除 `vendor`；workspace package version 为 `0.2.0`、edition 为 `2024`。这只是 manifest 事实，不是构建成功证据。

[CLI manifest](../../crates/cli/Cargo.toml) 的 package 名为 `khor-cli`，二进制名为 `khor`，入口为 `src/main.rs`，因此精确 CLI 构建命令为：

```text
cargo build --locked -p khor-cli --bin khor
```

该命令本轮 `NOT_RUN`。没有把 `cargo build` 的源码依赖解析当作执行结果，也没有运行 `cargo clean`。

### GUI 与 Tauri

[README](../../README.md) 要求先在 `apps/gui` 执行 `npm ci` 和 `npm run build`；README 说明 native/Web 构建依赖 `apps/gui/dist`。由于 contract 禁止安装依赖，本轮没有执行 `npm ci`。

[GUI package manifest](../../apps/gui/package.json) 的实际脚本关系为：

```text
工作目录：apps/gui
npm run build
  prebuild -> npm run gen
  gen -> mkdir -p src/gen && cargo run --locked -q -p khor-catalog --bin gen_ts > src/gen/catalog.ts && cargo run --locked -q -p khor-gui-core --bin gen_bindings
  build -> tsc --noEmit && vite build

npm test
  pretest -> npm run gen
  test -> npm run test:unit && npm run test:types

npm run test:types -> tsc --noEmit
npm run test:interaction -> manifest 中列出的交互检查链
```

因此 GUI build 是 Rust 生成步骤、TypeScript 类型检查和 Vite 构建的组合，不能简化为单一 Vite 命令。`mkdir -p`、重定向和生成文件写入也尚未在 Windows shell 中实测；不把它们写成已知失败。

[Tauri 配置](../../apps/gui/src-tauri/tauri.conf.json) 指定 `frontendDist: "../dist"`、`beforeBuildCommand: "npm run build"`、开发地址 `http://localhost:1430`，且 `bundle.active` 为 `false`。这些是配置事实；没有运行 Tauri build，也没有验证 Windows WebView2、安装包或签名。

### Agent

[Agent package manifest](../../apps/agent/package.json) 声明：

```text
npm test -> tsc --noEmit && node --test test/*.test.ts
npm run build -> node build.mjs
```

本轮没有执行 Agent 的依赖安装、build 或 test。目标平台和 Bun 产物必须以 [Agent build script](../../apps/agent/build.mjs) 与 CI 实际回执为准；仅有 manifest/脚本存在证据不等于 Windows sidecar 已生成或可运行。

## 3. 声明的工具链与本机实测

| 工具或平台 | 仓库/CI 声明 | 当前 worktree 本机实测 |
| --- | --- | --- |
| Rust / cargo | [quality workflow](../../.github/workflows/quality.yml)、[frontend workflow](../../.github/workflows/frontend.yml)、[native-tools action](../../.github/actions/native-tools/action.yml) 使用 Rust `1.95.0` | `rustc -vV`、`cargo -V`、`rustup show` 均 `NOT_RUN` |
| rustfmt | [工程约定](../code-quality/ENGINEERING.md) 指定 `1.9.0-stable` | `rustfmt --version` `NOT_RUN` |
| Python | [工程约定](../code-quality/ENGINEERING.md) 要求 Python `3.11+`，格式入口为 `python3 scripts/check-rust-format.py --check` | `python --version`、`python3 --version` 均 `NOT_RUN` |
| Node | CI 使用 `22.19.0` | `node --version` `NOT_RUN`；`{node}` 的解析路径未取得 |
| npm | GUI 与 Agent 均有 lockfile；CI 使用 `npm ci` | `npm --version` `NOT_RUN`；没有安装依赖 |
| Bun | native-tools/Agent CI 声明 `1.3.14` | `bun --version` `NOT_RUN` |
| Windows | workflow policy 为 `win32`；Windows job 使用 `windows-2025` | Windows 版本、build、架构、MSVC linker、Windows SDK、WebView2 均 `NOT_RUN` |

上述版本是仓库声明值，不是本机版本。没有版本命令的真实 stdout、stderr 和退出码，因此不得写“工具链已满足”。

建议由 Host 在允许执行命令的验证环境补齐：

```text
rustc -vV
cargo -V
rustup show
rustfmt --version
python --version
node --version
npm --version
bun --version
```

如果任一工具缺失，应记录为 `BLOCKED_ENV`，不安装依赖、不改变工具链决策。

## 4. CI 实读与精确命令

### 前端 workflow

[frontend workflow](../../.github/workflows/frontend.yml) 在 `ubuntu-24.04` 上固定 Rust `1.95.0` 与 Node `22.19.0`，实读到的关键命令为：

```text
python3 scripts/release_manifest.py version
npm ci --prefix apps/gui
npm run build --prefix apps/gui
python3 scripts/khor_lab/fixture_data.py .
python3 scripts/release_manifest.py create --root frontend --kind frontend
```

这些是 CI 配置中的命令，本轮没有运行。CI runner、缓存和 artifact 上传也不能代替当前 Windows worktree 的本机执行回执。

### Quality workflow

[quality workflow](../../.github/workflows/quality.yml) 包含 scripts、Agent、GUI、node-native 和 Windows process 等多个 job，不存在一个可将所有质量范围等同起来的单命令。实读到的代表性命令包括：

```text
python3 -m unittest discover -s scripts/tests -p 'test_*.py'
node scripts/check-native-io.mjs
cargo build --locked -p khor-cli --bin khor --message-format=json
cargo test --locked -p khor-node --lib --no-run --message-format=json
cargo test --locked -p khor-node --lib acp_process::tests -- --test-threads=1
cargo test --locked -p khor-gui-core --test terminal_current -- --test-threads=1
cargo test --locked -p khor-cli --test runtime_image -- --test-threads=1
npm run test:unit && npm run test:types
npm run gates && npm run mobile:static
npm test --prefix apps/agent
```

本轮以上命令均为 `NOT_RUN`；未声称任何构建、测试或质量 job 通过。

### Windows 专项 CI

同一 [quality workflow](../../.github/workflows/quality.yml) 的 `windows-agent-process` job 使用 `windows-2025` 与 Rust `1.95.0`，实读到的 PowerShell 命令为：

```powershell
cargo run --locked --manifest-path crates/node/tests/windows-process-harness/Cargo.toml *> "$env:RUNNER_TEMP/agent-process.log"
rustc --edition=2024 --test crates/node/tests/fanout_harness.rs -o "$env:RUNNER_TEMP/gui-fanout-tests.exe"
& "$env:RUNNER_TEMP/gui-fanout-tests.exe" --nocapture --test-threads=1 *> "$env:RUNNER_TEMP/gui-fanout.log"
```

CI 配置会保留 `$LASTEXITCODE` 并上传日志；本轮没有对应 GitHub job receipt，也没有在本地 Windows shell 执行这些命令，状态为 `NOT_RUN`。harness 实现正文未作为本轮必要基线读取，不能推断断言数量或通过数。

### 发布范围

当前已读 [release workflow](../../.github/workflows/release.yml) 的引用来自仓库现有 CI；本轮没有执行发布、签名、公证、安装或上传操作。任何 Windows 发布矩阵结论都必须以该文件和实际 artifact 查询为证据，不能以 Windows CI job 存在推断为已发布。

## 5. 执行证据与环境边界

| 操作 | 状态 | 真实证据能证明什么 |
| --- | --- | --- |
| Host baseline marker | `OBSERVED` | 当前任务分配的 base/head、branch、clean 和 worktree fingerprint |
| `steward_workspace map` | `EXECUTED` | 返回当前 revision `d3781ad...`，总文件数 2413；返回结果截断，不能视作全仓已读 |
| `steward_workspace diff`（写入前） | `EXECUTED` | base=head=`d3781ad...` 且 diff 为空；不是 shell `git status` 回执 |
| 指定文档、manifest、Tauri 和 CI 的 `steward_workspace read` | `EXECUTED` | 建立上文的静态配置事实；工具没有启动进程，因此没有退出码 |
| `pwd` / Git root / remote 命令 | `NOT_RUN` | 无本席位命令输出；Host 必须补齐真实 Git 身份闭环 |
| Rust/Node/npm/Bun/Python 版本命令 | `NOT_RUN` | 无本机工具版本证据 |
| `npm ci` 或其他依赖安装 | `NOT_RUN` | contract 明确禁止安装依赖 |
| GUI build / CLI build / Cargo test / GUI test / Agent test | `NOT_RUN` | 本席位没有通用 shell/exec；没有真实退出码、日志或产物 |
| Windows harness / fanout | `NOT_RUN` | 没有本地命令或 GitHub job receipt |
| 产品格式检查 | `NOT_RUN` | 未执行 `python3 scripts/check-rust-format.py --check` |
| contract acceptance | `HOST_PENDING` | Host 应执行 contract 指定的 doc acceptance，并保留完整命令、cwd、退出码和输出；本席位不能伪造 receipt |

本席位实际可用的角色工具只有受限 worktree 读写、map/diff、状态读取、方法读取和 Host 自检；没有通用 shell/exec。`danger-full-access` 是运行时文件策略，不是产品命令已执行的证据。隔离范围最多按 contract 要求记录为 `partial`。

## 6. 最小验证闭环

1. Host 在当前 worktree 取得 `pwd`、Git root、`git rev-parse HEAD`、branch、`git status --porcelain=v1`、common-dir、superproject、脱敏 remote 和默认分支；禁止 `git reset`、`git clean`、`git init`，提交后再次确认 clean。
2. Host 取得 Windows 精确版本/架构、Rust/cargo/rustup/rustfmt、Python、Node、npm、Bun、MSVC linker、SDK 和 WebView2 版本。缺失项标为 `BLOCKED_ENV`，不通过安装来消除证据缺口。
3. 在已有依赖且允许生成输出的隔离范围，按 [README](../../README.md)、[GUI package manifest](../../apps/gui/package.json)、[CLI manifest](../../crates/cli/Cargo.toml) 和 CI 先运行前端 build，再运行 `cargo build --locked -p khor-cli --bin khor` 及有来源的测试；每项保留命令、cwd、退出码、stdout/stderr、日志 SHA、产物路径和清理结果。当前本席位所有产品执行项均为 `NOT_RUN`。
4. Host 执行 contract acceptance：

```text
{node} "F:\#Gong\#AI4S\khor\meta-control\.dist\dsh-steward-0.10.122\scripts\doc-acceptance.mjs" --test "docs/windows-migration/BASELINE.md"
```

该验收与 workflow 的 `self_check.commands: []` 是两个不同字段；空自检不能证明文档 acceptance 或产品构建通过。

5. 最后确认本提交只包含 `docs/windows-migration/BASELINE.md`，Host commit receipt 给出新的 HEAD，正式 self-check 和 acceptance receipt 均可追溯，且工作树 clean。

本轮交付只更新 W0 文档和证据边界，不作架构、依赖、签名、打包、发布或 Windows 全量可用性决策。后续构建/测试基线需要具备命令执行能力的 Host/Verifier 补齐。
