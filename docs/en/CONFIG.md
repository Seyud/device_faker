# Configuration Guide

## Configuration File Path
- `/data/adb/device_faker/config/config.toml`

The configuration file uses TOML format.

## Global Settings

### default_mode (Global Default Mode)

```toml
default_mode = "lite"  # Recommended: Lite mode (better stealth)
```

**Available values**:
- `"lite"` - Lite mode (Recommended) ⭐
  - Only modifies Build class static fields
  - Unloads module after completion
  - Hard to detect
  - Suitable for 90% of apps

- `"full"` - Full mode
  - Modifies Build class + Spoofs SystemProperties
  - Module stays in memory
  - May be detected
  - Use only when lite is insufficient

- `"companion"` - Companion mode
  - Uses the companion process to modify properties through resetprop
  - Supports modifying read-only properties
  - Supports custom properties and property emptying/deletion
  - Spoofs `/proc/cpuinfo` when a CPU preset or custom CPU info is configured
  - Before an app enters companion mode, `getprop` is used to back up original values; companion automatically restores them after exit or app switching

### default_force_denylist_unmount (Global Default Unmount Denylist)

```toml
# Default false: Only enable for needed apps
default_force_denylist_unmount = false
```

**Description**: Enables Zygisk's `FORCE_DENYLIST_UNMOUNT` for target apps, forcibly unmounting module mount traces. Can be enabled globally, or overridden in templates/single apps.

### debug (Debug Mode)

```toml
debug = true  # Enable verbose logging (for debugging)
# debug = false  # Or delete this line, default off (normal use)
```

**Description**:
- Enabling outputs detailed Info level logs
- Disabling only outputs Error level logs
- Recommended to disable during normal use to improve stealth

## Editing Configuration

> Multi-user Note: Supports appending `@userId` to package names to target specific users only.
>
> - `userId` corresponds to the number in path `/data/user/<userId>/...` (e.g., `0`, `999`)
> - Matching priority: Matches `com.example.app@userId` first, falls back to `com.example.app` if not found
> - This syntax applies to both `package` in `apps` and `packages` list in templates

### Method One: Device Templates

Define a `packages` list in the template to automatically apply to all package names:

```toml
# Define template and list package names
[templates.redmagic_9_pro]
packages = [
    "com.mobilelegends.mi",
  # Only effective for userId=999
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

# No need to write [[apps]], all package names automatically use this template
```

**Advantages**:
- ✅ Centralized management of devices and package names
- ✅ No need to repeat [[apps]]
- ✅ Immediately see which apps use which template

### Method Two: Direct Configuration

Use [[apps]] to specify device information for individual apps:

```toml
[[apps]]
package = "com.omarea.vtools"
manufacturer = "Xiaomi"
brand = "Xiaomi"
model = "2509FPN0BC"
device = "Xiaomi 15 Pro"
product = "popsicle"
name = "popsicle"  # Product internal name (full mode only)
mode = "full"  # Optional: Override global mode

[[apps]]
package = "com.coolapk.market"
manufacturer = "Nothing"
brand = "Nothing"
marketname = "Nothing Phone (3)"
model = "A024"
```

**Advantages**:
- ✅ Flexible configuration
- ✅ Suitable for one-time configuration or overriding templates

**Overriding Templates**:
If a package name is in a template's `packages` and also has [[apps]] configuration, [[apps]] takes priority:

```toml
[templates.redmagic_9_pro]
packages = [
    "com.mobilelegends.mi",  # Uses this template by default
]
manufacturer = "ZTE"
model = "NX769J"

[[apps]]
package = "com.mobilelegends.mi"  # Override template configuration
manufacturer = "Samsung"
model = "SM-S9280"
```

**Field Priority**:
```
[[apps]] Direct Config > Template packages list > Global default_mode
```

**Mode Priority**:
```
[[apps]].mode > [templates].mode > Global default_mode
```

### App Configuration Field Description

