# Q052 当前终端协议：定向验证准备（未运行）

Owner：node_test_review。2026-10-27。仅本验证卡及专用 evidence runner 写权；`Q-052-terminal-current.md` 实施卡最终冻结写权已归 root。不能自审自己实现。

## 冻结前提

- 树：`/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-store-owner-recovery`，branch `fix/quality-terminal-current`，HEAD `13a9459f6c2dfd8d12912e6e2325b58de6e06999`。candidate01 12 路径（11 当前源+1删除）继续冻结；无源码/依赖/生成输出改动、无 commit。
- 1921 完整输入本次重新逐 SHA 核等于 candidate01；28 个候选文件外内盘逐 SHA 一致。manifest `81165938195fc3c14494e5db87995e10cf78f59ee036f156c6601f2598914fbe`；patch `42035b0dcdd8315cdf50269be70d11866a6e382ce75d0fc3f09030e03377f0e6`。
- 根独立源码审暂未确认新增阻断；这不是编译或运行通过。本准备只做读取、SHA、AST 解析、写计划/runner；没有 Cargo/native/PTY/tmux/gen/App 执行。
- 图先行已在实施阶段做过；图未覆盖当前未提交实现时以实际源为准，不以空关系/0 flow 当覆盖。

## 最小运行集

23 个不同普通父，9 个新定向父包含在内；没有 ignored worker 另计。12 种错误首帧输入仍只算所属一个 CLI 父。旧 ignored 手工旧 host 探针被替换，不算通过。

| 运行组 | 实际 target / 过滤 | 普通父数 | 边界 |
| --- | --- | ---: | --- |
| cli-current | `attach_version` | 2 | 实际 khor CLI→detached current host→真实 PTY；拒绝错误首帧后当前控制连接仍可用；Forget 真历史。 |
| core-current | `terminal_current` | 2 | 实际 term_open/history/rendererReady/Forget/key/resize API→自有 TCP peer；同 socket；不是 PTY 或 App。 |
| host-handshake | `khor_node` / `host::handshake_tests::` | 5 | 真实 TCP；current Version5 first、错误 Welcome/cookie、真实15s silence、150ms whole-frame四输入。 |
| core-io | `khor_gui_core` / `term::io_tests::` | 3 | 原3个 wire/order/queue oracle，仅fixture删version字段。 |
| gui-greeting | `gui_greeting` | 4 | 共享 Welcome helper 的原4 TCP父，含真实15s silence。 |
| snapshot-history | `host_snapshot` | 2 | 实际 host+sh PTY，model snapshot/history/modes 与原延迟门保留。 |
| replay | `host_replay` | 1 | 实际 host+sh PTY，ring trimmed 后模式仍正确。 |
| phone-ready | `open_from_phone` / `a_shell_opened_from_a_phone_is_on_the_phones_list_when_the_open_answers` | 1 | 两个私有 Node homes+hermetic relay+自 reexec host+PTY；Ready前不startup、Ready/reconnect只startup一次。custom harness只有此1父。 |
| private-tmux | `host_shell_tmux` | 2 | 真实PTY内私有 tmux -L khor-test-57/58-PID；私有 TMUX_TMPDIR；Drop只kill自己的-L。不接触用户 tmux。raw出现skipped即停，不计绿。 |
| cli-native-store | `open` / `a_private_host_does_not_inherit_an_unrelated_native_agent_store` | 1 | 只跑已机械修改的1个 CLI open父；实际 host/PTY，独立vendor home，不启动vendor App。 |

逐父精确名单（运行必须与原始 log 的名字和 passed/failed/ignored 汇总相符；custom phone harness核自己的实际1父输出）：

- **cli-current**：`the_real_host_rejects_missing_empty_old_future_and_wrong_first_ops`, `a_current_client_clears_the_host_history_for_later_attachers`。
- **core-current**：`core_rejects_noncurrent_welcomes_without_registering_an_attachment`, `core_sends_history_and_only_renderer_ready_then_forgets_and_continues_on_one_socket`。
- **host-handshake**：`host::handshake_tests::a_current_client_sends_version_five_before_any_input`, `host::handshake_tests::empty_old_future_unknown_and_gui_welcomes_are_protocol_errors`, `host::handshake_tests::a_cookie_refusal_keeps_the_hosts_reason`, `host::handshake_tests::a_host_that_never_welcomes_is_given_up_on`, `host::handshake_tests::the_first_frame_deadline_covers_silence_partial_prefix_partial_body_and_drips`。
- **core-io**：`term::io_tests::input_resize_and_clear_preserve_the_actual_host_protocol_order`, `term::io_tests::a_cancelled_queued_paste_writes_no_terminal_input`, `term::io_tests::queued_input_cannot_cross_a_closed_and_reopened_terminal_id`。
- **gui-greeting**：`current_gui_greeting_keeps_a_raw_stream_without_terminal_negotiation`, `empty_unknown_terminal_and_other_gui_greetings_are_rejected`, `a_refused_gui_handshake_preserves_the_hosts_reason`, `a_silent_gui_host_obeys_the_shared_handshake_deadline`。
- **snapshot-history**：`a_current_attacher_gets_the_snapshot_not_the_ring`, `a_re_attach_keeps_the_history_the_face_asked_for`。
- **replay**：`a_replay_trimmed_past_the_modes_still_starts_in_them`。
- **phone-ready**：`a_shell_opened_from_a_phone_is_on_the_phones_list_when_the_open_answers`。
- **private-tmux**：`a_tmux_typed_in_a_shell_is_not_handed_out_black_after_a_narrowing`, `a_shells_own_tmux_client_is_found_by_its_terminal`。
- **cli-native-store**：`a_private_host_does_not_inherit_an_unrelated_native_agent_store`。

