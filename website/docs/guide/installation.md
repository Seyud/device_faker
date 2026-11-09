# 安装

## 环境要求

::: danger 不支持 Magisk 自带的 Zygisk
Device Faker **不支持 Magisk 自带的 Zygisk**，需要第三方 Zygisk 实现（推荐 **ZygiskNext**）。

请先安装并重启，再安装本模块。否则安装会直接中止
:::

| 条件 | 要求 |
|------|------|
| Zygisk 实现 | 除 Magisk 自带 Zygisk 外均可 |
| 架构 | arm64 |

## 下载

从 [Releases](https://github.com/Seyud/device_faker/releases/latest) 页面下载最新的
`device_faker-(release).zip`。

## 安装步骤

1. 确认已安装 Zygisk 实现（推荐 **ZygiskNext**）并已重启
2. 打开 Magisk / KernelSU / APatch 的模块页面
3. 选择「从本地安装」，选中下载好的 zip
4. 安装完成后**重启设备**

## 更新

直接覆盖安装新版本 zip 即可，默认保留现有配置文件。

## 卸载

在 Magisk / KernelSU 的模块页面移除模块并重启即可。

::: warning 卸载会删除配置文件
模块自带了卸载脚本，卸载时会执行 `rm -rf /data/adb/device_faker`，
**配置目录会被一并删除**。需要保留配置的话，请先手动备份
`/data/adb/device_faker/config/config.toml`。
:::

## 下一步

- [基础配置](./configuration/index.md) — 配置文件的两种写法
- [WebUI 管理](./webui.md) — 用图形界面配置，比手改 TOML 更方便