**Field to System Property Mapping**:
| Field | Lite Mode | Full/Companion Modes (SystemProperties) | Description |
|------|----------|------------------------------|------|
| `manufacturer` | `Build.MANUFACTURER` | + `ro.product.manufacturer` | Manufacturer (e.g., Xiaomi, Samsung) |
| `brand` | `Build.BRAND` | + `ro.product.brand` | Brand (e.g., Redmi, nubia) |
| `model` | `Build.MODEL` | + `ro.product.model` | Model Number (e.g., 25010PN30C, NX769J) |
| `device` | `Build.DEVICE` | (Build fields only) | Codename (e.g., xuanyuan, NX769J) |
| `product` | `Build.PRODUCT` | (Build fields only) | Codename (e.g., xuanyuan, NX769J) |
| `fingerprint` | `Build.FINGERPRINT` | + `ro.build.fingerprint` | Fingerprint |
| `build_id` | `Build.ID` | + `ro.build.id` etc. | Build ID (e.g., UKQ1.230917.001) |
| `name` | ❌ | `ro.product.name` + `ro.product.device` | Codename (e.g., xuanyuan) |
| `marketname` | ❌ | `ro.product.marketname` | Marketing Name (e.g., REDMI K90 Pro Max) |
| `characteristics` | ❌ | `ro.build.characteristics` | Characteristics (e.g., tablet) |
| `android_version` | `Build.VERSION.RELEASE` | + `ro.build.version.release` etc. | Android Version (e.g., 15, 14) |
| `sdk_int` | `Build.VERSION.SDK_INT` | + `ro.build.version.sdk` etc. | SDK Version (e.g., 35, 34) |
| `custom_props` | ❌ | ✅ | Custom property mapping table |
| `force_denylist_unmount` | N/A | N/A | Whether to forcibly unmount module mount points for this app; uses `default_force_denylist_unmount` if not specified |

**Android Version Spoofing Fields**:
| Field | Description | Example |
|------|------|------|
| `android_version` | Android version number, supported by all modes | `"15"`, `"14"`, `"13"` |
| `sdk_int` | SDK version number, supported by all modes | `35`, `34`, `33` |

**Custom Properties Fields**:
| Field | Description |
|------|------|
| `custom_props` | Custom property mapping table, full/companion modes only |

**Configuration Metadata Fields** (Display only, does not affect spoofing):
| Field | Description |
|------|------|
| `version` | Configuration version (e.g., "v1.0") |
| `version_code` | Configuration version code (e.g., 20251212) |
| `author` | Configuration author |
| `description` | Configuration description |

**About `force_denylist_unmount`**:
- Can be written globally (`default_force_denylist_unmount`), in templates, or in single `[[apps]]`.
- Priority: Single app > Template > Global default.
- Suitable for sensitive apps like WeChat, recommended to enable on-demand rather than globally forcing.

**Note**:
- All fields are optional except `package`
- When using template's `packages`, no need to write [[apps]] (automatic application)
- Fields in [[apps]] override template configuration
- `name` and `marketname` only take effect in **full/companion modes** (affect SystemProperties)
- `name` field spoofs both `ro.product.name` and `ro.product.device` in full/companion modes
- `characteristics` field only takes effect in **full/companion modes**
- `android_version` and `sdk_int` take effect in **all modes**
- In **lite mode**, only `manufacturer`, `brand`, `model`, `device`, `product`, `fingerprint`, `build_id`, `android_version`, `sdk_int` take effect

## Build ID Spoofing

**Properties Modified by Build ID Spoofing**:

| Mode | Build Field | System Properties |
|------|------------|-------------------|
| lite | `ID` | ❌ |
| full | `ID` | `ro.build.id`, `ro.system.build.id`, `ro.vendor.build.id`, `ro.product.build.id` |
| companion | `ID` | `ro.build.id`, `ro.system.build.id`, `ro.vendor.build.id`, `ro.product.build.id` |

## Android Version Spoofing

