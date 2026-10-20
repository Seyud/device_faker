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

> **「分区变体」**指 `ro.product.{odm,vendor,system,system_ext,product,bootimage,system_dlkm,vendor_dlkm,odm_dlkm}.<字段>`。bionic 的属性读取按前缀路由到不同分区属性区，OnePlus/OPPO 等设备会读取这些变体，统一写入可保证各设备读取一致。
>
> 其中 **`system_dlkm` / `vendor_dlkm` / `odm_dlkm` 走「已存在才补齐」**：这三个动态内核模块分区只在 Android 13+ 的部分 ROM 上导出这些副本（Android 13+ 的部分 ROM 才有这些分区副本，旧设备可能一个都没有）。设备上不存在的 key 不会被写入——否则会在 `getprop` 全量输出与 `__system_property_foreach` 里留下真机没有的可枚举「幽灵属性」，而 app 对本就读不到的属性做伪装也没有收益。构建属性族 `ro.{system_dlkm,vendor_dlkm,odm_dlkm}.build.*` 同理。
>
> **营销名（`marketname`）整族同样走「已存在才补齐」**：`ro.product.marketname` 及其分区副本、以及 OnePlus/OPPO 别名 `ro.vendor.oplus.market.name`，只有在真机上已存在时才会被写入。OEM 营销名通常只导出 base 与 odm/vendor 等少数几个分区副本，其余副本（含非 OPPO 机型上的 `ro.vendor.oplus.market.name`）并不存在，无条件写入会在 `getprop` 全量输出里凭空多出真机没有的属性。COW 与 companion 两条路径都在写入前做存在性判断（companion 用进程内属性区查询，无额外进程开销）。**例外**：`custom_props` 里**显式写出**的 key 不受该规则限制——那是用户指名要求写入的属性（例如在非 OPPO 机型上伪装 OPPO 的 `ro.vendor.oplus.market.name`），照写不误；门控只作用于模块**自动展开**出来的分区/属性族变体。

### Android 版本与构建信息伪装

| 字段 | Build 字段 | 系统属性 | 示例 |
|------|-----------|----------|------|
| `android_version` | `Build.VERSION.RELEASE` | `ro.build.version.release`、`ro.system.build.version.release`、`ro.vendor.build.version.release`、`ro.product.build.version.release` | `"15"`, `"14"`, `"13"` |
| `sdk_int` | `Build.VERSION.SDK_INT`（整数） | `ro.build.version.sdk`、`ro.system.build.version.sdk`、`ro.vendor.build.version.sdk`、`ro.product.build.version.sdk` | `35`, `34`, `33` |
| `display_id` | `Build.DISPLAY` | `ro.build.display.id` | `PD2546_16.0.9.400(CN01)`，无分区变体 |
| `incremental` | `Build.VERSION.INCREMENTAL` | `ro.build.version.incremental` 及分区变体 | `compiler260228234011` |
| `security_patch` | `Build.VERSION.SECURITY_PATCH` | `ro.build.version.security_patch` 及分区变体 | `2025-06-05` |

> `security_patch` 会按 `ro.{system,system_ext,product,vendor,odm,bootimage,system_dlkm,vendor_dlkm,odm_dlkm}.build.version.security_patch` 整族写入，另写入裸名副本 `ro.vendor.build.security_patch`（即「供应商安全补丁级别」）。
>
> `sdk_int` 是数值字段，但同样支持字符串标记 `sdk_int = "__DELETE__"`，用于删除 `ro.build.version.sdk` 整族（高风险操作；WebUI 表单不提供该写法，需手改 TOML）。

### DPI伪装

在模板或 `[[apps]]` 中设置 `dpi`（范围 `120`–`640`），例如 `dpi = 420`。模块先保存当前的DPI；目标应用退后台约 2 秒或退出后恢复原值，回到前台时重新应用。没有备份时会重置`。

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

> **结构化字段**（`brand`、`manufacturer`、`model`、`name`、`device`、`marketname`、`fingerprint`、`build_id`、`incremental`、`security_patch`、`display_id`、`characteristics`、`hardware`、`board`、`soc_model`、`android_version`、`sdk_int`）写 `__DELETE__` 时：该字段对应的**属性族**全部进入删除列表（如 `brand` → `ro.product.brand` + 9 个分区副本），同时**不再写入属性、也不再 hook 对应 Build 字段**；`__EMPTY__` 则按空字符串写入（属性与 Build 字段都会写成空串）。少数字段额外删除 OEM 别名：`marketname` → `ro.vendor.oplus.market.name`，`product`/`name` → `ro.build.product`，`security_patch` → `ro.vendor.build.security_patch`。
>
> `sdk_int` 的 `__DELETE__` 写作 `sdk_int = "__DELETE__"`（数值字段的字符串标记，WebUI 表单只接受整数，需手改 TOML），删除 `ro.build.version.sdk` 整族——**SDK_INT 由该属性派生，删除后相关读取会失败，属高风险操作**。
>
> ⚠️ **删除只作用于全局属性，且只在 `companion_resetprop = true` 时生效**：默认的严格 COW 模式不写全局属性区，删除列表会被跳过并逐条打 WARN（`__DELETE__` 与 per-process 隔离天然冲突）。`__EMPTY__` 在 COW 模式可以正常生效（写进程内私有副本）。

### 功能开关

| 字段 | 默认值 | 说明 |
|------|--------|------|
| `companion_resetprop` | `false` | `true` 时跳过 COW，所有属性交给 companion 进程 `resetprop` 写入（直写属性区，绕过 property_service），全系统读取一致；`false`（默认）时 COW 优先，仅影响当前进程内存。详见 [属性伪造机制](#属性伪造机制) |
| `dpi` | — | 临时设置系统显示 density（`wm density`），范围 `120`–`640`；由 companion 保存并恢复原始 override |
| `force_denylist_unmount` | 继承 `default_force_denylist_unmount` | 对该应用强制启用 Zygisk `FORCE_DENYLIST_UNMOUNT`；优先级：应用 > 模板 > 全局默认 |

**注意**：
- 除 `package` 外所有字段均为可选
- 使用模板的 `packages` 时无需写 `[[apps]]`（自动应用）
- `[[apps]]` 中的字段整记录取代模板，未设置字段不回落（见[优先级](#优先级)）
- 不在上表中的字段会被静默忽略（不影响解析）

## 属性伪造机制

所有应用统一走同一执行流（无需选择模式）：

```
① JNI 覆写 Build 静态字段 → ② COW 或 companion resetprop → ③ DPI 伪装 → ④ DlClose 卸载模块
```

- **COW（默认）**：**只影响当前进程**；模块写完立即 DlClose，零驻留
- **companion resetprop（`companion_resetprop = true`）**：全部属性经 companion 进程直写属性区（`skip_svc`，绕过 property_service），**全系统读取一致**；应用退出或退后台约 2 秒后自动恢复原始值，回到前台重新应用

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
