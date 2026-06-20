# 配置说明

## 配置文件路径
- `/data/adb/device_faker/config/config.toml`

配置文件使用 TOML 格式

## 全局设置

### default_mode（全局默认模式）

```toml
default_mode = "lite"  # 推荐：轻量模式（隐藏性更好）
```

**可选值**：
- `"lite"` - 轻量模式（推荐）⭐
  - 只修改 Build 类静态字段
  - 完成后卸载模块
  - 不易被检测
  - 适合 90% 的应用

- `"full"` - 完整模式
  - 修改 Build 类 + 伪装 SystemProperties
  - 模块驻留内存
  - 可能被检测
  - 仅在 lite 不够用时使用

- `"companion"` - Companion 模式
  - 使用 companion 进程通过 resetprop 修改属性
  - 支持修改只读属性
  - 支持自定义属性和属性置空/删除
  - 配置 CPU 预设或自定义 CPU 信息时，会伪装 `/proc/cpuinfo`
  - 在应用进入 companion 模式前会用 `getprop` 备份原始值，退出或切换到其它应用后由 companion 自动还原

### default_force_denylist_unmount（全局默认卸载挂载点）

```toml
# 默认 false：仅在需要的应用上开启
default_force_denylist_unmount = false
```

**说明**：为目标应用启用 Zygisk 的 `FORCE_DENYLIST_UNMOUNT`，强制卸载模块挂载痕迹。可在全局开启，也可在模板 / 单个应用里覆盖。

### debug（调试模式）

```toml
debug = true  # 启用详细日志（用于调试）
# debug = false  # 或删除此行，默认关闭（正常使用）
```

**说明**：
- 启用后会输出详细的 Info 级别日志
- 关闭时只输出 Error 级别日志
- 正常使用建议关闭以提高隐蔽性

## 编辑配置

> 多用户说明：支持在包名后追加 `@userId` 来只对指定用户生效。
> 
> - `userId` 对应路径 `/data/user/<userId>/...` 中的数字（例如 `0`、`999`）
> - 匹配优先级：先匹配 `com.example.app@userId`，找不到再回退匹配 `com.example.app`
> - 该写法同时适用于 `apps` 里的 `package` 和模板的 `packages` 列表

### 方式一：机型模板

在模板中定义 `packages` 列表，自动应用到所有包名：

```toml
# 定义模板并列出包名
[templates.redmagic_9_pro]
packages = [
    "com.mobilelegends.mi",
  # 仅对 userId=999 生效
  # "com.mobilelegends.mi@999",
    "com.supercell.brawlstars",
    "com.blizzard.diablo.immortal",
]
manufacturer = "ZTE"
brand = "nubia"
model = "NX769J"
device = "REDMAGIC 9 Pro"
fingerprint = "nubia/NX769J/NX769J:14/UKQ1.230917.001/20240813.173312:user/release-keys"
build_id = "UKQ1.230917.001"

[templates.pixel_xl]
packages = [
    "com.google.android.apps.photos",
]
manufacturer = "Google"
brand = "google"
model = "marlin"
device = "Pixel XL"
product = "marlin"
fingerprint = "google/marlin/marlin:10/QP1A.191005.007.A3/5972272:user/release-keys"
build_id = "QP1A.191005.007.A3"

# 无需写 [[apps]]，所有包名自动使用该模板
```

**优点**：
- ✅ 集中管理机型和包名
- ✅ 无需重复写 [[apps]]
- ✅ 一目了然地看到哪些应用使用哪个模板

### 方式二：直接配置

使用 [[apps]] 为单个应用指定设备信息：

```toml
[[apps]]
package = "com.omarea.vtools"
manufacturer = "Xiaomi"
brand = "Xiaomi"
model = "2509FPN0BC"
device = "Xiaomi 15 Pro"
product = "popsicle"
name = "popsicle"  # 产品内部名称（仅 full 模式生效）
mode = "full"  # 可选：覆盖全局模式

[[apps]]
package = "com.coolapk.market"
manufacturer = "Nothing"
brand = "Nothing"
marketname = "Nothing Phone (3)"
model = "A024"
```

**优点**：
- ✅ 配置灵活
- ✅ 适合一次性配置或覆盖模板

**覆盖模板**：
如果一个包名既在模板的 `packages` 中，又有 [[apps]] 配置，则 [[apps]] 优先：

