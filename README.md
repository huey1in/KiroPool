# Kiro Pool

[![GitHub](https://img.shields.io/badge/GitHub-huey1in%2FKiroPool-181717?logo=github)](https://github.com/huey1in/KiroPool)
[![GitHub Release](https://img.shields.io/github/v/release/huey1in/KiroPool?display_name=tag&sort=semver)](https://github.com/huey1in/KiroPool/releases)
[![License](https://img.shields.io/github/license/huey1in/KiroPool)](LICENSE)

Kiro Pool 是一个基于 Vue 3 和 Tauri 的本地 Kiro 账号管理客户端。

## 功能

- 支持 Builder ID、Enterprise、GitHub 和 Google 凭证
- 一个账号永久绑定一个 Windows `MachineGuid`
- 切换时原子更新账号凭证与机器码，失败自动回滚
- 支持查看账号额度和切换历史

## 启动

```powershell
pnpm install
pnpm run tauri dev
```

## 许可与署名

本项目基于 [Cloxl/CursorPool_Client](https://github.com/Cloxl/CursorPool_Client) 修改，原作者为 **Cloxl**、**Sanyela**，并遵循 [MIT License](LICENSE)。修改和分发时必须保留原版权信息。

衍生版本不得使用原项目的品牌元素，包括原项目名称及其变体、原版软件图标，以及在安装包中显示原名称。本项目使用独立的 Kiro Pool 名称与图标。
