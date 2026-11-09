# 字段参考

## 设备信息字段

以下字段可在模板或 `[[apps]]` 中使用。每个字段会同时改写应用内读到的 `android.os.Build` 字段与对应的系统属性，两者保持一致：

| 字段 | Build 字段 | 系统属性 | 说明 |
|------|-----------|----------|------|
| `manufacturer` | `Build.MANUFACTURER` | `ro.product.manufacturer` 及分区变体 | 厂商 (如: Xiaomi, Samsung) |
| `brand` | `Build.BRAND` | `ro.product.brand` 及分区变体 | 品牌 (如: Redmi, nubia) |
| `marketname` | —（仅属性） | `ro.product.marketname` + `ro.vendor.oplus.market.name` | 市场名 (如: Nothing Phone (3))；OnePlus/OPPO 设备读取后者 |
| `model` | `Build.MODEL` | `ro.product.model` 及分区变体 | 型号 (如: NX769J, SM-S9280) |
| `name` | —（仅属性） | `ro.product.name` 及分区变体 | 产品内部名 (如: popsicle) |
| `device` | `Build.DEVICE` | `ro.product.device` 及分区变体 | 代号 (如: xuanyuan)；未设置时依次回落到 `product` / `name` |
| `product` | `Build.PRODUCT` | `ro.product.name` 及分区变体 + `ro.build.product` | 产品代号 (如: xuanyuan)；与 `name` 同源，未设置时两者互相回填 |
| `hardware` | `Build.HARDWARE` | `ro.hardware` | 硬件名 (如: qcom)，无分区变体 |
| `board` | `Build.BOARD` | `ro.product.board` | 主板名 (如: kalama)，无分区变体 |
| `soc_model` | `Build.SOC_MODEL` | `ro.soc.model` | SoC 型号 (如: SM8550)，无分区变体 |
| `fingerprint` | `Build.FINGERPRINT` | `ro.build.fingerprint` | 指纹 |
| `build_id` | `Build.ID` | `ro.build.id`、`ro.system.build.id`、`ro.vendor.build.id`、`ro.product.build.id` | Build ID (如: UKQ1.230917.001) |
| `characteristics` | —（仅属性） | `ro.build.characteristics` | 特性 (如: tablet) |

::: info 什么是「分区变体」？
指 `ro.product.{odm,vendor,system,system_ext,product,bootimage,system_dlkm,vendor_dlkm,odm_dlkm}.<字段>`：不同厂商的 ROM 会从不同分区读取机型信息，整族写入才能让各条读取路径拿到一致的值。

设备上**原本不存在**的属性不会被写入——例如部分 ROM 没有 `ro.product.system_dlkm.model`，非 OPPO 机型没有 `ro.vendor.oplus.market.name`。这样不会凭空多出真机没有的属性，也不影响伪装效果。**例外**：`custom_props` 里显式写出的 key 按你的要求照写。
:::

::: tip 指纹会自动补全其它字段
只写 `fingerprint` 也能得到一致的伪装：模块会解析指纹，把**未显式填写**的 `brand`、`product`、`device`、`android_version`、`build_id`、`incremental` 自动补上（已写的字段不会被覆盖）。补出来的 `incremental` 会同时驱动 `Build.VERSION.INCREMENTAL` 与 `ro.build.version.incremental`。
:::

## Android 版本与构建信息伪装

| 字段 | Build 字段 | 系统属性 | 示例 |
|------|-----------|----------|------|
| `android_version` | `Build.VERSION.RELEASE` | `ro.build.version.release`、`ro.system.build.version.release`、`ro.vendor.build.version.release`、`ro.product.build.version.release` | `"15"`, `"14"`, `"13"` |
| `sdk_int` | `Build.VERSION.SDK_INT`（整数） | `ro.build.version.sdk`、`ro.system.build.version.sdk`、`ro.vendor.build.version.sdk`、`ro.product.build.version.sdk` | `35`, `34`, `33` |
| `display_id` | `Build.DISPLAY` | `ro.build.display.id` | `PD2546_16.0.9.400(CN01)`，无分区变体 |
| `incremental` | `Build.VERSION.INCREMENTAL` | `ro.build.version.incremental` 及分区变体 | `compiler260228234011` |
| `security_patch` | `Build.VERSION.SECURITY_PATCH` | `ro.build.version.security_patch` 及分区变体 | `2025-06-05` |

> `security_patch` 会连同各分区副本一起写入。
> `sdk_int` 是数值字段，但支持字符串标记 `sdk_int = "__DELETE__"` 删除 `ro.build.version.sdk` 整族（高风险操作，WebUI 表单不提供，需手改 TOML）。

## DPI 伪装

在模板或 `[[apps]]` 中设置 `dpi`（范围 `120`–`640`），例如 `dpi = 420`。模块先保存当前的 DPI；目标应用退到后台或退出后恢复原值，回到前台时重新应用。原本没有保存时走 DPI 重置。

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
| `"__DELETE__"` | 删除该属性（**仅在 `companion_resetprop = true` 时执行**） |

::: warning 结构化字段中的标记值行为不同
`__DELETE__` 会触发对应属性删除（**同样只在 `companion_resetprop = true` 时生效**），字面量 `__DELETE__` 仍会同时写入属性和 Build 字段；`__EMPTY__` 则完全按字面值写入。**建议只在 `custom_props` 中使用标记值**；结构化字段留空（`""` 或省略）即为不修改。
:::

## 功能开关

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `companion_resetprop` | `false` | `true` 时伪装值对**全系统生效**（`getprop`、其它应用也能读到），应用切到后台或退出后自动恢复真值；`false`（默认）时只在应用自己的进程内生效。详见 [高级用法](./advanced.md#生效方式) |
| `dpi` | — | 临时修改系统显示密度（范围 `120`–`640`）；应用退到后台或退出后自动恢复原值 |
| `force_denylist_unmount` | 继承 `default_force_denylist_unmount` | 对该应用强制启用 Zygisk `FORCE_DENYLIST_UNMOUNT`；优先级：应用 > 模板 > 全局默认 |

**注意**：

- 除 `package` 外所有字段均为可选
- 使用模板的 `packages` 时无需写 `[[apps]]`（自动应用）
- `[[apps]]` 中的字段整记录取代模板，未设置字段不回落（见[基础配置](./index.md#优先级)）
- 不在上表中的字段会被静默忽略（不影响解析）
