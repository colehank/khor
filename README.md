# Khor

> 坛城（Kyil-khor），意为连接。

产品与设计见 [docs/KHOR.md](docs/KHOR.md)。

## 上手

`cargo test` 跑全部测试。`target/` 是指向外挂盘的符号链接，别 `cargo clean`。

桌面 app 在 `apps/gui`：开发窗口 = `npm run dev`（apps/gui 下）加
`cargo run -p khor-gui`；真连排版验收 = `node scripts/smoke.mjs`。