```toml
[templates.redmagic_9_pro]
packages = [
    "com.mobilelegends.mi",  # 默认使用这个模板
]
manufacturer = "ZTE"
model = "NX769J"

[[apps]]
package = "com.mobilelegends.mi"  # 覆盖模板配置
manufacturer = "Samsung"
model = "SM-S9280"
```

**字段优先级**：
```
[[apps]] 直接配置 > 模板 packages 列表 > 全局 default_mode
```

**模式优先级**：
```
[[apps]].mode > [templates].mode > 全局 default_mode
```

### 应用配置字段说明

**字段与系统属性映射关系**:
| 字段 | lite 模式 | full/companion 模式 (SystemProperties) | 说明 |
|------|----------|------------------------------|------|
| `manufacturer` | `Build.MANUFACTURER` | + `ro.product.manufacturer` | 厂商 (如: Xiaomi, Samsung) |
| `brand` | `Build.BRAND` | + `ro.product.brand` | 品牌 (如: Redmi, nubia) |
| `model` | `Build.MODEL` | + `ro.product.model` | 序号 (如: 25010PN30C，NX769J) |
| `device` | `Build.DEVICE` | (仅 Build 字段) | 代号 (如: xuanyuan，NX769J) |
| `product` | `Build.PRODUCT` | (仅 Build 字段) | 代号 (如: xuanyuan，NX769J) |
| `fingerprint` | `Build.FINGERPRINT` | + `ro.build.fingerprint` | 指纹 |
| `build_id` | `Build.ID` | + `ro.build.id` 等 | Build ID (如: UKQ1.230917.001) |
| `name` | ❌ | `ro.product.name` + `ro.product.device` | 代号 (如: xuanyuan) |
| `marketname` | ❌ | `ro.product.marketname` | 型号 (如: REDMI K90 Pro Max) |
| `characteristics` | ❌ | `ro.build.characteristics` | 特性 (如: tablet) |
| `android_version` | `Build.VERSION.RELEASE` | + `ro.build.version.release` 等 | Android 版本号 (如: 15, 14) |
| `sdk_int` | `Build.VERSION.SDK_INT` | + `ro.build.version.sdk` 等 | SDK 版本号 (如: 35, 34) |
| `custom_props` | ❌ | ✅ | 自定义属性映射表 |
| `force_denylist_unmount` | N/A | N/A | 是否对该应用强制卸载模块挂载点；未指定时使用 `default_force_denylist_unmount` |

**Android 版本伪装字段**:
| 字段 | 说明 | 示例 |
|------|------|------|
| `android_version` | Android 版本号，所有模式都支持 | `"15"`, `"14"`, `"13"` |
| `sdk_int` | SDK 版本号，所有模式都支持 | `35`, `34`, `33` |

**自定义属性字段**:
| 字段 | 说明 |
|------|------|
| `custom_props` | 自定义属性映射表，仅 full/companion 模式支持 |

**配置元数据字段**（仅用于显示，不影响伪装效果）:
| 字段 | 说明 |
|------|------|
| `version` | 配置版本号 (如: "v1.0") |
| `version_code` | 配置版本码 (如: 20251212) |
| `author` | 配置作者 |
| `description` | 配置描述信息 |

**关于 `force_denylist_unmount`**：
- 可写在全局（`default_force_denylist_unmount`）、模板或单个 `[[apps]]`。
- 优先级：单个应用 > 模板 > 全局默认。
- 适合微信等敏感 App，建议按需开启而非全局强开。

**注意**:
- 除了 `package` 外,所有字段都是可选的
- 使用模板的 `packages` 时,无需写 [[apps]](自动应用)
- [[apps]] 中的字段会覆盖模板的配置
- `name` 和 `marketname` 仅在 **full/companion 模式**下有效(影响 SystemProperties)
- `name` 字段在 full/companion 模式下会同时伪装 `ro.product.name` 和 `ro.product.device`
- `characteristics` 字段仅在 **full/companion 模式**下生效
- **lite 模式**下,只有 `manufacturer`、`brand`、`model`、`device`、`product`、`fingerprint`、`build_id`、`android_version`、`sdk_int` 生效

## Build ID 伪装

**Build ID 会修改的属性**：

