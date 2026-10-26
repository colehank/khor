# Q052 root独立源码审

2026-10-27，root非实现作者。先graph minimal：图686f旧/HEAD不符，继而审实际全部生产变化、相关测试与真实调用方，不以0flow为覆盖。候选未编译/运行。

host要求精确current greeting/首Version5，不min或降级；版本验证前不resize/no replay/no独立startup。绝对整帧deadline涵盖prefix/body/drip；认证后Version往返使用15s合理，健康client立即发送。旧Hello本无read deadline，新共享reader也使其有界。GUI connect_welcomed保cookie与独立greeting/成功后恢复无timeout。model缺失的真实ring用途、tmux modes/redraw、snapshot、history/Forget、renderer Ready仅释放一次都保留。

Term版本状态及冗余versioned包装已删除，重连仍经严格connect并补history；commands真正raw-mode/双byte pump就绪后发Ready。Forget保既有bool API，成功true/写失败Err，不动TS/UI。全仓Rust versioned/hostAttached残留核零，GUI自己的Attached仍保；rawHello caller均为GUI或故意错误cookie对照，没有找到漏改正常terminal客户端。

核新增TCP5父（greeting/cookie/currentVersionfirst/silent/wholeframe drips）、CLI2真实binary+PTY父（12坏首op子场景仅1父，EOF/reset而非任意超时为绿，坏客户端不改尺寸或注入input）、core2真实TCP+Node registry+core API父（拒绝后无attachment、History/rendererReady/Forget/Resize/同socket继续）。删除旧ignored旧binary探针不算新绿；term_io只删字段且3 oracle不变；snapshot/tmux/phone/native/CLIopen机械适配，phone startupOnlyOnce保留。corefixture明确不将本测试PID marker传给Node.close。

Root实际核双盘28file、当前1921个input逐SHA/size及完整patch SHA一致（source-inputs为对象值，按sha256字段核），12变化路径冻结。**源码审无确认新增阻断，可进入定向验证。** 尚需正常Node/core/commands/CLI编译，9新父、termIO3、snapshot/history/replay/phoneReady、共享GUI greeting4及适用native/tmux回归。App/物理设备边界另记，源码审不代替运行。

## candidate02 最小测试隔离补充

Root实际审01→02单源delta：host_shell_tmux helper仅增加`-f /dev/null`，两个真实启动typed命令本来已带该参数；私有-L及原两父/全部oracle不变，不覆写HOME。全1921当前inputs逐SHA核齐，30候选双盘一致；完整patch74745a2101a5e520305608ef01134b1c94cb1c23a673e8e2a395217b653f2e96。无新增确认阻断，仍未编译/运行。plan01保历史，资源分批plan02尚在准备不可执行；新candidate02/root审证据位于Q052/terminal-candidate02-root-review。

Root另实际审资源plan03：6双盘files、runner913ed80d63c7c12e0089af474eb6d356ebb7ecabb83c519df751e0f547a94ee6，11批/23唯一父/AST3009005GiB齐；只执行隔离纯dep-info parser小例，无Cargo。修正02错误ROOT子串门，按实际Cargo当前manifest/src_path+fresh=false与Make RHS相对cwd解析/源SHA证明；本run已完exe精确退休及原HOME保留。无新增控制阻断，尚未授槽。root证据Q052/terminal-plan03-root-review。
