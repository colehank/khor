# Windows W1 CI 基线

## 1. 现状

`.github/workflows/quality.yml` 已包含 Windows 专项 job：

```yaml
windows-agent-process:
  runs-on: windows-2025
  # PowerShell
  cargo run --locked --manifest-path crates/node/tests/windows-process-harness/Cargo.toml
  rustc --edition=2024 --test crates/node/tests/fanout_harness.rs -o "$env:RUNNER_TEMP/gui-fanout-tests.exe"
  & "$env:RUNNER_TEMP/gui-fanout-tests.exe" --nocapture --test-threads=1
```

同时 `.github/workflows/frontend.yml`、`quality.yml` 声明 Rust 1.95.0、Node 22.19.0。

## 2. 本轮验证

- 已读取 `.github/workflows/quality.yml`、`frontend.yml` 与 `.github/actions/native-tools/action.yml`；
- 本地按 Windows 等价路径完成：
  - 前端生产构建（tsc + vite）成功；
  - `cargo build --locked -p khor-cli --bin khor` 成功；
  - 诊断入口成功。
- **远端 GitHub Actions 未执行（NOT_RUN）**；本机不是 windows-2025 GitHub runner。

## 3. 本地等价执行

### 3.1 Windows process harness

```text
cargo run --locked --manifest-path crates/node/tests/windows-process-harness/Cargo.toml
exit 0
```

12 项 PASS（argv/环境/cwd/stdin/stdout/stderr/退出码、缺失程序、batch/NUL 拒绝、Job Object 终止、取消阻塞、deadline、MCP breakaway、HANDLE_LIST lease、runtime groups）。

### 3.2 fanout harness

```text
rustc --edition=2024 --test crates/node/tests/fanout_harness.rs -o F:\Temp\gui-fanout-tests.exe
exit 0

F:\Temp\gui-fanout-tests.exe --nocapture --test-threads=1
9 passed; 1 failed
```

失败项：

```text
fanout::tests::a_history_producer_waits_for_real_capacity_without_retiring_the_reader
```

隔离重跑该测试为 ok，说明是 Windows 顺序相关 flake；远端 windows-2025 CI 的同一测试为 10/10 PASS。

## 3.3 远端 GitHub Actions

独立分支已推送并触发 workflow：

- Run: https://github.com/GongZhengxin/khor/actions/runs/43557690271
- Head: `03d861ca2399724313cefcbe29c526b6e0f899bc`
- 结论：success
- 步骤：Frontend build success；CLI build success；CLI version success；doctor JSON success；process harness success；fanout build/run success（10/10）
- 日志：`F:\khor-w1-evidence\ci-run2.log`

这次运行是真实 `windows-2025` runner，不是本地模拟。

注意：run 43557690271 对应 rebase 前 commit `03d861ca`。分支已 rebase 到最新 main `9a4219ff`，rebase 后 HEAD `574cce3b` 的 Windows W1 workflow 已再次运行：

- Run: https://github.com/GongZhengxin/khor/actions/runs/43559639618
- Head: `574cce3b20826f62f85c28ce9d87f9bffda782ff`
- 结论：success（Frontend、CLI、doctor、process harness、fanout 10/10 全部通过）

这是基于最新 main 的最终 W1 远端证据。

## 4. W1 判定

- CI 配置文件存在且 Windows job 可识别：**PASS（静态）**
- 本地构建等价路径：**PASS（本地）**
- process harness：**PASS（12/12）**
- fanout harness：**PASS（远端 10/10；本地一次顺序相关 flake）**
- 远端 CI 真实执行：**PASS（run 43557690271 success）**
- Windows 10：**NOT_RUN（按用户决定延后）**
- 不把静态 YAML 当作 CI 绿灯；本次有真实 run 回执。



## 4. W2+ 待办

- 在真实 windows-2025 runner 上执行 `quality.yml`；
- 前端 job 与 Tauri build job；
- 让 CI 使用无 `#` 的构建路径或修正 Vite 路径假设；
- 记录 artifact hash 与 job URL。
