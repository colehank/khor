# Q037 R0：六个 Stop 超过四槽后的取消投递首红

所有者：mcp_worker_owner。状态：**同源构建通过、精确1普通父取得目标业务首红，清理全通过，唯一槽已归还**。2026-10-27（UTC冻源20261026T162252Z，run163311Z）。本卡只测known-task取消投递；R1持久intent/公平step/receipt与R2未知task/late-accept fence仍开放，见 `Q-037-remote-receipts.md`。

## 分支、授权和源码

- 唯一写树 `/Volumes/disk0/Khor/worktrees/current/khor-quality-agent-protocol`。开始时核 `fix/quality-mcp-cancel@3e676325daab05d513133c859e295393cd54ad0e` tracked clean、旧自有native PID无残留；保留 `refs/heads/audit/mcp-worker-106-20261027` 指向原commit。
- 本次新分支 `fix/quality-remote-cancel` 从实际集成 `f721a2c3ac0e12cb1faf5dd9634015dea1054a4b` 创建，不新建worktree，不改main/ship。
- 仅新 `crates/node/src/khor_agent_remote_cancel_tests.rs`、新 `crates/node/src/fixtures/remote-cancel.py`；`khor_agent.rs`仅追加4行cfg(all(test,unix)) mod入口。原106测试及fixture未改，生产行为未改。
- graph impact先调用；图明确built7550、head f721不匹配，仅作结构线索，不能据1048影响/500截断结果声称新源覆盖。当前无.codegraph，回退实际source。
- 冻结：`Q037/remote-red/candidate-20261026T162252Z/source-freeze.json`。khor_agent SHA256 `03289b0f471cbe0015b2295763b7945093b39f71a57d0ecefd81d26e01c12fa8`；Rust test `32cf0c86b3e8d1f07623467cbc1189087aeff2cac19bfe6258213761b7a9bac9`；Python `f9e7760583b86bd3809504573ba60827fb1b492a5c65a4f6f18dd6412956309a`。每文件原字节+cfg diff均已保留。
- 内盘副本 `/Users/zgh/Library/Caches/khor-code-quality/recovery/inflight-source/Q037-REMOTE/r0-20261026T162252Z`。fresh外盘6,731,739,136B、内盘12,894,294,016B，均高于5GiB。源码检查不等于授予Cargo槽。

## 真实调用链与精确oracle

普通父测试 `khor_agent::remote_cancel_tests::six_stopped_runs_eventually_deliver_remote_cancel_after_four_held_owners_retire` 只启动一个隔离libtest子进程 `remote_cancel_worker`（ignored内部worker，不计另一个distinct普通父），使进程全局SWEEPS从空状态开始，避免旧测试handle干扰。要求 `KHOR_TEST_MCP_EXECUTABLE` 为本轮同源fresh CLI绝对路径；不使用已安装release或fake worker。

1. 同一private home真实 `Node::khor_agent_here(Prompt)` → Python合作Pi收到正式start；等ready后，生产 `Store::accept` + `Execution::claim/update(Working)`构造受控peer record，再经正式 `delegation::track_child` 写该实际run的durable known-task link。**这里只覆盖生产落盘同路径，不声称完整真实delegate网络接受或ACP执行。** 每task绑定实际parent、caller、home；一个run一个独立task。
2. 真实Stop六个不同run。每次等Saved cancelled、Running map absent、executor claim可得、该Pi PID ESRCH且确实收到abort。前四个真实sweep worker经正式隐藏CLI入口读实际schema，再经 `cancel_children → Node::delegation_on → cookie IPC Handoff/Get`；peer回Working后必须实际收到同task的Handoff/Cancel。四个Cancel reply由按task index的屏障扣住。
3. 前四个每个都确认真实Cancel到达后才推进下一个run，不假定read_dir或线程顺序。第五/六本地同样收口；现有pending/not_admitted仅日志诊断，不作为最终oracle，也不写死成未来公平实现的绿障碍。第六收口时确认四个exact fresh CLI直接子PID都仍存活，且第一屏障至此严格<4s；若5s保险抢先退休导致现场无四槽，判**setup失败**，不报业务红。
4. 释放前四个Cancel reply并等原四个CLI PID全部ESRCH。随后只驱动正式 `reconcile_agent`（serve startup也用此入口）和普通 `Action::Read`，最长8s；不再Prompt/Stop、不直调schedule_cancel、不读全局Vec长度/STARTED计数。此窗口也超过原owner的全部5s有限预算。
5. **唯一业务oracle**：第五/六个task实际Handoff/Cancel最终都应进入受控peer。初始源审预期缺少二者；本卡末尾本轮实际运行已在此断言得到 `BUSINESS RED R0`。只证明IPC投递/admission gap，不证明远端ACP终态receipt、mesh权限全链或取消成功。

## 失败前回收与独立保险

- 主exercise所有Result及caught panic先进入显式cleanup：release全部屏障，请求当前真实Running停止，等map/claim drain；停止listener并join它拥有的每个连接handler；核所有记录Pi及exact CLI工作进程消失，之后才断言业务结果。
- Pi自身25s alarm，MCP sweep原5s保险保持；IPC读取3s、held reply15s、listener40s有限。外层隔离libtest有55s wall fuse及独占PGID，kill后wait。外层每250ms只观察该子进程的exact CLI直接子PID；故障兜底只在命令逐字匹配且getpgid(pid)==pid时kill自有group，再等ESRCH，不按裸PID/宽匹配杀进程。Python PID来自本夹具私有记录，兜底仍核exact命令/PGID。
- 正常路径报告actual worker reap与map/claim分别的证明；不声称CLI ESRCH等于远端cancel confirmed。Rust SWEEPS内部JoinHandle不由测试伪造读数；有限owner由原worker结果退休，8s正常生命周期窗口超过其总预算；进程退出OS回收不表述为“全部后台handle均显式join”。fixture自己拥有的listener/handler显式join。
- 测试scratch最终删除，阶段/实际task/PID/metadata/回收结果输出到父测试日志；执行runner须保完整日志及二进制SHA，并对失败classified setup/cleanup/business。无300s默认IPC残留。使用短TMPDIR规避Darwin agent socket长度；该准备源尚未执行任何native fixture。

