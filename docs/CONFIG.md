# 配置说明

为不同的应用配置不同的伪装机型与系统属性。配置文件使用 TOML 格式。

## 配置文件路径

- `/data/adb/device_faker/config/config.toml`

配置为**热加载**：模块在每次应用启动时重新读取配置文件。修改配置后只需重启目标应用，**无需重启系统**。

- 配置文件缺失或解析失败时，该应用跳过伪装并卸载模块，不影响其他应用
- 未出现在任何配置中的应用不做任何伪装

## 全局设置

```toml
debug = false                        # 调试日志（默认关闭）
default_force_denylist_unmount = false
```

- `debug`：启用后输出 Info 级别日志（关闭时仅 Error），写入 `/data/adb/device_faker/logs/device_faker.log`；正常使用建议关闭以提高隐蔽性
- `default_force_denylist_unmount`：为目标应用启用 Zygisk 的 `FORCE_DENYLIST_UNMOUNT`，可在模板/应用里用 `force_denylist_unmount` 覆盖

## 编辑配置

> 多用户说明：支持在包名后追加 `@userId` 来只对指定用户生效。
> 
> - `userId` 对应路径 `/data/user/<userId>/...` 中的数字（例如 `0`、`999`）
> - 匹配优先级：先匹配 `com.example.app@userId`，找不到再回退匹配 `com.example.app`
> - 该写法同时适用于 `apps` 里的 `package` 和模板的 `packages` 列表
> - **裸包名（不带 `@userId`）是「user 0 的配置」+「所有分身/多开用户（非 0）的回退配置」双语义**。
>   因此想让主应用和它的分身用不同模板时，裸包名只写在一个模板里，分身在另一个模板里写
>   `com.example.app@999`；**不要把同一个裸包名同时写进两个模板**（运行时只认书写顺序第一个，
>   WebUI 会给出重复包名告警）。反过来，修改裸包名的配置会影响所有依赖回退的分身。
>
> 作用范围速查（运行时按 `pkg@uid` → 裸 `pkg` 顺序查找，命中即停）：
>
> | 写法 | 作用范围 |
> | --- | --- |
> | `com.example.app` | user 0 **+ 所有分身**（分身没单独配置时回退到这里） |
> | `com.example.app@0` | **仅 user 0**（精确键优先于裸名，分身不会命中） |
> | `com.example.app@999` | **仅该分身**（user 999） |
>
> 只想让主应用生效、不让分身继承时用 `@0`；想让主应用和所有分身都生效时用裸包名。

### 方式一：机型模板

在模板中定义 `packages` 列表，自动应用到所有包名：

```toml
[templates.redmagic_9_pro]
packages = [
    "com.mobilelegends.mi",
  # 仅对 userId=999 生效
  # "com.mobilelegends.mi@999",
    "com.supercell.brawlstars",
]
manufacturer = "ZTE"
brand = "nubia"
model = "NX769J"
device = "REDMAGIC 9 Pro"
fingerprint = "nubia/NX769J/NX769J:14/UKQ1.230917.001/20240813.173312:user/release-keys"
build_id = "UKQ1.230917.001"

# 无需写 [[apps]]，所有包名自动使用该模板
```

**优点**：
- ✅ 集中管理机型和包名
- ✅ 无需重复写 [[apps]]
- ✅ 一目了然地看到哪些应用使用哪个模板

### 方式二：直接配置

使用 `[[apps]]` 为单个应用指定设备信息：

```toml
[[apps]]
package = "com.omarea.vtools"
manufacturer = "Xiaomi"
brand = "Xiaomi"
model = "2509FPN0BC"
device = "Xiaomi 15 Pro"
product = "popsicle"
name = "popsicle"
```

**注意**：`package` 是唯一**必填**字段，缺失会导致整个配置解析失败。

### 优先级

```
[[apps]] 直接配置 > 模板 packages 列表 > 全局默认值
```

- **`[[apps]]` 是整记录匹配**：一旦包名命中 `[[apps]]`，该条记录**整体取代**模板——记录中未设置的字段**不会**回落模板，只套用全局默认值
- 未命中 `[[apps]]` 时，在模板的 `packages` 列表中查找
- 最终套用全局默认值：`force_denylist_unmount` ← `default_force_denylist_unmount`
- 未匹配任何配置的应用：不做伪装并卸载模块

**覆盖模板示例**：

```toml
[templates.redmagic_9_pro]
packages = ["com.mobilelegends.mi"]  # 默认使用这个模板
manufacturer = "ZTE"
brand = "nubia"

[[apps]]
package = "com.mobilelegends.mi"  # 命中后整记录取代模板
manufacturer = "Samsung"
# 模板中设置了但这里未写的字段（如 brand）不会生效
```

## 字段说明

### 设备信息字段

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

> **「分区变体」**指 `ro.product.{odm,vendor,system,system_ext,product,bootimage,system_dlkm,vendor_dlkm,odm_dlkm}.<字段>`：不同厂商的 ROM 会从不同分区读取机型信息，整族写入才能让各条读取路径拿到一致的值。
>
> 设备上**原本不存在**的属性不会被写入——例如部分 ROM 没有 `ro.product.system_dlkm.model`，非 OPPO 机型没有 `ro.vendor.oplus.market.name`。这样不会凭空多出真机没有的属性，也不影响伪装效果。**例外**：`custom_props` 里显式写出的 key 按你的要求照写。

