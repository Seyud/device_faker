# 字段参考

## 设备信息字段

以下字段可在模板或 `[[apps]]` 中使用。每个字段同时驱动 `android.os.Build` 静态字段（JNI 覆写，仅在应用进程内生效）和对应的系统属性（供 native 层 `__system_property_get` 等读取）：

| 字段 | Build 字段 | 系统属性 | 说明 |
|------|-----------|----------|------|
| `manufacturer` | `Build.MANUFACTURER` | `ro.product.manufacturer` 及分区变体 | 厂商 (如: Xiaomi, Samsung) |
| `brand` | `Build.BRAND` | `ro.product.brand` 及分区变体 | 品牌 (如: Redmi, nubia) |
| `marketname` | —（仅属性） | `ro.product.marketname` + `ro.vendor.oplus.market.name` | 市场名 (如: Nothing Phone (3))；OnePlus/OPPO 设备读取后者 |
| `model` | `Build.MODEL` | `ro.product.model` 及分区变体 | 型号 (如: NX769J, SM-S9280) |
| `name` | —（仅属性） | `ro.product.name` 及分区变体 | 产品内部名 (如: popsicle) |
| `device` | `Build.DEVICE` | `ro.product.device` 及分区变体 | 代号 (如: xuanyuan)；未设置时自动使用 `name` 的值 |
| `product` | `Build.PRODUCT` | —（无系统属性） | 代号 (如: xuanyuan)，仅覆写 Build 字段 |
| `hardware` | `Build.HARDWARE` | `ro.hardware` | 硬件名 (如: qcom)，无分区变体 |
| `board` | `Build.BOARD` | `ro.product.board` | 主板名 (如: kalama)，无分区变体 |
| `fingerprint` | `Build.FINGERPRINT` | `ro.build.fingerprint` | 指纹 |
| `build_id` | `Build.ID` | `ro.build.id`、`ro.system.build.id`、`ro.vendor.build.id`、`ro.product.build.id` | Build ID (如: UKQ1.230917.001) |
| `characteristics` | —（仅属性） | `ro.build.characteristics` | 特性 (如: tablet) |

::: info 什么是「分区变体」？
指 `ro.product.{odm,vendor,system,system_ext,product,bootimage,system_dlkm,vendor_dlkm,odm_dlkm}.<字段>`。bionic 的属性读取按前缀路由到不同分区属性区，OnePlus/OPPO 等设备会读取这些变体，统一写入可保证各设备读取一致。

其中 `system_dlkm` / `vendor_dlkm` / `odm_dlkm` 走**「已存在才补齐」**：这三个动态内核模块分区只在 Android 13+ 的部分 ROM 上导出这些副本，设备上不存在的 key 不会被写入（否则会在 `getprop` 全量输出里留下真机没有的可枚举「幽灵属性」）。`ro.{system_dlkm,vendor_dlkm,odm_dlkm}.build.*` 同理。**营销名（`marketname`）整族也走同一规则**：`ro.product.marketname` 及其分区副本、以及 OnePlus/OPPO 别名 `ro.vendor.oplus.market.name`，只有真机上已存在的才会被写入（OEM 通常只导出少数几个副本）。**例外**：`custom_props` 里显式写出的 key 不受此限制，按你的要求照写。
:::

## Android 版本伪装

| 字段 | Build 字段 | 系统属性 | 示例 |
|------|-----------|----------|------|
| `android_version` | `Build.VERSION.RELEASE` | `ro.build.version.release`、`ro.system.build.version.release`、`ro.vendor.build.version.release`、`ro.product.build.version.release` | `"15"`, `"14"`, `"13"` |
| `sdk_int` | `Build.VERSION.SDK_INT`（整数） | `ro.build.version.sdk`、`ro.system.build.version.sdk`、`ro.vendor.build.version.sdk`、`ro.product.build.version.sdk` | `35`, `34`, `33` |

## DPI 伪装

在模板或 `[[apps]]` 中设置 `dpi`（范围 `120`–`640`），例如 `dpi = 420`。模块先保存当前的 DPI；目标应用退后台约 2 秒或退出后恢复原值，回到前台时重新应用。没有备份时会重置。

## 自定义属性

`custom_props` 可设置任意系统属性（直接写入，无分区变体展开），支持在模板与 `[[apps]]` 中使用，也支持[特殊标记值](#特殊标记值)：

```toml
[[apps]]
package = "com.custom.app"
manufacturer = "Custom"

[apps.custom_props]
"ro.custom.property" = "custom_value"
"ro.debug.mode" = "__DELETE__"    # 删除该属性
"ro.empty.value" = "__EMPTY__"    # 设为空字符串
```

## 特殊标记值

| 标记值 | 含义 |
|--------|------|
| 普通字符串 | 设置为该值 |
| `""` | 设置为空字符串（等价于 `__EMPTY__`） |
| `"__EMPTY__"` | 设置为空字符串 |
| `"__DELETE__"` | 删除该属性（经 companion resetprop） |

::: warning 结构化字段中的标记值行为不同
`__DELETE__` 会触发对应属性删除，但字面量 `__DELETE__` 仍会同时写入属性和 Build 字段；`__EMPTY__` 则完全按字面值写入。**建议只在 `custom_props` 中使用标记值**；结构化字段留空（`""` 或省略）即为不修改。
:::

## 功能开关

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `companion_resetprop` | `false` | `true` 时跳过 COW，所有属性交给 companion 进程 `resetprop` 写入（直写属性区，不经过 property_service），全系统读取一致；`false`（默认）时 COW 优先，仅影响当前进程内存。详见 [高级用法](./advanced.md#属性伪造机制) |
| `dpi` | — | 临时设置系统显示 density（`wm density`），范围 `120`–`640`；由 companion 保存并恢复原始 override |
| `force_denylist_unmount` | 继承 `default_force_denylist_unmount` | 对该应用强制启用 Zygisk `FORCE_DENYLIST_UNMOUNT`；优先级：应用 > 模板 > 全局默认 |

**注意**：

- 除 `package` 外所有字段均为可选
- 使用模板的 `packages` 时无需写 `[[apps]]`（自动应用）
- `[[apps]]` 中的字段整记录取代模板，未设置字段不回落（见[基础配置](./index.md#优先级)）
- 不在上表中的字段会被静默忽略（不影响解析）
