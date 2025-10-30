# 配置说明

## 配置文件路径
- `/data/adb/device_faker/config/config.toml`

配置文件使用 TOML 格式，示例:

```toml
[[apps]]
package = "com.omarea.vtools"
manufacturer = "Xiaomi"
brand = "Xiaomi"
marketname = "Xiaomi 17 Pro Max"
model = "2509FPN0BC"
name = "popsicle"

[[apps]]
package = "com.coolapk.market"
manufacturer = "Nothing"
brand = "Nothing"
marketname = "Nothing Phone (3)"
model = "A024"
```

## 字段说明

所有字段除了 `package` 外都是**可选的**，只配置需要伪装的字段即可:

- `package` (必需): 应用包名
- `manufacturer` (可选): 制造商，伪装 `Build.MANUFACTURER` 和 `ro.product.manufacturer`
- `brand` (可选): 品牌，伪装 `Build.BRAND` 和 `ro.product.brand`
- `marketname` (可选): 市场名称，伪装 `ro.product.marketname`
- `model` (可选): 型号，伪装 `Build.MODEL` 和 `ro.product.model`
- `name` (可选): 产品名，伪装 `Build.PRODUCT`、`Build.DEVICE` 和 `ro.product.name`

## 最小配置示例

只伪装部分字段:

```toml
[[apps]]
package = "com.example.app"
model = "Pixel 9"
brand = "Google"

[[apps]]
package = "com.another.app"
manufacturer = "Samsung"
```