> **指纹会自动补全其它字段**：只写 `fingerprint` 也能得到一致的伪装——模块解析指纹后，把**未显式填写**的 `brand`、`product`、`device`、`android_version`、`build_id`、`incremental` 自动补上（已写的字段不会被覆盖）。补出来的 `incremental` 会同时驱动 `Build.VERSION.INCREMENTAL` 与 `ro.build.version.incremental`。

### Android 版本与构建信息伪装

| 字段 | Build 字段 | 系统属性 | 示例 |
|------|-----------|----------|------|
| `android_version` | `Build.VERSION.RELEASE` | `ro.build.version.release`、`ro.system.build.version.release`、`ro.vendor.build.version.release`、`ro.product.build.version.release` | `"15"`, `"14"`, `"13"` |
| `sdk_int` | `Build.VERSION.SDK_INT`（整数） | `ro.build.version.sdk`、`ro.system.build.version.sdk`、`ro.vendor.build.version.sdk`、`ro.product.build.version.sdk` | `35`, `34`, `33` |
| `display_id` | `Build.DISPLAY` | `ro.build.display.id` | `PD2546_16.0.9.400(CN01)`，无分区变体 |
| `incremental` | `Build.VERSION.INCREMENTAL` | `ro.build.version.incremental` 及分区变体 | `compiler260228234011` |
| `security_patch` | `Build.VERSION.SECURITY_PATCH` | `ro.build.version.security_patch` 及分区变体 | `2025-06-05` |

> `security_patch` 会连同各分区副本一起写入。

### DPI伪装

在模板或 `[[apps]]` 中设置 `dpi`（范围 `120`–`640`），例如 `dpi = 420`。模块先保存当前的 DPI；目标应用退到后台或退出后恢复原值，回到前台时重新应用。原本没有保存时走 DPI 重置。

### 自定义属性

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

### 特殊标记值

| 标记值 | 含义 |
|--------|------|
| 普通字符串 | 设置为该值 |
| `""` | 设置为空字符串（等价于 `__EMPTY__`） |
| `"__EMPTY__"` | 设置为空字符串 |
| `"__DELETE__"` | 删除该属性（结构化字段则删除对应属性族；**仅在 `companion_resetprop = true` 时真正执行**） |

> **结构化字段**（`brand`、`model`、`fingerprint`、`android_version` 等）写 `__DELETE__` 时：该字段不再写入，并删除它对应的整组属性；`__EMPTY__` 则按空字符串写入（Build 字段同样写成空串）。少数字段会一并处理厂商别名（如 `marketname`、`security_patch`）。
>
> `sdk_int` 的写法是 `sdk_int = "__DELETE__"`（数值字段的字符串标记，WebUI 表单只接受整数，需手改 TOML）——**删除后相关读取会失败，属高风险操作**。
>
> ⚠️ **删除只在 `companion_resetprop = true` 时执行**：默认模式下删除列表会被跳过（`__DELETE__` 与「只影响本应用」天然冲突）。`__EMPTY__` 在默认模式下也能正常生效。

### 功能开关

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `companion_resetprop` | `false` | `true` 时伪装值对**全系统生效**（`getprop`、其它应用也能读到），应用切到后台或退出后自动恢复真值；`false`（默认）时只在应用自己的进程内生效。需要全局生效时再开启，详见 [生效方式](#生效方式) |
| `dpi` | — | 临时修改系统显示密度（范围 `120`–`640`）；应用退到后台或退出后自动恢复原值 |
| `force_denylist_unmount` | 继承 `default_force_denylist_unmount` | 对该应用强制启用 Zygisk `FORCE_DENYLIST_UNMOUNT`；优先级：应用 > 模板 > 全局默认 |

**注意**：
- 除 `package` 外所有字段均为可选
- 使用模板的 `packages` 时无需写 `[[apps]]`（自动应用）
- `[[apps]]` 中的字段整记录取代模板，未设置字段不回落（见[优先级](#优先级)）
- 不在上表中的字段会被静默忽略（不影响解析）

## 生效方式

所有应用统一走同一执行流（无需选择模式）：

```
① 覆写 Build 字段 → ② 修改系统属性 → ③ DPI 伪装 → ④ 卸载模块
```

- **默认（推荐）**：伪装只在**应用自己的进程内**生效——应用里读到的都是伪装值，其它应用、系统服务和 `getprop` 读到的仍是真值；模块改完立即卸载，不常驻、不残留。
- **`companion_resetprop = true`**：伪装值**对全系统生效**（`getprop`、其它应用也能读到）。应用切到后台或退出后自动恢复真值，回到前台重新应用。

需要让检测工具（多数通过 `getprop` 子进程读取属性）也看到伪装值时，才需要开启 `companion_resetprop`；代价是应用运行期间伪装值对全局可见。

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
