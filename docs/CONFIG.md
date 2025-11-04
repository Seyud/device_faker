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
  - 修改 Build 类 + Hook SystemProperties
  - 模块驻留内存
  - 可能被检测
  - 仅在 lite 不够用时使用

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

### 方式一：机型模板

在模板中定义 `packages` 列表，自动应用到所有包名：

```toml
# 定义模板并列出包名
[templates.redmagic_9_pro]
packages = [
    "com.mobilelegends.mi",
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
fingerprint = "Xiaomi/2509FPN0BC/Xiaomi15Pro:14/UP1A.231005.007/V816.0.3.0.VNBCNXM:user/release-keys"
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
| 字段 | 对应系统属性 | 说明 |
|------|------------|------|
| `manufacturer` | `ro.product.manufacturer` | 制造商 (如: Xiaomi, Samsung) |
| `brand` | `ro.product.brand` | 品牌 (如: Redmi, Nothing) |
| `marketname` | `ro.product.marketname` | 市场名称 (如: REDMI K90 Pro Max) |
| `model` | `ro.product.model` | 型号 (如: 25102RK69C) |
| `name` | `ro.product.name` | 产品名称 (如: myron) |
| `device` | `ro.product.device` | 设备代号 (如: myron) |
| `product` | `ro.build.product` | 构建产品 (如: myron) |
| `fingerprint` | `ro.build.fingerprint` | 指纹信息 |

**注意**:
- 除了 `package` 外,所有字段都是可选的
- 使用模板的 `packages` 时,无需写 [[apps]](自动应用)
- [[apps]] 中的字段会覆盖模板的配置
- `name` 和 `marketname` 仅在 **full 模式**下有效(影响 SystemProperties)
- **lite 模式**下,只有 `manufacturer`、`brand`、`model`、`device`、`product`、`fingerprint` 生效

## 模式对比

| 特性 | lite 模式 ⭐ | full 模式 |
|------|-------------|-----------|
| Build 类伪装 | ✅ | ✅ |
| SystemProperties Hook | ❌ | ✅ |
| 模块可卸载 | ✅ | ❌ |
| 隐蔽性 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |
| 被检测风险 | 极低 | 中等 |
| 推荐度 | ⭐⭐⭐⭐⭐ | ⭐⭐⭐ |

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
