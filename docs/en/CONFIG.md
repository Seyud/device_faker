# Configuration Guide

Configure a fake device identity and system properties per app. The config file uses TOML format.

## Config File Path

- `/data/adb/device_faker/config/config.toml`

The config is **hot-reloaded**: the module re-reads the file on every app launch. After editing, just restart the target app — **no system reboot required**.

- If the config file is missing or fails to parse, the app skips spoofing and the module unloads; other apps are unaffected
- Apps not present in any config are not spoofed at all

## Global Settings

```toml
debug = false                        # debug logging (off by default)
default_force_denylist_unmount = false
```

- `debug`: when enabled, Info-level logs are output (Error-only when disabled), written to `/data/adb/device_faker/logs/device_faker.log`; keep it disabled during normal use for better stealth
- `default_force_denylist_unmount`: enables Zygisk's `FORCE_DENYLIST_UNMOUNT` for the target apps; can be overridden per template / per app with `force_denylist_unmount`

## Editing the Configuration

> Multi-user note: append `@userId` to a package name to target a specific user only.
> 
> - `userId` is the number in the path `/data/user/<userId>/...` (e.g. `0`, `999`)
> - Match priority: `com.example.app@userId` is tried first, then falls back to `com.example.app`
> - This applies to both `package` in `apps` and the `packages` list of templates

### Method One: Device Templates

Define a `packages` list in a template; it is automatically applied to all listed packages:

```toml
[templates.redmagic_9_pro]
packages = [
    "com.mobilelegends.mi",
  # Only applies to userId=999
  # "com.mobilelegends.mi@999",
    "com.supercell.brawlstars",
]
manufacturer = "ZTE"
brand = "nubia"
model = "NX769J"
device = "REDMAGIC 9 Pro"
fingerprint = "nubia/NX769J/NX769J:14/UKQ1.230917.001/20240813.173312:user/release-keys"
build_id = "UKQ1.230917.001"

# No [[apps]] needed — all listed packages automatically use this template
```

**Advantages**:
- ✅ Centralized management of device identity and package lists
- ✅ No need to repeat [[apps]]
- ✅ See at a glance which apps use which template

### Method Two: Direct Configuration

Use `[[apps]]` to specify device info for a single app:

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

**Note**: `package` is the only **required** field; if missing, the whole config fails to parse.

### Priority

```
[[apps]] direct configuration > template packages list > global defaults
```

- **`[[apps]]` matches the whole record**: once a package matches an `[[apps]]` entry, that record **entirely replaces** the template — fields not set in the record do **not** fall through to the template, only global defaults apply
- If no `[[apps]]` entry matches, the template `packages` lists are searched
- Finally, global defaults are applied: `force_denylist_unmount` ← `default_force_denylist_unmount`
- Apps matching nothing: no spoofing, module unloads

**Template override example**:

```toml
[templates.redmagic_9_pro]
packages = ["com.mobilelegends.mi"]  # uses this template by default
manufacturer = "ZTE"
brand = "nubia"

[[apps]]
package = "com.mobilelegends.mi"  # on match, the whole record replaces the template
manufacturer = "Samsung"
# Fields set in the template but not here (e.g. brand) do NOT take effect
```

## Field Reference

### Device Information Fields

The following fields can be used in templates or `[[apps]]`. Each field drives both an `android.os.Build` static field (JNI overwrite, effective only inside the app process) and the corresponding system properties (readable by native APIs such as `__system_property_get`):

| Field | Build field | System properties | Description |
|-------|-------------|-------------------|-------------|
| `manufacturer` | `Build.MANUFACTURER` | `ro.product.manufacturer` + partition variants | Manufacturer (e.g. Xiaomi, Samsung) |
| `brand` | `Build.BRAND` | `ro.product.brand` + partition variants | Brand (e.g. Redmi, nubia) |
| `marketname` | — (properties only) | `ro.product.marketname` + `ro.vendor.oplus.market.name` | Market name (e.g. Nothing Phone (3)); OnePlus/OPPO devices read the latter |
| `model` | `Build.MODEL` | `ro.product.model` + partition variants | Model number (e.g. NX769J, SM-S9280) |
| `name` | — (properties only) | `ro.product.name` + partition variants | Internal product name (e.g. popsicle) |
| `device` | `Build.DEVICE` | `ro.product.device` + partition variants | Codename (e.g. xuanyuan); falls back to the value of `name` when unset |
| `product` | `Build.PRODUCT` | — (no system property) | Codename (e.g. xuanyuan); Build field only |
| `hardware` | `Build.HARDWARE` | `ro.hardware` | Hardware name (e.g. qcom); no partition variants |
| `board` | `Build.BOARD` | `ro.product.board` | Board name (e.g. kalama); no partition variants |
| `fingerprint` | `Build.FINGERPRINT` | `ro.build.fingerprint` | Fingerprint |
| `build_id` | `Build.ID` | `ro.build.id`, `ro.system.build.id`, `ro.vendor.build.id`, `ro.product.build.id` | Build ID (e.g. UKQ1.230917.001) |
| `characteristics` | — (properties only) | `ro.build.characteristics` | Characteristics (e.g. tablet) |

> **"Partition variants"** means the 6 properties `ro.product.{odm,vendor,system,system_ext,product,bootimage}.<field>`. Bionic property reads route by prefix to partition-specific property areas — OnePlus/OPPO and other devices read these variants, so writing them all keeps reads consistent across devices.

