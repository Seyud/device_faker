# 常见问题

## 安装相关

### 安装时提示「未检测到 Zygisk 实现」

Device Faker **不支持 Magisk 自带的 Zygisk**。必须先安装第三方 Zygisk 实现（推荐 **ZygiskNext**），重启后再安装本模块。

### 支持 Magisk 自带的 Zygisk 吗？

不支持。Magisk 内置 Zygisk 的隐藏性不能满足本模块的运行要求。

### 安装后模块不生效

1. 是否已安装并启用 Zygisk 实现？
2. 安装完成后是否**重启了设备**？
3. 配置文件路径是否正确：`/data/adb/device_faker/config/config.toml`
4. 目标应用的包名是否写对？

### 更新模块后配置会丢吗？

覆盖安装时，默认会保留现有配置（除非在安装选择窗口按了音量 + 使用默认配置）。

::: warning 卸载会删除配置
卸载模块时会执行 `rm -rf /data/adb/device_faker`，配置目录会被一并删除。需要保留配置请先手动备份。
:::

## 配置相关

### 配置改了没生效

- 配置是**热加载**，修改后需要**重启目标应用**，不是重启系统
- 检查 `config.toml` 是否有语法错误：TOML 对引号、缩进敏感，一个缺失的引号会导致整份配置解析失败
- 检查目标应用是否命中了 `[[apps]]` 或模板的 `packages`

### /data/adb/device_faker/config/config.toml 找不到

首次安装时如果按了音量 + 选择使用默认配置，配置路径就是上述路径。如果目录不存在，说明安装后未重启或安装被中止。

### 优先级到底怎么算？

```
[[apps]] 直接配置 > 模板 packages 列表 > 全局默认值
```

一旦命中 `[[apps]]`，该记录**整体取代**模板，未设置的字段不会回落到模板。

### 怎么只对某个用户（工作资料 / 多开）生效？

在包名后追加 `@userId`：

```toml
[[apps]]
package = "com.example.app@999"
model = "FakeModel"
```

匹配顺序：先匹配 `@999`，找不到再回退匹配无后缀的包名。

### 配置写错了会怎样？

- 单条 `[[apps]]` 缺少 `package`：整份配置解析失败，所有应用都不伪装
- 某个字段值无效：该字段会被静默忽略（如未知字段）
- 某条模板写错：只影响命中该模板的应用

## 属性与性能

### __DELETE__ 和留空有什么区别？

| 写法 | 行为 |
|------|------|
| 结构化字段省略或 `""` | 不修改该字段 |
| `custom_props` 里 `"key" = ""` | 把该属性设为空字符串 |
| `custom_props` 里 `"key" = "__EMPTY__"` | 把该属性设为空字符串 |
| `custom_props` 里 `"key" = "__DELETE__"` | 删除该属性（**仅在 `companion_resetprop = true` 时执行**，默认严格 COW 下跳过并记 WARN） |

## WebUI 相关

### 在线模板库加载失败

- 检查设备网络连接
- 模板源地址：https://github.com/Seyud/device_faker_config
- 部分地区访问 GitHub 需要网络工具
