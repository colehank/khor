# Q-037-g GUI 回放只在成功后替换历史

状态：作者实现已固定提交，真实组件红绿完成；待root非作者审查/集成。Q041已由root接管，本任务不恢复旧写权。

## 现场与范围

工作副本 `/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-overwrite-recovery`。接手原分支 `fix/quality-native-helper-ownership` HEAD `adb868e016322d9b2541e1858801e47673855eba` tracked clean，保留该ref，原地从精确集成 `58450bbcfe55f86759528ba9e98354993c9b68a7` 创建 `fix/quality-replay-paint`。允许ChatView、最小有用fold模块、专用测试/夹具与本卡；未改全局package/gate/CI、协议/生成、布局/icon/token。无Cargo/下载/原生App/真实provider/SSH/上传。测试仅本地loopback真实React与受控Tauri invoke；生成catalog/ChatFrame只读使用现场既有产物，不重新生成。

已读中央STATE/PLAN、khor-design技能与设计README/states；本轮不调整布局。遵循已知内容加载不清空。用户当前纯开发协议，不新增legacy兼容逻辑；main不合并/ship。

## 图优先与实际事件链

Code review graph最小context与semantic search先行；`absorb history_end arriving ChatView replay`无命中，图索引旧于当前分支，实际文件回退核验：

- `crates/node/src/gui_host.rs` HostCompletion::Replay失败/越限发送ControlResult(operation=replay,error=Some)然后HistoryEnd；publish_history_to取消投递同样先error后end，可已有部分history入队。
- `apps/gui/core/src/chat.rs::frame_of`原样映射上述两帧，chat_poll按游标给frames及独立gone标志。pipe关闭可能只有gone=true，没有HistoryEnd。
- `apps/gui/src/api.ts::chatReplay`仅发chat_replay；ChatView poll先展示controlError，absorb原先无条件在HistoryEnd用arriving替换past，因此完成标记不能单独代表成功。

## 真实目标红

证据根 `/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q037-gui-replay`。

`red-01/report.json`：先通过真实poll交付完整history+end并在DOM确认，再分别交付空回放失败+end、部分history→下一batch错误→下一batch end。原生产ChatView在Chrome/WebKit各2例全部失败，共4/4目标红，pageerror均0；空失败清掉已有历史，部分失败将历史换成fragment。before/after source指纹稳定。未把graph静态疑点或setup失败冒充产品复现。

## 候选设计

Painting保留replayFailed，只有operation=replay且error非null才拒绝本次提交；立即丢本次arriving并拒收至对应end，跨batch保留失败标志。end后重置，后续成功回放可独立提交；成功空回放仍清空past。普通control错误与live折叠互不影响。新attachment effect丢未完历史片段，retired batch吸收完已成功结束的回放后再丢剩余partial；已有past/live不变。仅增加小量状态逻辑，不为不足30行另抽象fold模块。

既有batch.lost主动清空有洞paint并请求重放的策略不变，不把已丢帧的图声称为完整历史。此任务保护的是仍已知完整past与未完成回放的提交边界。

## 验证与待收口

新增正式入口 `apps/gui/scripts/check-chat-replay.mjs` 与 `scripts/fixtures/chat-replay-cases.mjs`，沿用已正式使用的真实ChatView/TooltipProvider/tabAct、production api.ts与held poll seam。页面无API调度重写，Playwright只控制时间与投递帧；所有未匹配命令返回本地空fixture数据，不连接用户服务。默认port23827，临时Vite目录在本树并finally回收，证据默认在Khor分类root。selector先验证非空/未知/重复再建目录，fingerprint包含ChatView/API/ACP projection/脚本/生成catalog与ChatFrame。

最终13场景：空失败、跨batch部分失败、取消投递、空字符串error、成功空、失败后成功再空、live交错、普通control错误、成功control result、Gone帧+flag重连、仅Gone flag重连、mid-replay真实tabAct重开/旧poll迟到、成功end先于Gone。必须root非作者复审后集成。


## 固定交付

提交 `1c76c680a4334212455ce9610fc9b35a667a47d3`，仅ChatView与两个新专用脚本/fixture，提交后tracked/untracked均clean。原Q043 ref未移动；无全局package/CI登记，由root处理。新增runner默认证据在本机Khor分类root，`CHAT_REPLAY_CHECK_DIR`可指定CI证据位置；不需要Cargo缓存/生成器运行，前置是现有依赖与本分支对应已生成catalog/bindings，缺失明确失败。

最终命令（从本工作副本）：

```sh
PLAYWRIGHT_BROWSERS_PATH=/Volumes/disk0/Khor/caches/dependencies/khor-playwright \
CHAT_REPLAY_WEBKIT=1 \
CHAT_REPLAY_CHECK_DIR=/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q037-gui-replay/final \
node apps/gui/scripts/check-chat-replay.mjs

PLAYWRIGHT_BROWSERS_PATH=/Volumes/disk0/Khor/caches/dependencies/khor-playwright \
CHAT_POLL_WEBKIT=1 \
CHAT_POLL_CHECK_DIR=/Volumes/disk0/Khor/evidence/runs/khor-code-quality-evidence/Q037-gui-replay/poll-regression \
node apps/gui/scripts/check-chat-poll.mjs

apps/gui/node_modules/.bin/tsc --noEmit -p apps/gui/tsconfig.json
git diff --check
```

实际：最终回放Chrome/WebKit各13，共26/26；原轮询各11，共22/22；两组pageerror全0，最终回放consoleErrors全空。最终report带7个来源before/after SHA，全部稳定；poll旧正式runner本身执行指纹稳定断言。types实际exit0，diff --check通过。空/未知/重复selector3/3非零退出，未新增临时fixture目录，详selector-validation.json。所有临时Vite目录finally清理，两个浏览器/server正常退出，无真实服务调用。

`red-01`原先4个目标红之后扩展了fixture覆盖，保留原red源码快照：source-reconstruction.json逐一核对当时report内runner/cases/原ChatView SHA完全一致，未将新fixture指纹追写旧report。`final/source/`保留与最终7源SHA相同的固定来源。`commit.json`核对提交、clean与最终3交付源码指纹。

边界：这是实际浏览器React+生产fold/API与受控transport，未宣称真实原生IPC/provider验收；后端错误/end序列来自实际源码链，未改协议。未知网络中断/gone不推断本次回放成功。原lost有洞重建策略未改；仅history_end成功完成才有权替换已有完整past。无main merge/ship。
