# N1 source09 root源码独审

2026-10-27，root非作者。范围为usage-pending-candidate09相对08的三源delta；并读实际transport select、Work.done、take_push_state/take_pull_state和receipt flush上下文。图缺该新私有模块时以实际源码为准，未将0flow当覆盖。

已核：23文件外内字节一致；delta SHA881ee330280b793e4052efaa202ba99a248a3fd60209c9174fb2f5532d126b6f；当前1084选择域逐SHA等09冻结。此选择域非全部GUI仓库。

- TX在原write_all完成后记录，即使finish/read/解码/后续本地处理失败仍留账；这是本地写入transport的body量，不声称远端接收或导入。未完成但已开始的尝试只扣不确定预算，不计公开moved。
- RX在同一次Response解码后、原cap/validate/roles/wrong-doc等检查前记录可确定body长度，错误不丢账。ordinary answer仍走同一原校验/本地化逻辑，无重复decode、新wire或RPC retry。
- StepOutcome先合并Usage再处理Busy/硬错误，公平quantum同时按实际/不确定body与块数扣量；opening预留2块，后续Write/Read预留1块，保32MiB/30s/4块边界。
- opening先保两方向manifest，receipt/本地receive失败时started仍false；仅处理完后提交started，并显式保留任一Missing重Exchange信号，避免纯pull被误报done。
- receipt控制路径接同一Usage，正常零body不制造进度；Loaded/Sink不跨新增network await，普通exchange仍复用原liveness/retirement实现。

root未确认源码阻断，等待另一位非作者独审及实际验证。source09未编译/运行，不能将上述静态结论写成功能绿。最低回归为原unit11、policy1、ordinary proto2、quiet/Busy/purepull3、原N0正常Node接入；真实32MiB网络边界、崩溃恢复与N2另待，不冒纯策略测试为网络验证。

## 第二位非作者独审

node_test_review已独立收口，无确认源码阻断，可进入验证。已核23双盘/current14及选择域1084；ordinary proto逆去wrapper/observer逐字等08，普通transport/liveness/retire逆去观察分支亦逐字等08；Missing与Busy后的started未被晚赋值覆盖。

两位审查者均保留原有边界：30s是下一step/receipt准入门，in-flight仍沿原RPC预算与RTT，不能称整次visit硬deadline；非法意外body虽记实际量，合法opening2/后续1块预留不声称硬限任意违规peer raw流量。这些未由09新增，不阻断本片修复，也不冒静态审查为网络预算实际验证。

## 实际A与fixture10收口

A `node-transfer-n1-green09-control06` 正常libtest67.796s构建通过，policy1+原unit11+proto2+quiet共15项实际通过。Busy用例在flow_tests:896全visit无条件交替断言失败，purepull未跑；最终计量断言在其后，不能称accounting绿。原238份双盘、源码/产物稳定与17自有组/TMP清理收据保留。

root与作者、另一审查者均从真实raw核前10个D/A quantum交替，Agents最后合法read末片覆盖其2140316B固定manifest并成功AfterRead，随后只剩Devices quantum。原公平断言没有排除已完成文档。

fixture10仅改doc_transfer_flow_tests.rs，其余1083含生产逐字等09。复用真实After(agents,Read)暂停，现场读取双方真实epoch/target VV均已包含对应固定manifest后才记同一Point Entry的order；此前仍严格交替，无完成证明则全visit严格，此后只准剩余Devices继续。root核23dual/current1084/sole delta，node_test_review独立逆核其他四heavy body及wire/SeenPin/反向整块/计量/quota断言未减，两审无确认阻断。

现已授权resume08（runner76329f5dc527adfb1182ab1163270445e272dc06d94b243f0e61168f526b4873）只一次必要编译+Busy/purepull2，继承旧15；尚未取得该两项实际结果，原N0另待。原失败不回填。

## source11：收费与逻辑进展分离，实际17项全部通过

resume08实际Busy通过42次、actual=charged15,815,178B；purepull却在首visit未完成的控制断言失败。实际raw显示gate释放后同visit再次Exchange并完整Read/Confirm，未到最终Pending断言。root重新检查发现09新增 `advanced |= usage.charged()>0` 把失败流量当业务进展，使只剩Busy的文档继续重试直到visit准入预算耗尽。

source11只删该一行，保先累计Usage/quantum、Ok才推进、全Busy无逻辑进展则Pending退出，健康sibling仍可继续。node_test_review独立同意，无正确性反例；root核23dual/current1084及唯一delta ae15907895706db4f504cd43b6474734c92beab52654aefc345f25c243c596fb。fixture10因果公平检查及原purepull gate/body保持，不再调整测试迁就空转。

`node-transfer-n1-green11-control09` 重新编译82.035s，完整17 distinct全部各1/0/0，不继承旧15；root独读全部17raw与final，确认source/binary/deps稳定、18自有组/TMP清、无guard/drain。Busy恢复6次且actual=charged6,377,994B，SeenPin及实际完成proof通过；purepull first_pending=true/first_target_absent=true/reopened_target=true，原sameManifest/VV/Spec均过。作者250dual已闭合。

已授B `node-transfer-n1-n0-green11-control10`（runner fd0470ed64b7aab1080fd92ea1b3a47c7fdb3cea4f9bcec7a80168b43a0a78f1）仅原N0公开Node大历史目标；当前执行中，尚未有收敛结果。N2/真实中途SIGKILL/优化构建32MiB网络边界仍待。

## N0实际通过与集成

原B11实际成功：正常编译46.24s，worker111.61s，总158.955s；原outer+ignored worker各1/0/0，只计1业务父。raw_gap25,447,950B/epoch1，四visit依次传6,816,274、8,912,896、8,126,464、2,379,278B；第一起Seen123/Pintrue，第四固定VV覆盖与内容匹配，cap_rejections0。root逐关键raw/final并核49dual，源/normalNode及依赖/exe/组/TMP稳定，未放宽4visit。

作者d7a4192d与原dd60回归已695fa455/c13a5c44集成。N1非共同9路径、R1非共同23路径及Q05212路径完整SHA保留，catalog键值为两方并集；三共同源lib/link/proto需组合native/App验证，不以分支作者测试替代。后续crash14与实际32MiB/N2仍待。
