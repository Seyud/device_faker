# 📝 Changelog

## ⬆️[v1.3.5 → v1.4.0] - 2026-05-02

### 🆕 New Features
- 🖥️ WebUI apps page supports system app display toggle
- 🌐 WebUI online template library added GitHub source
- 🏷️ Added build_id and Build.ID camouflage support for all three modes
- 📥 WebUI template page added import/export tools and local template export
- 📤 WebUI template card added export button
- 👆 WebUI supports swipe gesture to switch between pages
- ✏️ WebUI template page supports editing existing template names

### 🐛 Bug Fixes
- 🧹 Fixed residual old values when clearing partial fields in WebUI template editor
- 🔄 Fixed state sync issue of custom property camouflage session when switching between foreground and background in resetprop mode

### ⚡ Improvements
- 🔗 Updated to adapt jni 0.22 API
- ⚡ Significantly optimized WebUI page switching performance
- ⚡ Optimized WebUI app list loading performance
- 🏗️ Refactored the entire WebUI online template library pipeline
- 📏 Used cutting-edge Web technology pretext to optimize text height measurement in online template virtual list
- 🚀 Optimized WebUI first screen page switch responsiveness
- 🎬 Added WebUI page switching animation
- 🦀 Refactored external resetprop binary call into built-in Rust library (by @Tools-cx-app)
- 💾 Improved configuration file backup format

## ⬆️[v1.3.0 → v1.3.5] - 2026-03-20

### 🆕 New Features
- 👁️ WebUI home page added follow module

### 🐛 Bug Fixes
- 🔧 Fixed WebUI scrolling issue caused by changes in Android System WebView rendering logic
- 🔧 Fixed configuration parsing failure due to old Android System WebView not supporting ES2022 standard

### ⚡ Improvements
- 🏗️ Replaced python scripts with xtask for core compilation (by @Tools-cx-app)
- 🌐 Optimized WebUI online template library loading, added wget loading method and retry mechanism
- 🎨 Optimized WebUI app configuration editing interface interaction
- ⚡ Optimized WebUI app list loading display
- 📋 Improved WebUI configuration reading, parsing, editing and saving

## ⬆️[v1.2.0 → v1.3.0] - 2026-01-22

### 🆕 New Features
- 📦 WebUI supports displaying configured but not installed package names
- 🏷️ WebUI online template library added brand classification
- 🤖 Added Android version camouflage and custom attribute features
- 🔍 Added template search function
- 👥 WebUI supports displaying and distinguishing multi-user package names

### ⚡ Improvements
- 🎨 Optimized WebUI styles and layout
- ⚡ Optimized WebUI application list loading
- 🔧 characteristics field supports full mode
- 🔒 Optimized full mode __system_property_get function interception
- 🔄 Refactored WebUI configuration conversion, supports direct selection

## ⬆️[v1.1.0 → v1.2.0] - 2025-12-17

### 🆕 New Features
- 🌐 WebUI added online template library feature
- 🛠️ Added CLI tool for configuration conversion and online template loading
- 🔧 Added resetprop mode support and characteristics property
- 🌍 WebUI added multi-language support
- 📋 Added FORCE_DENYLIST_UNMOUNT support
- 👥 Added support for multi-user environment configuration

### ⚡ Improvements
- 💾 Installation script changed backup file suffix to .bak for easier recovery
- 📱 WebUI adapted KernelSU API 2.1.1 to support KernelSU 2.1.2+ app name and icon display
- 🎨 Adapted KernelSU WebUI immersion standard
- 🌐 WebUI used fetch API instead of curl command for network requests, improving compatibility
- 🏗️ Modular architecture refactored Zygisk module core
- 🎨 Optimized WebUI display and layout
- 🏗️ Refactored WebUI template page to component-based structure
- 🔄 Migrated C++ atexit implementation to Rust implementation
- 📋 WebUI added configuration metadata support

## ⬆️[v1.0.5 → v1.1.0] - 2025-11-09

### 🆕 New Features
- 🌐 Added WebUI interface for more friendly configuration management experience
- ⚙️ Installation script added configuration selection feature, supporting personalized installation options

### ⚡ Improvements
- 🗺️ Optimized configuration field property mapping logic, improving configuration processing efficiency
- 📦 Migrated to new zygisk-api-rs library and updated dependencies, compatible with more Zygisk implementations
- 🔧 Updated Rust edition to 2024 version, utilizing latest language features
- 📚 Optimized configuration documentation structure, improving readability and usability

## ⬆️[v1.0.0 → v1.0.5] - 2025-11-04

### 🆕 New Features
- 📱 Added device template configuration for convenient application to multiple package names
- 🔄 Added lite and full dual-mode configuration to meet different usage scenarios
- 💾 Added module installation backup configuration mechanism for improved stability

### ⚡ Improvements
- 🚀 Optimized core performance and size for smoother operation
- 🧩 Optimized module unloading logic, reducing memory residency time
- 🎯 Used local_cxa_atexit_finalize_impl to enhance stealth
