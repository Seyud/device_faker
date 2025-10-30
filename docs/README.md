# Device Faker 📱

一个基于 Zygisk 的机型伪装模块,可以为不同的应用配置不同的设备型号。

## 特性 ✨

- 🎯 **精确控制**: 为每个应用单独配置设备信息
- 🔄 **实时生效**: 修改配置后仅需重启应用,无需重启系统
- 🛡️ **安全可靠**: 基于 Zygisk 框架,模块化设计
- 📝 **简单配置**: 使用 TOML 格式配置文件,易于编辑
- ⚡ **性能优化**: 仅对配置的应用生效,不影响其他应用

## 配置说明 ⚙️

详细的配置说明请参考 [配置文档](CONFIG.md)。

配置文件位于 `/data/adb/device_faker/config/config.toml`，使用 TOML 格式。修改配置后仅需重启对应应用即可生效，无需重启系统。

## 致谢 🙏

本项目在开发过程中参考了以下优秀项目：

- [zygisk-dump-dex](https://github.com/ri-char/zygisk-dump-dex) - 提供了 Rust 开发 Zygisk 模块的原型参考
- [zygisk-rs](https://github.com/Kr328/zygisk-rs) - 提供了 Zygisk 的 Rust 依赖支持
- [MiPushZygisk](https://github.com/wushidia/MiPushZygisk) - 提供了 Zygisk 机型伪装的方案参考

感谢这些项目的开发者！💖

---

**📱 让设备不为应用的机型限制所困！** 🚀

> 💝 如果这个模块对你有帮助，可以给个 Star 支持一下