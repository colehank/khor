# 移动地址通知：准确锁定版本的静态核对

本次只读核对R9081b94d（相关App/journal/iroh源与I117c39相同），没有新增移动实现或物理验收。完整N4仍未完成。

锁定的上游是netwatch **0.19.3**。从Turing实际Cargo缓存读取原crate archive，SHA256 **39da9cad5f23aa43401f09497d3b280e51b5feba115d9ffbf38ca35d6ff95e96**与Cargo.lock相同，内存读取源码；没有依赖安装、执行代码或改变被测source。准确文件/hash在E/upstream-netwatch-0193-mobile/source-files.json；分析E/mobile-address-source-audit-v1.json。

静态链路如下：iOS在上游build.rs明确走BSD AF_ROUTE监听，InterfaceAddr事件会触发状态重读；监视器将可读队列排空，出错有受限重建退避。actor读取新State并与旧状态比较，State含local_addresses。Interface自身eq忽略addrs，不能孤立据此认定地址被忽略；major判定另比较接口/flags和前缀，同前缀变化可能是minor。iroh对minor也会重新获取外部地址，并通知QUIC。Node另从watch_direct_addrs观测实际接口集合并通过Interfaces合并后触发Addrs。

App的watch_the_path确实只按可达状态和Wi-Fi/蜂窝/有线种类去重，同种类下地址回调会在这一入口被忽略。不过上游AF_ROUTE是独立通知来源，故此源码观察**尚不证明漏事件或手机现场的根因**。也不因上游有监听就认定khor自定义major-change API或App hook可直接移除。

下一步实际验证必须覆盖：同prefix IPv6隐私IID替换而IPv4保持、IPv6前缀变化、IPv4/NAT映射独立变化、双族/双方及peer relay自身换址；检查发布/Relay登记/新旧探测epoch和端到端读/PTY恢复，同时记录原期限与资源。手机仍是旧安装版，新的候选真机与锁屏/后台/能耗尚未执行。另需为持续无关路由事件设计有界通知测试：上游actor的250ms debounce每个事件都重置是静态风险，不能当成已复现bug。

准确源码与最终集成后的测试决定是否增加或删除custom；本次没有把未验证假设转换成新补丁。