`native_handoff` 仅编译当前已机械适配调用；它运行依赖显式真实 vendor conversation，默认跳过不能记绿，当前不授权运行。不包括用户 App、实际 Tauri→WebView 终端链、物理 iPhone/IME、Windows；这些仍需后续验收。

## 构建与执行顺序

三个必要 `--no-run` 构建产生实际 binary，随后直接按上表执行实际路径；不单独无故 cargo check、不嵌套 Cargo。CLI构建覆盖当前 normal Node/commands/CLI，core构建覆盖 normal gui-core；Cargo JSON 必须 manifest_path 指向本树、所需首次 normal/test artifact fresh=false。测试执行期间的 CLI/phone host只启动已经构建的当前 binary，不会调用Cargo。

```text
cargo test --locked --offline -j 1 -p khor-cli --test attach_version --test open --no-run --message-format=json-render-diagnostics
cargo test --locked --offline -j 1 -p khor-gui-core --lib --test terminal_current --no-run --message-format=json-render-diagnostics
cargo test --locked --offline -j 1 -p khor-node --lib --test gui_greeting --test host_snapshot --test host_replay --test open_from_phone --test host_shell_tmux --test native_handoff --no-run --message-format=json-render-diagnostics
```

- 精确构建目标共11个测试可执行产物（node lib、core lib、8 runtime integration、native_handoff compile-only；以 JSON实际记录为准）。运行10组合计23父。先3次构建，再运行；第一批当前源 `fresh=false` 后各二进制保存全 SHA/size/mtime，所有后续 phase 与最终核不变。
- 构建前仅对全部 first-party Rust作 byte-stable mtime refresh 以防共享 target跨树 stale artifact；完整输入 SHA 前/每phase/后均必须同冻结1921，不修改字节。此动作尚未执行。
- 若编译/fixture失败，保留首个 rawlog并立即停；编译红、环境/fixture错误、磁盘/超时 guard 与业务 assertion失败分别报告，不把任意 Err/0 tests当通过。

## 受控执行与回收

- 已准备 runner/execution-data 的准确外内路径见下。只有 root另行授唯一槽后可执行 `python3 run.py --execute-authorized`；准备阶段不执行或 import runner。启动再核HEAD/branch/1921 inputs/双盘28候选/AST常量和无cargo/rustc/gen_bindings在跑；任意不符停报。
- 具名常量 `STEP_TIMEOUT_SECONDS=300`、`TOTAL_TIMEOUT_SECONDS=900`、`MIN_FREE_BYTES=5368709120`；AST确实解析这些赋值，不只文字声明。每250ms动态核外/内空间与总时限；单组独立 session/PGID；超线只TERM/KILL本组，wait 3/5s，有限3+3s drain。被迫drain不算绿。
- 固定 jobs1/offline/locked/incremental0/shared target `/Volumes/disk0/Khor/caches/rust/khor-quality-target`；独占短scratch `/Volumes/disk0/Khor/caches/tmp/t52v1`，private HOME/tmp/TMUX_TMPDIR；保留原Cargo/Rustup cache路径供离线工具用，移除KHOR_HOME/session、TMUX和旧fixture环境。不启动用户 App/用户tmux/常驻服务，不新target/worktree。
- fixture正常Drop负责实际detached host、PTY child、私有tmux退出；runner采样自己PGID/后代、私有scratch命令和精确本父tmux tag的PID+start身份，逐phase及最终核已退，普通父与内部host/relay不双计。guard只杀自己PGID；若任何已观察detached owner仍活，停报并保留scratch，不能宣称槽/cleanup已齐，root依据实际身份另收口。
- 清理仅精确自有scratch，先完整列举，全部regular/dir且无symlink、无live owner才删除；遇socket/未知项保留原件报未清，不扫前缀/用户目录。退出组、源、binary、scratch、最小空间、原log与父名单/guard、双盘逐SHA收据分开记录。
- 构建11产物的空间需求没有先验假绿；5GiB启动门不保证足够余量，全程guard为准。停线后不自行清target或重跑。

## 已准备工件

外盘：`/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q052/terminal-validation-plan01`。内盘：`/Users/zgh/Library/Caches/khor-code-quality/recovery/Q052/terminal-validation-plan01`。

- `run.py` SHA256：`c0ae5dbb581fc4e41f3705fcc4660af9082683f0b81706fd81cf09380f143f92`。
- `execution-data.json` SHA256：`1e56d5ec2b40ff97a6abba101411161d846432c6e1022c90f09fa2f6dfe04b16`。
- `preparation-audit.json`：AST 300/900/5GiB、23唯一父、1921 current=candidate01、候选28双盘齐；只查到了 tmux 路径 `/opt/homebrew/bin/tmux`，没有执行tmux，不称当前可运行。
- 预留真正执行回执 `Q052/terminal-green01`（当前不存在）；未经另授槽不得创建执行结果或冒运行通过。
