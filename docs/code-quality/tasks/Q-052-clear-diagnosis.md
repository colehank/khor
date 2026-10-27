# Q05207 clear / replay 只读归因与最小修订建议

Owner：node_test_review（Q052作者，本文是诊断，不是独立审查）。2026-10-27。未Cargo/native/改生产/测试；执行槽属于N1。原05/06/07失败日志和candidate02保持，本文不改变已冻1921输入或运行统计。

## 固定来源

树`/Users/zgh/Developer/Khor/worktrees/recovery/khor-quality-store-owner-recovery`，fix/quality-terminal-current@13a9459f6c2dfd8d12912e6e2325b58de6e06999 + candidate02。完整当前1921 source逐SHA等于terminal-green07/source-before.json。无.codegraph，按规则跳过CodeGraph；先CRG callers_of Feed::forget，命中的是main@8704a503，故只作定位，以下均核本树实际源，不把main覆盖当候选事实。

- term.rs SHA7cc4920a931f33e9292238b9bc102ca1c4e6379ce34e16915f89cd86ae352b0d
- host.rs SHA2536d1288706e296c39b4ef018d74f90c2464764202a2e951860b1a56c766096
- terminal_current.rs SHA0dfe807e85b04904de25d36eacec3e4644c8293b0eb9433ccfeafd3f9d71f4d7
- TerminalPane.tsx SHAd618961d3ab4f2e59a517dd9981aef4b8a3937b19f949d2ac32176a563f8876d

## 已确认产品因果

1. `TerminalPane.tsx:1850–1872`的clearBuffer明确承诺清scrollback及cursor上方，保prompt，并通过term.clear()→termForget(id, atPrompt)清本地/host；只有普通屏幕调用后端，在prompt才附Ctrl-L。`term.rs:1454–1471`依次写Forget、可选Input([0x0c])，然后只Feed::forget。
2. `Feed::forget`(:85–93)删除最后LF之前的bytes并推进head modes，既未改变life也未清本地vt100 parser。其文档:81–84承诺重新开始的face也不收到已清内容。`take_since`:594–601锁parser→feed，计算start=end-len；`since<start && since>=life`重放tail modes+parser旧screen。初始life0，合法旧游标0必然走这里。新TCP父:109–125确实通过公共term_stream(id,0)取原生结果，:201真实见before-clear；不是模拟计数。随后服务端:189 EOF由主断言panic/fixture退出诱发，不能另当目标失败。
3. 实际产品通常用已跟上的seq，所以clear后即时不一定reset；但termWait/termStream允许落后游标，`TerminalPane.tsx:2065–2083`在reset时term.reset()+replay后还发送Ready。隐藏pane、后台积压或另一个face落后时会实际显示被清screen。atPrompt=false不会Ctrl-L，因此不能依赖shell稍后重绘修好本地parser。atPrompt=true也只有异步重绘，没有本地同步保证。
4. 这是原有clear契约漏洞被新current-protocol集成父暴露；协议清理没有改变Feed::forget或take_since这条逻辑。仅让测试用最新cursor会避开真实缺口，不能算修复。

## 后续trim仍可复活，不能只移life

设clear后life切到retained start；旧cursor落在切点之前可走Feed::replay，第一步不再见旧screen。但parser仍保存旧normal screen。随后一个clear后的合法cursor被大于FEED_CAP的无LF/覆盖局部输出甩在head之前，仍满足same-life snapshot条件，snapshot会再次含未覆盖的旧行。因此必须同时清本地parser。为使该回归可观察，应使用"重复的有界完整OSC title（例如每条≤1KiB，以BEL结束）+ after标记"等增大原始字节、但不滚走整屏旧内容的合法真实输入；不要只用大量newline把旧行自然冲掉后假绿。FEED_CAP现为2*RING，边界取实际常量，完整合法OSC结束后再取结果。

## 真实host不是同一遗漏

