# Khor

> 坛城（Kyil-khor），意为连接。

产品与设计见 [docs/KHOR.md](docs/KHOR.md)。

## 安装

从 [Releases](https://github.com/colehank/khor/releases) 选择对应系统的安装包：

- **macOS**：Apple 芯片选 `khor-macos-arm64.pkg`，Intel 选 `khor-macos-x86_64.pkg`。双击安装，之后从“应用程序”打开 Khor；终端使用 `khor`。
- **Windows**：运行 `khor-windows-x86_64.msi`，安装 App 和 CLI。安装后重新打开终端，使新增的 PATH 生效。
- **Linux 桌面**：Ubuntu 24.04 及兼容系统使用对应架构的 `.deb`，通过 `sudo apt install ./安装包文件名.deb` 安装 App、CLI 和系统依赖。
- **服务器或只需要 CLI**：使用发布页的 `install.sh`，独立 CLI 用 `khor upgrade` 更新。

`khor gui` 打开已安装的桌面 App；`khor web` 打开网页界面。随 App 安装的 CLI 与 App 一起更新，已有的独立 CLI 会被保留。更多安装、迁移和更新说明见 [发布指南](docs/RELEASE.md)。

## 开发

可以选择纯前端 Lab 或完整原生 Lab，也可以把编译和检查交给构建机。本机仅保留源码时，请使用远端构建模式。见 [Lab 开发与验收](docs/development/LAB.md)、[远端开发](docs/REMOTE-DEVELOPMENT.md)及 [工程约定](docs/code-quality/ENGINEERING.md)。
