# 第三轮实现与验收交接

本轮四项代码及用户首轮验收修正已完成，Root单独执行并自审；当前等待再次体验验收。没有合并 main、ship、重启旧 GUI 或接管用户会话。代码基线为 main27a9ce5c；本轮分支为 `integrate/round3`。

## 交付

| 项目 | 代码提交 | 结果 |
| --- | --- | --- |
| R3-01 终端链接 | dee0caab | 鼠标模式中认出的链接由Khor持有点击；单击内部，Cmd/Ctrl主按钮为系统意图，拖选/程序鼠标各有归属 |
| R3-02 会话药丸 | d6114c10 | 恢复关系由原药丸表达，健康状态安静，连接统一呼吸；自有GUI安全进入时续接，未知所有权/未决intent/Shell新运行仍守原资格门 |
| R3-03 工作区导航 | 722120b2 | 两入口共用导航行，保留原session标签栏与甲板；bounded history、每次提交实体标签、预览/保留与真实编辑器草稿 |
| R3-04 预览标题 | f57cae56 | 临时文件/提交标题斜体，无空心圆；双击/编辑保留后正体，副标题/未读/未保存标记独立 |

详细实现与边界在各 [任务卡](tasks/R3-03-workspace-navigation.md)。临时预览归属是当前观看端的状态，磁盘上的真实标签重启后作为保留标签恢复；它不等于不可关闭的置顶。提交页从节点获取真实作者/正文，未伪造持久化摘要。

## 已完成验证

证据统一位于 `/Volumes/disk0/Khor/evidence/round3`，失败及夹具修正记录保留，未把失败回写成通过。

| 检查 | 证据 | 结果 |
| --- | --- | --- |
| GUI单元/设计门 | combination-unit.log | 实际exit0，含终端手势、恢复纯状态投影和原有单元门 |
| TypeScript | combination-types.log | 实际exit0 |
| 原恢复/长历史与进入续接 | combination-chat/report.json | Chrome18/18，包含原13组历史；源hash前后一致 |
| 工作区/预览/草稿/窄屏 | combination-workspace/report.json | Chrome5/5；真实原组件与CodeMirror，初始viewport meta已校准，源hash前后一致 |
| 真实xterm DOM事件 | combination-terminal/report.json | Chrome10/10，观察实际鼠标报告字节与打开意图，无PTY、无外部app执行 |
| 节点标签落盘 | R3-03/tabs-green.log | Rust `--lib`30/30，提交renderer先红后绿；请求重试、置顶、关闭等原规则一起过 |
| 模型/边界 | R3-03/model-final.log、history.log | 实际exit0，快速连点/跨session/重试不重复建/正确标题/返回清preview/重启/分页保留 |
| Shell运行资格 | R3-02/shell-final.log、ownership-final.log | 实际exit0，仍核进程出生、宿主/子进程归属及已记录退出；未放宽新开门 |

图查询用main完整图定位和确认影响，新增树增量图只作辅助；没有把“0受影响流程”当成无风险证明。这里是作者自审，不是独立智能体审查。界面截图已查看，分支文字和图标、历史/X中心线、两枚36×30切换按钮也有实测。

## 还需真实体验验收

- Mac真实Codex TUI：单击URL/文件在Khor内打开，Cmd单击走默认应用，折行文件、拖选和程序鼠标正常。
- 正常/断连/确认结束的真实Shell与GUI：健康不显示恢复词；动作后原药丸统一呼吸，保留必要资格判断，未决操作不重复启动。
- 独立工作区及session原导航：宽窄、新建/搜索、长分支、独立提交标签、双击保留、滚动/收起状态、草稿不丢。
- iPhone原生：原甲板、提交页返回、触摸选字、手势和实际键盘；Chrome触摸模拟不等于这项通过。

WebKit运行时缺失，未下载，未宣称通过。本轮没有原生发布构建；当前运行app仍是旧main版本。Lab使用受控内存数据与正式前端组件，只用于审查本轮界面，不能把它当成用户真实节点数据或新原生版本。

## 执行及资源交接

两个命令选择失误已处理并保留日志：第一次Rust定向命令漏`--lib`，额外构建集成目标后停止自己的Cargo；组合`npm test`的pretest隐式触发绑定编译后停止，改显式unit/types，删除本次误建的2.7GiB工作树target。共享Rust缓存保留，不清未知产物。最后内盘约16GiB、外盘91GiB；不以这个快照替代以后大构建前检查。

中央权威入口为规划树 `docs/code-quality/ROUND-3-CHECKPOINT.json`、`STATE.md` 和 `ROUND-3-PLAN.md`；镜像及仅本轮提交bundle位于内盘recovery目录。恢复先核实际分支/diff/进程；不重做已提交四项，不唤醒子智能体，不从旧inflight压缩包覆盖代码。Lab进程/URL在检查点登记，合并和ship等待用户另行授权。

## 2026-11-01 首轮验收修正

见 [修正记录](tasks/R3-review-corrections.md)。新建页首次内容原位转换；文件端口不再被浮动页padding覆盖；浏览器移除工作区返回；Lab补齐正式app全局滚动监听。最新review-fixes/model、types-final、unit均0，regression-final2五组通过，light-final专门验证首行/新建页/浏览器返回与连续滚动材质。没有Rust或原生构建，main与用户app不动。
