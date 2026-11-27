# 高级用法

## 生效方式

所有应用统一走同一执行流（无需选择模式）：

```
① 覆写 Build 字段 → ② 修改系统属性 → ③ DPI 伪装 → ④ 卸载模块
```

- **默认（推荐）**：伪装只在**应用自己的进程内**生效——应用里读到的都是伪装值，其它应用、系统服务和 `getprop` 读到的仍是真值；模块改完立即卸载，不常驻、不残留。
- **`companion_resetprop = true`**：伪装值**对全系统生效**（`getprop`、其它应用也能读到）。应用切到后台或退出后自动恢复真值，回到前台重新应用。

需要让 `getprop` 子进程或其它应用也看到伪装值时，才需要开启 `companion_resetprop`。

## 调试与日志

```toml
debug = true
```

启用后输出 Info 级别日志（关闭时仅 Error），写入 `/data/adb/device_faker/logs/device_faker.log`。正常使用建议保持关闭，以免留下不必要的运行痕迹。

## 完整配置示例

```toml
# ── 全局设置 ──────────────────────────────────────────────
debug = false                        # 调试日志（默认关闭）
default_force_denylist_unmount = false

# ── 机型模板 ──────────────────────────────────────────────
[templates.redmagic_9_pro]
packages = [
    "com.mobilelegends.mi",
  # 仅对 userId=999 生效
  # "com.mobilelegends.mi@999",
    "com.supercell.brawlstars",
]
manufacturer = "Nubia"
brand = "REDMAGIC"
model = "NX809J"
device = "REDMAGIC 11 PRO"
product = "NX809J"
fingerprint = "REDMAGIC/NX809J-UN/NX809J:16/BP2A.250605.031.A3/20251017.000000:user/release-keys"
build_id = "BP2A.250605.031.A3"

# ── 直接配置 ──────────────────────────────────────────────
[[apps]]
package = "com.omarea.vtools"
manufacturer = "Xiaomi"
brand = "Xiaomi"
model = "2509FPN0BC"
device = "Xiaomi 15 Pro"
product = "popsicle"
name = "popsicle"
android_version = "15"
sdk_int = 35
dpi = 420                       # 临时屏幕密度（120–640）
force_denylist_unmount = true  # 覆盖全局默认，仅对该应用启用

[[apps]]
package = "com.example.detected.app"
companion_resetprop = true     # 全系统属性一致（应用退出后自动恢复）
manufacturer = "Custom"

[apps.custom_props]
"ro.custom.property" = "custom_value"
"ro.debug.mode" = "__DELETE__"
```
