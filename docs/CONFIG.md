# 配置说明

## 配置文件路径
- `/data/adb/device_faker/config/config.toml`

配置文件使用 TOML 格式

## 全局设置

### default_mode（全局默认模式）

```toml
default_mode = "lite"  # 推荐：轻量模式（隐藏性更好）
# default_mode = "full"  # 完整模式（可能被检测）
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

- `"resetprop"` - Resetprop 模式
  - 使用 resetprop 工具修改属性
  - 支持修改只读属性（如 `ro.build.characteristics`）
  - 在应用进入 resetprop 模式前会用 `getprop` 备份原始值，退出或切换到其它应用后由守护进程用 resetprop 自动还原

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
| 字段 | lite 模式 | full 模式 (SystemProperties) | 说明 |
|------|----------|------------------------------|------|
| `manufacturer` | `Build.MANUFACTURER` | + `ro.product.manufacturer` | 厂商 (如: Xiaomi, Samsung) |
| `brand` | `Build.BRAND` | + `ro.product.brand` | 品牌 (如: Redmi, nubia) |
| `model` | `Build.MODEL` | + `ro.product.model` | 序号 (如: 25010PN30C，NX769J) |
| `device` | `Build.DEVICE` | (仅 Build 字段) | 代号 (如: xuanyuan，NX769J) |
| `product` | `Build.PRODUCT` | (仅 Build 字段) | 代号 (如: xuanyuan，NX769J) |
| `fingerprint` | `Build.FINGERPRINT` | + `ro.build.fingerprint` | 指纹 |
| `name` | ❌ | `ro.product.name` + `ro.product.device` | 代号 (如: xuanyuan) |
| `marketname` | ❌ | `ro.product.marketname` | 型号 (如: REDMI K90 Pro Max) |
| `characteristics` | ❌ | `ro.build.characteristics` | 特性 (如: tablet) - 仅 resetprop 模式生效 |
| `force_denylist_unmount` | N/A | N/A | 是否对该应用强制卸载模块挂载点；未指定时使用 `default_force_denylist_unmount` |

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
- `name` 和 `marketname` 仅在 **full 模式**下有效(影响 SystemProperties)
- `name` 字段在 full 模式下会同时伪装 `ro.product.name` 和 `ro.product.device`
- `characteristics` 字段仅在 **resetprop 模式**下生效
- **lite 模式**下,只有 `manufacturer`、`brand`、`model`、`device`、`product`、`fingerprint` 生效

## 模式对比

| 特性 | lite 模式 ⭐ | full 模式 | resetprop 模式 |
|------|-------------|-----------|----------------|
| Build 类伪装 | ✅ | ✅ | ✅ |
| SystemProperties 伪装 | ❌ | ✅ | ✅ |
| 只读属性修改 | ❌ | ❌ | ✅ |
| 模块可卸载 | ✅ | ❌ | ❌ |
| 隐蔽性 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ |
| 被检测风险 | 极低 | 较低 | 较低 |
| 推荐度 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ | ⭐⭐ |

## 如何选择模式？

### 推荐策略

1. **默认使用 lite 模式** - 覆盖 90% 的场景
2. **测试应用** - 看是否能正常使用
3. **如果不行** - 再改用 full 模式

### 判断依据

**使用 lite 模式**：
- ✅ 大多数应用
- ✅ 追求隐蔽性
- ✅ 不想被检测

**使用 full 模式**：
- 应用会读取 SystemProperties
- lite 模式下仍能检测到真实机型
- 不在乎被检测到模块

**使用 resetprop 模式**：
- 需要修改 `ro.build.characteristics` (如 QQ 平板模式)
