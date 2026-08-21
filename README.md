# Khor

> 坛城（Kyil-khor），意为连接。

产品与设计见 [docs/KHOR.md](docs/KHOR.md)。

## 上手

**先建一次前端**：`cd apps/gui && npm ci && npm run build`。网页脸
（`khor web`）把 `apps/gui/dist` 编进二进制，所以在它存在之前，
`cargo build` / `cargo test` 会红——错误信息里就写着这条命令。
产物 gitignore，建一次留在盘上；改了前端要重建才进得了二进制。

`cargo test` 跑全部测试。`target/` 是指向外挂盘的符号链接，别 `cargo clean`。

桌面 app 在 `apps/gui`：开发窗口 = `npm run dev`（apps/gui 下）加
`cargo run -p khor-gui`；真连排版验收 = `node scripts/smoke.mjs`，
网页脸那一份 = `node scripts/web-face.mjs`。
