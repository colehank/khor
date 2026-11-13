# 确认探测与质量来源的一致性

2026-11-12。完整目标保持PLAN N0–N5；本项只处理原peer relay失败中的一处具体不一致，不把它先定为9.3秒停顿的唯一原因。

## 已观察触发

原新栈028的`110840Z-1e90b3dd`里，X在middle重接后+2051.718ms开始Relay→Custom确认。完整DEBUG行2122的质量代表为Conn140064923605712/Path1，95.483590ms、近期样本1；另一Conn140065527340576/Path1当时138.971123ms、近期样本0。紧接2124的确认probe选择后者（PTO0），未刷新正在尝试确认的质量来源。随后另一副本111.157583ms的新样本成为代表，request-open持续到约7秒仍报告这一质量；后续才重启确认并于9.301772s Go。原双向四秒门失败保持。

E/iroh14-confirm-copy-observation-v1.json另绑定行2118/2119（同tick两copy均PTO0）、moving在+2298.518ms仍pending而+2551.014ms已None/restNone的原始行。这证明probe和代表copy不同；并未直接观察原selector取消日志，不能把全部停顿唯一归因。三轮被动诊断未捕获目标方向取消，固定两轮追加已关闭，见STATE与E/iroh14-pending-cancel-two-diagnostics-results-v1.json。

## 最小候选与反例

保留当前路最大PTO优先、候选路最小PTO优先；仅在PTO相等时，沿用现有质量代表的近期样本数/采样时间排序。只在live、可ping的副本集合中选择，关闭的代表不能强行被保留。质量和确认仍来自同一副本，旧ACK不能补新确认；原confirm时长、探测预算、250ms节奏、idle timer、收益门和四秒验收门全部保持。

不新增恢复状态、连接或定时器。该候选收束现有探测和测量的排序，后续继续核查可由上游替代的机制；不是为了补丁数而删除必要保护。

## 固定验证步骤与准确源

1. 从冻结028独立建`khor-net-iroh-confirm-copy-v1` / `experiment/net-iroh-confirm-copy-v1`。先只抽取原priority计算（常数后缀保证排序不变），增加确认探测真正刷新其质量来源的行为回归。已有samples_after和Moving合同保持。
2. 红源 **9f3a6ee6682ad87d94961194c30d60cc5c116fa4** / tree **a2eb303894b8c4464c7f8d1851b5c0ef9fd2fe2f** / snapshot **b27ca9efdfeabaa65f78de8aeb0b0be76922191a**。E/iroh14-confirm-copy-red-source-audit-v1.json文本还原核对原生产行为；本机零Rust执行。
3. 红全vendor门 **turing-20261112T124312Z-e33e746e** 已终态101/338.074690s：Noq467过，iroh原197过/原ignored1，仅新增行为回归红（实际90ms、预期85ms），node lib/tests编译过；三锁/原文件hash/source diff/group空核对。E/run/confirm-copy-library-verification-v1.json绑定命名人口，非编译或格式假红。
4. 红后候选 **80aeeff919aa641dae55ba1a18b2020fead3e05e** / tree **f857f33c60e45cb7fee862c47c510cf5a8a27765** / snapshot **bc35d760bc6d0cf9d47aeee390e1913c175b8a10**；仅equal-PTO tie-break，回归fixture逐字不变。E/iroh14-confirm-copy-source-audit-v1.json证明primary PTO公式、应用/锁/物理输入未改。同control全库门 **125007Z-645fe88c** 已0/233.761110s，Noq467/iroh198全过、原ignored1；node lib/tests编译、三锁/source diff/17原hash/group空核验，红绿回归文件SHA相同。固定三轮原物理peer replug由E/iroh14-confirm-copy-physical-plan-v1.json冻结（原7份输入/metadata SHA及原四秒门），R1 **125445Z-23ca34d4** 已101/197.619032s：首setup约10.271s超时，1error、尚未注入换网，两严格门未执行，原失败/identity/原hash/attempts/group空与ledger保留。当前R2 **125919Z-c38e8c1d** / pid1892500在途；全部保留、三轮后停止该批。R1已经使三轮全过条件不成立，unit绿不晋升。
5. 只有单位合同、原负例和实际物理门证据匹配准确最终源，才能评价本项；不能以绿unit宣告手机/WAN/全N2或整体优化完成，不改canonical/已装App。

机器可读协议 E/iroh14-confirm-copy-plan-v1.json SHA256 `0b12e7438b85f4a6a9c3dcd2f60eb44482be49bc7e66ee377842a8210a5a1df9`。固定普通性能源028保持，原性能不能自动借给修改后的候选。运行状态以STATE为接续入口。

## 固定物理结果：不接纳

三轮全部完成，未替换失败：R1 125445首setup10.271172s失败、未注入换网；R2 125919业务124ok但两严格门红，X/Y Go5.522531/7.762941s；R3 130239业务124ok、两门绿，Go3.375083/3.613151s。各16原files和source/control/精确attempts/原hash/group空核验，proof/ledger全闭。结果E/iroh14-confirm-copy-physical-results-v1.json SHA256 `7a80dc776d5244aa0a3422b2ef167b06f3f53ce94f1a0e5c29ad4a0e9a3c17b5`。

候选80aeeff9不接纳，保留独立分支，不合canonical。单位合同的修正不足以关闭真实恢复；首连问题独立诊断，恢复质量/收益门与连续确认仍需重新核对。没有因为结果失败调低门或增大probe预算。