### Android Version and Build Info Spoofing

| Field | Build field | System properties | Examples |
|-------|-------------|-------------------|----------|
| `android_version` | `Build.VERSION.RELEASE` | `ro.build.version.release`, `ro.system.build.version.release`, `ro.vendor.build.version.release`, `ro.product.build.version.release` | `"15"`, `"14"`, `"13"` |
| `sdk_int` | `Build.VERSION.SDK_INT` (integer) | `ro.build.version.sdk`, `ro.system.build.version.sdk`, `ro.vendor.build.version.sdk`, `ro.product.build.version.sdk` | `35`, `34`, `33` |
| `display_id` | `Build.DISPLAY` | `ro.build.display.id` | `PD2546_16.0.9.400(CN01)`; no partition variants |
| `incremental` | `Build.VERSION.INCREMENTAL` | `ro.build.version.incremental` + partition variants | `compiler260228234011` |
| `security_patch` | `Build.VERSION.SECURITY_PATCH` | `ro.build.version.security_patch` + partition variants | `2025-06-05` |

> `security_patch` writes the whole family `ro.{system,system_ext,product,vendor,odm,bootimage,system_dlkm,vendor_dlkm,odm_dlkm}.build.version.security_patch` plus the bare vendor copy `ro.vendor.build.security_patch` (the Vendor security patch level).

### Per-app DPI

Set `dpi` (range `120`–`640`) in a template or `[[apps]]`, for example `dpi = 420`. The root companion runs `wm density 420` after saving the current density override. It restores the original value about 2 seconds after the target app goes to the background or exits, and reapplies it when the app returns to the foreground. If there was no override, it restores with `wm density reset`.

### Custom Properties

`custom_props` sets arbitrary system properties (written verbatim, no partition-variant expansion), usable in both templates and `[[apps]]`, and supports [special marker values](#special-marker-values):

```toml
[[apps]]
package = "com.custom.app"
manufacturer = "Custom"

[apps.custom_props]
"ro.custom.property" = "custom_value"
"ro.debug.mode" = "__DELETE__"    # delete this property
"ro.empty.value" = "__EMPTY__"    # set to empty string
```

### Special Marker Values

| Marker value | Meaning |
|--------------|---------|
| Normal string | Set to that value |
| `""` | Set to an empty string (equivalent to `__EMPTY__`) |
| `"__EMPTY__"` | Set to an empty string |
| `"__DELETE__"` | Delete the property (via companion resetprop) |

> ⚠️ **Marker values behave differently in structured fields (e.g. `brand`, `model`)**: `__DELETE__` triggers deletion of the corresponding property, but the literal `__DELETE__` string is also written into the property and the Build field; `__EMPTY__` is written literally as-is. **Use marker values only in `custom_props`**; for structured fields, leaving the value empty (`""` or omitted) means "leave unchanged".

### Feature Switches

| Field | Default | Description |
|-------|---------|-------------|
| `companion_resetprop` | `false` | When `true`, skips COW and sends all properties through the companion `resetprop` (writes the property area directly, bypassing property_service) for system-wide consistent reads; when `false` (default), COW takes priority — it only affects the current process's memory. See [Property Spoofing Mechanism](#property-spoofing-mechanism) |
| `dpi` | — | Temporarily sets system display density via `wm density`; range `120`–`640`; the companion saves and restores the original override |
| `force_denylist_unmount` | inherits `default_force_denylist_unmount` | Force-enable Zygisk `FORCE_DENYLIST_UNMOUNT` for this app; priority: app > template > global default |

**Notes**:
- All fields except `package` are optional
- When using a template's `packages`, no `[[apps]]` entry is needed (auto-applied)
- An `[[apps]]` record replaces the template entirely; unset fields do not fall through (see [Priority](#priority))
- Fields not listed in the tables above are silently ignored (they do not affect parsing)

## Property Spoofing Mechanism

All apps go through the same unified flow (no mode selection needed):

```
① JNI overwrite of Build static fields → ② COW or companion resetprop → ③ DPI spoofing → ④ DlClose unload
```

- **COW (default)**: **only the current process is affected**; the module calls DlClose right after writing, leaving zero resident footprint
- **Companion resetprop (`companion_resetprop = true`)**: all properties are written directly to the property area by the companion process (`skip_svc`, bypassing property_service), so **reads are consistent system-wide**; original values are restored automatically ~2 seconds after the app exits or goes to background, and re-applied on return to foreground

## Complete Configuration Example

```toml
# ── Global settings ───────────────────────────────────────
debug = false                        # debug logging (off by default)
default_force_denylist_unmount = false

# ── Device templates ──────────────────────────────────────
[templates.redmagic_9_pro]
packages = [
    "com.mobilelegends.mi",
  # Only applies to userId=999
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

# ── Direct configuration ──────────────────────────────────
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
dpi = 420                       # temporary display density (120–640)
force_denylist_unmount = true  # overrides the global default, this app only

[[apps]]
package = "com.example.detected.app"
companion_resetprop = true     # system-wide consistent properties (restored automatically on app exit)
manufacturer = "Custom"

[apps.custom_props]
"ro.custom.property" = "custom_value"
"ro.debug.mode" = "__DELETE__"
```
