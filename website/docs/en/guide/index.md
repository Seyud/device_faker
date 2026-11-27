# Device Faker 📱

A device model spoofing module based on Zygisk that can configure different device models for different applications.

## Features ✨

- 🎯 **Precise Control**: Configure device information individually for each application, only takes effect on configured apps
- 📁 **Template Management**: Multiple device templates, easily apply to multiple package names
- 🔄 **Real-time Effect**: After modifying configuration, just restart the application, no need to restart the system
- 🛡️ **Safe and Reliable**: Based on Zygisk framework, modular design
- 📝 **Simple Configuration**: Using TOML format configuration files, easy to edit
- 🎭 **Unified Execution Flow**: No mode selection needed, automatically schedules JNI field spoofing, COW property spoofing and companion services
- 🔒 **COW Property Engine**: process-private property-area copies for spoofing, per-process isolation, zero residency and no write-mapping fingerprint
- 🌐 **WebUI Management**: Provides graphical interface for convenient configuration management

## WebUI Features 🖥️

Device Faker provides a modern web management interface.

- 📋 **Template Management**: Create, edit and delete device templates, batch apply to multiple package names
- 📱 **Application Management**: Intuitive view of installed applications and their configuration status, supports multi-user apps display
- 🖋️ **Configuration Editing**: Graphical interface for editing application configuration, supporting template application and custom configuration
- 🌍 **Multilingual**: 简体中文, English, Türkçe

## Configuration Guide ⚙️

For detailed configuration instructions, please refer to the [Configuration Documentation](./configuration/index.md).

The configuration file is located at `/data/adb/device_faker/config/config.toml` and uses TOML format. After modifying the configuration, just restart the corresponding application to take effect, no need to restart the system.

## Template Configuration Contribution 🎁

Thanks to community contributions! You can also participate! Device Faker configuration repository:

- 📦 [device_faker_config](https://github.com/Seyud/device_faker_config) - Contribute device template configurations

Contribute device configurations to help more users get better device spoofing results!

## Acknowledgments 🙏

This project references the following excellent projects during development:

- [zygisk-dump-dex](https://github.com/ri-char/zygisk-dump-dex) - Provides prototype reference for Rust Zygisk module development
- [zygisk-api-rs](https://github.com/rmnscnce/zygisk-api-rs) - Provides Rust dependency support for Zygisk API
- [MiPushZygisk](https://github.com/wushidia/MiPushZygisk) - Provides reference for Zygisk device spoofing solutions

Thanks to the developers of these projects! 💖

---

**📱 Let devices not be limited by application model restrictions!** 🚀

> 💝 If this module helps you, please give it a ⭐ Star for support