## 静态门与待执行

已通过 `rustfmt --check --edition 2024 --config skip_children=true`（两Rust源）、`git diff --check`、Python `ast.parse`（不执行fixture、不写pycache）。只三路径source差异。

初始准备轮未Cargo/fixture。root独审无阻断后于下一turn明确授予唯一槽，按下述同源闭环完成；结束已归槽，不自行扩测/改生产。任何编译/fixture setup错误应停轮修源、新freeze，不冒充业务红；本次没有此类错误，也没有放宽屏障、时间或oracle。

## 实际首红：run-20261026T163311Z

证据根 `Q037/remote-red/run-20261026T163311Z/`，主收据 `first-red-receipt.json`，原始日志 `exact-r0/command.log`。按root批准，`pipeline.py`中显式 `STEP_TIMEOUT=180`、`TOTAL_TIMEOUT=360`，launch前以AST逐常量断言独立且step<total。每步实际 `subprocess.run(check=True)`，失败立即停止；质量target/jobs1/incremental0/lockedoffline、短自有TMPDIR、两盘5GiB/三次I/O/仅自有后代PGID守卫均生效。

| 阶段 | 实际结果 |
| --- | --- |
| 源码preflight | 三源原freeze SHA全同；khor_agent生产原字节为前缀、只追加cfg mod；1914 tracked+新测试输入记录。419第一方Rust仅mtime更新，逐文件SHA前后相同，实际当前树重编。 |
| frontend | 标准本树 `npm --offline --prefix apps/gui run build`，87.90s绿；真实gen_ts/gen_bindings→tsc/Vite。未借旧dist；已审locked cache两lock SHA核同。155 gen+277 dist每文件hash及producer两binary SHA/mtime保存。 |
| CLI | `cargo build --locked --offline -j1 -p khor-cli --bin khor`，11.14s绿，当前树CLI实际重编。 |
| Node libtest | `cargo test --locked --offline -j1 -p khor-node --lib --no-run`，53.58s绿，日志明确当前agent-protocol树Compiling khor-node。 |
| 精确1普通父 | 11.73s，目标业务assert失败；ignored内部子fixture11.47s，不另计distinct父。其余1011 filtered out，不把106旧批扩跑进本轮。 |
| 守卫 | 总165.47s<360；所有阶段无resource-stop、无I/O错误；最小外盘6,372,880,384B、内盘12,868,730,880B，均>=5GiB。 |

当轮actual CLI `/Volumes/disk0/Khor/caches/rust/khor-quality-target/debug/khor`：SHA256 `efcce3e14eb59cc0e05230a95a6bcce03cd20bfb377863fdc8c25470cf268af6`，257,288,312B，mtime_ns1793032614842007374。实际Node libtest `.../debug/deps/khor_node-73af34573690fd43`：SHA256 `b2ec51a0ab013f6342805480104161dd29dc520000a67259ae787b0f1113a811`，229,287,064B，mtime_ns1793032668242957624。两binary运行前后SHA+mtime全同，全部2347当前输入before/after全同，三源未修动。

真实阶段日志：六个Pi PID 19996/20037/20044/20049/20056/20057全部收到abort并ESRCH；六个Saved均cancelled、map=false、claim=free。前四个actual Get/Cancel到达，四个exact CLI worker 19999/20038/20045/20052同时仍live且回包被扣；第五/六metadata为not_admitted。释放后四CLI全ESRCH；正常Read/reconcile8s仍只有4个actual Cancel，缺失task `01a17913-566a-7649-9b61-d6e3dd764ae4` 和 `01a17913-570d-72c3-928b-06cd074e75c6`，最终触发原业务断言。

**断言之前**日志三项全Ok：IPC listener/所有handler显式join、map/claim drain、全部Pi/CLI ESRCH。outer child wait完成，Pi/native ESRCH=true、cleanup=[]；守卫owned groups全部无残留。实际target为admission/自动补投递失败，不是compile/setup/cleanup错误；不把controlled peer的合成Canceled回包当实际ACP终态证明。

只清理由本runner `mkdir(exist_ok=False)` 创建的 `/Volumes/disk0/Khor/caches/tmp/r0`：先存全路径inode/nlink/mtime/SHA，lsof确认无open handles，250个独占文件共3,362,456B，清理后目录absent。未碰其它cache/用户资料/source/release/App；`owned-scratch-before-cleanup.json`、`owned-scratch-cleanup.json`保留。runner/所有记录Pi、worker和fixture子进程fresh kill(0)均ESRCH，清单在首红receipt。源码仍三路径未提交，无本轮production实现、无main/ship。

## 首红提交

root最终证据审确认后授权仅提交三测试路径：`f0104d738f0de3162ea908d8ed672603ce0366d5`，base f721a2c3，保 `refs/heads/audit/remote-cancel-r0-red-20261027`；branch tracked clean。`commit-source-receipt.json`逐文件核tested freeze=blob=worktree，diff仅三路径，其余106源码未变。无生产/Cargo/main/ship。
