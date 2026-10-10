# 网络实验室

连接的 bug 先写成场景，在这里复现，再修；不靠负责人举着手机试。2026-10-09 定（车道 net-lab）。

## 是什么

- **patchbay**（n0 的网络实验室，MIT OR Apache-2.0，`vendor/patchbay`）：在 Linux 网络命名空间里搭
  路由器、NAT、防火墙、延迟丢包，全程不要 root（先进一个 user namespace）。
- **场景**在 `crates/cli/tests/netlab.rs` 与 `crates/cli/tests/netlab/`：每台机器是一个真的
  `khor serve`，跑在自己的设备命名空间里，家目录、身份、tmux socket 全是临时的，不开 mDNS；
  中继是 `khor relay --dev`（北京那种：HTTP、不答 QUIC 地址发现），需要时另起 n0 那种答 QAD 的替身
  （`lab::qad_relays`，进程内的 iroh-relay，自签证书按 IP）。
- **读法**：每个场景断言 khor「应当」怎样，今天做不到的就是红；量到的数字打成
  `[netlab-result] 场景 | green/red | 数字` 一行。
- **iroh-doctor**（n0，MIT OR Apache-2.0）不在测试里，是真机器上的诊断工具，用法见
  `audit/iroh-doctor.md`。

## 怎么跑

一条命令，在 Mac 上：

```sh
scripts/netlab.sh                 # 全部场景，在 turing 上
scripts/netlab.sh a1 d            # 只跑名字含 a1 或 d 的
scripts/netlab.sh --on <主机> …   # 换一台 Linux
```

它把工作树 rsync 到那台机器的本地盘 `/tmp/khor-netlab-src`，target 在 `/tmp/khor-netlab-target/khor`
（不碰 NFS 家目录，不碰机群构建用的 `/tmp/khor-src-*`、`/tmp/khor-target-*`，不装机），
编译、并行跑（8 个一起，每个场景是自己的一套命名空间），最后打一张表；全文日志在那台机器的
`/tmp/khor-netlab-run/<时间>.log`。在 Linux 上直接跑同一个脚本也行。

机器要求：Linux、允许非特权 user namespace、有 `tc`。没有 `nft`、`conntrack` 时脚本用
`apt-get download` 把发行版的包解到 `~/khor-netlab/tools`（不要 root）。

## 两处为实验室开的口子

- **`KHOR_NETLAB`**（`khor_net::endpoint::LAB_ENV`）：patchbay 的「公网」是 198.18.0.0/16，
  而 khor 把 198.18.0.0/15 当代理 TUN 的假地址丢掉（`endpoint::unroutable`），不开这个口子实验室里
  永远打不通直连。设了它，这一段算可路由，日志里「默认路由经过代理的 TUN」那行也不写。只给临时节点，
  khor 自己从不设；场景给每个常驻进程都设上。
- **vendor/patchbay 的两处补丁**：turing 是 Linux 5.4、nft 0.9.3，原样的 patchbay 建不起带 NAT 的路由器。
  见 `vendor/patchbay/KHOR.md`。代价：5.4 上 conntrack 超时用内核默认，测映射超时的场景要记住。

## 加一个场景

1. 选文件：NAT 与打洞进 `netlab/nat.rs`，中继进 `netlab/relay.rs`，换网进 `netlab/uplink.rs`，
   转发进 `netlab/hop.rs`；新的一类就新开一个文件，在 `netlab.rs` 里 `#[path] mod`。
2. 搭网：`Lab::new()`；中继 `lab::relay(&lab, 场景, 名字, 延迟)`；路由器 `nat::router(&lab, 名字, Net::…)`
   （`Public`、`Home`、`Open`、`Strict`、`Campus`、`Cellular`，各自对应 net-d §1.1 量到的哪种网，
   写在 `Net` 上）；设备 `lab.add_device(名字).uplink(路由器.id())`。
3. 起机器：`lab::node(场景, 设备, 名字, &[&中继], &[环境变量])`，配对 `lab::mesh(&[..])`。
4. 读：`a.until(&b, 时限, |link| …)` 轮询 a 对 b 的路（打印每次变化）；`a.link(&b)`；
   `a.journal(Some(&b))` / `a.journal(None)` 读连接记录（JSONL，`e` 是种类）；`a.freeze()` / `thaw()`。
   改网：`slow(&设备, 往返)`、`iface.replug(别的路由器)`、`router.set_nat(…)`、在设备里跑 `nft`（见 `relay.rs` 的 `wedge`）。
5. 断言写「应当」，红就红着；先用 `lab::result(场景, 是否达标, 数字)` 打出今天量到的，再 `assert!`。
6. **规矩**：场景名用前缀区分（临时目录按 `场景-机器名-pid` 命名，同一进程里重名会互相踩）；
   只停自己起的进程，按 pid；不要连外网。

## 门

现在不是门：一轮要七八分钟、要一台 Linux，红的场景是待修的事，不是回归。
等 a–f 全绿、跑得稳，再考虑写进 `docs/design-system/guidelines/50-gates.md`。