`host.rs:2235–2242`在ring锁内同时ring.forget和Model::forget。ring:842–854裁raw历史且维护gone/pictures；Model:1167–1192在normal屏重建fresh parser，保cursor行及下方、颜色属性/输入模式/隐藏光标，cursor移到第0行原列，原scrollback不复制。后续attach:2181–2184有model则snapshot它，无model才ring；后续Model::keep/settle也从已经清理的snapshot重建，不另藏旧parser副本。alternate分支不清与GUI仅normal清相符。未发现同类host旧screen复活源码路径。

已有host单测the_model_forgets_what_was_cleared(:2993–3009)真实喂parser，验prompt/colour/bracketed paste/scrollback0及snapshot无旧line。06真实CLI/PTY父a_current_client_clears_the_host_history_for_later_attachers也已通过；这些不能证明GUI本地parser正确。本文没有重新执行它们。

## 最小完整改法（供root独审/授权后实施）

保持wire/TS/seq协议不变，不增产品开关或新事务框架。

- 将本地clear收为一个小helper：与paint/take_since同样持有parser→feed锁，Feed裁历史和本地parser清屏必须同一临界区，避免reader在两者之间把同一批数据分别画入不同代screen/feed。锁不跨socket IO；仍先完成Forget/可选Ctrl-L写入。保持ACS filter及head/tail mode tracking，不把clear做成rejoin/start_over。
- 为clear前已丢弃cursor建立replay边界，可将life语义扩为"当前可snapshot段的起点"并至少推进到当前retained start（不退回旧life）；因此落在clear前已删除部分的cursor走feed.replay，不走旧snapshot。fed/end不跳号：clear时已经caught-up的end仍返回空/不reset，后续新增bytes仍从相同end连续。无需重用restart_feed的fetch_add。
- 本地parser也应按"normal screen清到cursor行，保该行/下方/列/属性/输入模式/visibility"重建。最窄可靠复用是把host现有Model::forget的parser重建小段提成host.rs中的明确normal-screen helper（类似已有pub parser_like/resize_screen），core调用；Model的Beside offset/forget仍留Model，原host效果保持。这样不用另造一套逐cell formatter。若选择core私有实现，也必须满足这些实际屏幕不变量，不能只新建空parser或只重喂任意截断的rawtail来冒保prompt；Feed按LF裁剪不是完整vt100 cursor-row解释器。
- 不单独改现失败父的cursor；保其原Version/Scrollback/renderer Ready/Forget→Ctrl-L→Resize/同socket后续Input oracle。

## 最小适用验证（尚未运行）

1. 扩展现a_forgotten_feed…或相邻一父，使用真正paint/共同clear helper。输入旧history +当前带色prompt和typed、bracketed/application/keypad/cursor模式。clear后：take_all/旧cursor replay无secret且prompt及模式正确；fed=end不变；take_since(end)空且不reset；再paint新bytes，caught-up只拿新bytes。用fresh vt100解析回放，验屏幕/列/模式，非仅搜字符串。
2. 同父或一独立父覆盖"clear后的cursor再被trim甩掉"：留下至少一行旧screen且不用滚屏自然抹掉，clear后记录新cursor，再通过paint送超过FEED_CAP的完整合法无可见滚屏输入及新mark。断言reset snapshot不含旧secret、仍含所保prompt/新mark，游标/模式符合实际新screen。再做第二次clear和再append，排除只第一次清有效。不要只测私有life字段或validator形状。
3. 原terminal_current TCP两父按同源actualfresh=false executable重跑；保第201行和同socket最后Input/echo及renderer Ready门。只让父通过不足，还需(1)(2)堵后续trim。此TCP peer验证GUI-core实际IO，不冒真实host/PTY或App验收。
4. 若抽host parser helper，重跑原the_model_forgets_what_was_cleared、现host snapshot/history两父和原真实CLI host-clear父；仅当前受影响范围，不重跑已绿无关拒绝矩阵/构建前提。构建与native仍需root另授唯一槽。真实App clear/后台回来属于最终GUI验收，本文不宣称已验。

本文仅追加只读设计卡，未改source1921、测试、冻结计划或05/06/07历史，不启动任何进程/编译。写权交root，可按此最小行为范围另派修订；Q052仍open。
