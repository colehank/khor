# Mobile 会话候选连续跟手（R2-025）

更新：2026-10-29。状态：**独立 Lab 已完成，待用户手机验收；正式产品未接入。** 用户先要求检查、讨论后 Lab，确认“对 做吧”。本次授权覆盖该 Lab；不扩展到已登记的其他专项、main、App重启或ship。

## 已确认的交互

长按唤醒后，候选先随手指连续移动，越过阈值再顺势吸附；保持偏移按原慢节拍自动循环；收回中立区暂停并保留已预选项；明确反向拨动倒退。阈值前回中回原位，松手才真正打开预选会话。方向必须和手指一致。

覆盖 GUI 右下头像扇形、点击头像展开的药丸、终端底部药丸。保留全部标题、近大远小、五个可见候选、循环、斜向行程以及推远展开全部列表。普通底部左右拖动保留既有行为。

## 根因与修改边界

生产 deck-joystick 只给整数 preview；超过约10px就step，绘制没有连续位置。其index方向与几何offset方向相反，因此底部手向右候选向左，右扇形手向上候选向下。普通 deck-scrub 的 `base - dx/width` 已正确，应保留。

Lab把连续position与已接受target/preview拆开；手动跟随、自动吸附、循环用同一位置。右侧adapter将斜向行程投影到共享引擎；UI持有中直绘position，结束收拢用target。吸附期间反拨从当前可见位置衔接并限位。先暂停再反拨，避免单次跨中立区事件误连跳。

初版参数：hold280ms，slop4px，阈值32px，64px/格，吸附160ms；循环1400/1000/700ms，吸附后计。参数属于当前Lab候选，不冒用户已认可手感。

## 可恢复入口

- 工作副本：`/Volumes/disk0/Khor/worktrees/current/khor-workspace-layout-lab`
- 分支：`lab/workspace-layout-20261028`
- 本次参考：`97101142a7e1a4720637ea1f4e0af42d1b466294`
- 入口：<http://localhost:23832/lab/session-follow.html>
- 源文档：`apps/gui/lab/session-follow.md`
- 证据：`/Volumes/disk0/Khor/evidence/labs/session-follow-20261029`

现有Vite23832继续运行，沿用workspace-layout.vite.ts；无需native构建。新增12个Lab局部文件，无src产品改动。工作区已验参考c5010294、材质已验参考4d541e5e分别保留，不能用新HEAD替代其验收身份。

本地组件副本仅为隔离Lab。正式接入须更新原共享引擎与组件并删除副本，不能保留多套规则。示例只有React本地状态与假终端文字，不运行shell，不写真实session。

## 验证及未完成项

- 根因回归：`engine-check.mjs --legacy`预期失败，小幅拨动已变更身份。新引擎18项行为测试全过，TypeScript通过，diff检查通过。
- 非作者审查未见阻断；额外内存时钟检查右侧跟手方向、回中刹车、overview暂停/回撤、回头像取消和定时清理。
- CUA390×844：GUI/展开药丸/终端可用，普通左拖sample2→3、右拖3→2，十二项完整列表可选sample10，切换计数0→1→2→3。无横向溢出，捕获浏览器error/warn为空。
- 尚未验证真实手机长按连续拖动、触觉与原生App。CUA本次只能普通drag，不能将纯状态测试写成真机手感通过。当前不声称用户已验收此Lab。

下一步由用户在Lab比较三入口：轻拨是否跟手、松手唯一切换、保持偏移是否可控循环、回中与反拨是否自然，1/2/5/12项及推远列表是否可用。反馈限此Lab；其他新问题继续登记下一轮。

## 协作与压缩恢复

root负责入口/适配/验收与总账；session_follow_engine仅写engine与行为检查；sticky_glass_review只读独审。作者都已交付，无未完成源码所有权。

恢复先读STATE/PLAN/本文，再核对实际branch/diff和恢复收据。产品未接入、main仍e7c335c8；不能按此前发布授权自动合并。待用户确认后可将本次Lab指定为正式接入参考。