```toml
# Template example: Spoof as Android 15
[templates.android_15]
packages = ["com.app.needs.android15"]
manufacturer = "Google"
brand = "google"
model = "Pixel 9 Pro"
android_version = "15"
sdk_int = 35

# App example: Spoof as older Android
[[apps]]
package = "com.needs.old.android"
mode = "lite"  # Lite mode is also supported!
android_version = "13"
sdk_int = 33
```

**Properties Modified by Version Spoofing**:

| Mode | Build.VERSION Fields | System Properties |
|------|-------------------|----------|
| lite | `RELEASE`, `SDK_INT` | ❌ |
| full | `RELEASE`, `SDK_INT` | `ro.build.version.release`, `ro.build.version.sdk` etc. |
| companion | `RELEASE`, `SDK_INT` | `ro.build.version.release`, `ro.build.version.sdk` etc. |

**Complete System Property List** (full/companion modes):
- `ro.build.version.release`
- `ro.system.build.version.release`
- `ro.vendor.build.version.release`
- `ro.product.build.version.release`
- `ro.build.version.sdk`
- `ro.system.build.version.sdk`
- `ro.vendor.build.version.sdk`
- `ro.product.build.version.sdk`

## Custom Properties

**full/companion modes** both support custom properties, allowing setting any system property:

```toml
[[apps]]
package = "com.custom.app"
mode = "companion"
manufacturer = "Custom"

# Custom properties
[apps.custom_props]
"ro.custom.property" = "custom_value"
"ro.another.prop" = "another_value"
```

### Special Marker Values

Support using special marker values to perform special operations:

| Marker Value | Meaning | Example |
|--------|------|------|
| Regular string | Set to that value | `"ro.prop" = "value"` |
| `""` or omitted | Do not modify (keep original) | `brand = ""` |
| `"__EMPTY__"` | Set to empty string | `brand = "__EMPTY__"` |
| `"__DELETE__"` | Delete property | `model = "__DELETE__"` |

**Example**:

```toml
[[apps]]
package = "com.example.app"
mode = "companion"
manufacturer = "Google"
brand = "__EMPTY__"           # Set brand to empty string
model = "__DELETE__"          # Delete model property

# Custom properties also support special markers
[apps.custom_props]
"ro.custom.flag" = "enabled"
"ro.debug.mode" = "__DELETE__"
"ro.empty.value" = "__EMPTY__"
```

## Mode Comparison

| Feature | Lite Mode ⭐ | Full Mode | Companion Mode |
|------|-------------|-----------|----------------|
| Build Class Spoofing | ✅ | ✅ | ✅ |
| SystemProperties Spoofing | ❌ | ✅ | ✅ |
| Characteristics Spoofing | ❌ | ✅ | ✅ |
| Read-only Property Modification | ❌ | ❌ | ✅ |
| Custom Properties | ❌ | ✅ | ✅ |
| Property Emptying/Deletion | ❌ | ✅ | ✅ |
| Android Version Spoofing | ✅ | ✅ | ✅ |
| SDK Version Spoofing | ✅ | ✅ | ✅ |
| CPU Info Spoofing | ❌ | ❌ | ✅ |
| Module Unloadable | ✅ | ❌ | ✅ |
| Stealth | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ |
| Detection Risk | Very Low | Lower | Lower |
| Recommendation | ⭐⭐⭐⭐⭐ | ⭐⭐⭐⭐ | ⭐⭐⭐⭐ |

## How to Choose a Mode?

### Decision Basis

**Use lite mode**:
- ✅ Most apps
- ✅ Pursuing stealthiness
- ✅ Don't want to be detected

**Use full mode**:
- App reads SystemProperties
- Can still detect real device in lite mode
- Need to spoof characteristics (e.g., QQ tablet mode)
- Need custom properties

**Use companion mode**:
- Need to modify read-only properties
- Need to delete or empty certain properties
- Need complete custom property support
- Need to spoof `/proc/cpuinfo`