| 模式 | Build 字段 | 系统属性 |
|------|-----------|----------|
| lite | `ID` | ❌ |
| full | `ID` | `ro.build.id`, `ro.system.build.id`, `ro.vendor.build.id`, `ro.product.build.id` |
| companion | `ID` | `ro.build.id`, `ro.system.build.id`, `ro.vendor.build.id`, `ro.product.build.id` |

## Android 版本伪装

```toml
# 模板示例：伪装为 Android 15
[templates.android_15]
packages = ["com.app.needs.android15"]
manufacturer = "Google"
brand = "google"
model = "Pixel 9 Pro"
android_version = "15"
sdk_int = 35

# 应用示例：伪装为旧版 Android
[[apps]]
package = "com.needs.old.android"
mode = "lite"  # lite 模式也支持！
android_version = "13"
sdk_int = 33
```

**版本伪装会修改的属性**：

| 模式 | Build.VERSION 字段 | 系统属性 |
|------|-------------------|----------|
| lite | `RELEASE`, `SDK_INT` | ❌ |
| full | `RELEASE`, `SDK_INT` | `ro.build.version.release`, `ro.build.version.sdk` 等 |
| companion | `RELEASE`, `SDK_INT` | `ro.build.version.release`, `ro.build.version.sdk` 等 |

**完整的系统属性列表**（full/companion 模式）：
- `ro.build.version.release`
- `ro.system.build.version.release`
- `ro.vendor.build.version.release`
- `ro.product.build.version.release`
- `ro.build.version.sdk`
- `ro.system.build.version.sdk`
- `ro.vendor.build.version.sdk`
- `ro.product.build.version.sdk`

## 自定义属性

**full/companion 模式** 都支持自定义属性，可以设置任意系统属性：

```toml
[[apps]]
package = "com.custom.app"
mode = "companion"
manufacturer = "Custom"

# 自定义属性
[apps.custom_props]
"ro.custom.property" = "custom_value"
"ro.another.prop" = "another_value"
```

### 特殊标记值

支持使用特殊标记值来执行特殊操作：

| 标记值 | 含义 | 示例 |
|--------|------|------|
| 普通字符串 | 设置为该值 | `"ro.prop" = "value"` |
| `""` 或省略 | 不修改（保持原值） | `brand = ""` |
| `"__EMPTY__"` | 设置为空字符串 | `brand = "__EMPTY__"` |
| `"__DELETE__"` | 删除该属性 | `model = "__DELETE__"` |

**示例**：

```toml
[[apps]]
package = "com.example.app"
mode = "companion"
manufacturer = "Google"
brand = "__EMPTY__"           # 将 brand 设置为空字符串
model = "__DELETE__"          # 删除 model 属性

# 自定义属性也支持特殊标记
[apps.custom_props]
"ro.custom.flag" = "enabled"
"ro.debug.mode" = "__DELETE__"
"ro.empty.value" = "__EMPTY__"
```

## 模式对比

| 特性 | lite 模式 ⭐ | full 模式 | companion 模式 |
|------|-------------|-----------|----------------|
| Build 类伪装 | ✅ | ✅ | ✅ |
| SystemProperties 伪装 | ❌ | ✅ | ✅ |
| characteristics 伪装 | ❌ | ✅ | ✅ |
| 只读属性修改 | ❌ | ❌ | ✅ |
| 自定义属性 | ❌ | ✅ | ✅ |
| 属性置空/删除 | ❌ | ✅ | ✅ |
| Android 版本伪装 | ✅ | ✅ | ✅ |
| SDK 版本伪装 | ✅ | ✅ | ✅ |
| CPU 信息伪装 | ❌ | ❌ | ✅ |
| 模块可卸载 | ✅ | ❌ | ✅ |
| 隐蔽性 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| 被检测风险 | 极低 | 较低 | 较低 |
| 推荐度 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ |

## 如何选择模式？

### 判断依据

**使用 lite 模式**：
- ✅ 大多数应用
- ✅ 追求隐蔽性
- ✅ 不想被检测

**使用 full 模式**：
- 应用会读取 SystemProperties
- lite 模式下仍能检测到真实机型
- 需要伪装 characteristics（如 QQ 平板模式）
- 需要自定义属性

**使用 companion 模式**：
- 需要修改只读属性
- 需要删除或置空某些属性
- 需要完整的自定义属性支持
- 需要伪装 `/proc/cpuinfo`
