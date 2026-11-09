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
> - **A bare package name (no `@userId`) has two meanings: "the config for user 0" + "the fallback config for every clone / multi-account user (non-zero)"**. So when the main app and its clones need
>   different templates, put the bare name in one template only and write `com.example.app@999` for the clone in another; **never put the same bare name in two templates** (at runtime only the
>   first one in file order wins, and the WebUI warns about duplicate package names). Conversely, editing the bare name affects every clone that relies on the fallback.
>
> Scope cheat sheet (runtime lookup order is `pkg@uid` → bare `pkg`, first hit wins):
>
> | Form | Scope |
> | --- | --- |
> | `com.example.app` | user 0 **+ all clones** (clones fall back here when they have no own config) |
> | `com.example.app@0` | **user 0 only** (the exact key wins over the bare name, so clones never match it) |
> | `com.example.app@999` | **that clone only** (user 999) |
>
> Use `@0` when only the main app should be spoofed and clones must not inherit; use the bare name when the main app and all its clones should be spoofed.

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

The following fields can be used in templates or `[[apps]]`. Each field rewrites both the `android.os.Build` field read inside the app and the matching system properties, so the two stay consistent:

| Field | Build field | System properties | Description |
|-------|-------------|-------------------|-------------|
| `manufacturer` | `Build.MANUFACTURER` | `ro.product.manufacturer` + partition variants | Manufacturer (e.g. Xiaomi, Samsung) |
| `brand` | `Build.BRAND` | `ro.product.brand` + partition variants | Brand (e.g. Redmi, nubia) |
| `marketname` | — (properties only) | `ro.product.marketname` + `ro.vendor.oplus.market.name` | Market name (e.g. Nothing Phone (3)); OnePlus/OPPO devices read the latter |
| `model` | `Build.MODEL` | `ro.product.model` + partition variants | Model number (e.g. NX769J, SM-S9280) |
| `name` | — (properties only) | `ro.product.name` + partition variants | Internal product name (e.g. popsicle) |
| `device` | `Build.DEVICE` | `ro.product.device` + partition variants | Codename (e.g. xuanyuan); falls back to `product` / `name` when unset |
| `product` | `Build.PRODUCT` | `ro.product.name` + partition variants, plus `ro.build.product` | Product codename (e.g. xuanyuan); same source as `name` — each fills in the other when unset |
| `hardware` | `Build.HARDWARE` | `ro.hardware` | Hardware name (e.g. qcom); no partition variants |
| `board` | `Build.BOARD` | `ro.product.board` | Board name (e.g. kalama); no partition variants |
| `soc_model` | `Build.SOC_MODEL` | `ro.soc.model` | SoC model (e.g. SM8550); no partition variants |
| `fingerprint` | `Build.FINGERPRINT` | `ro.build.fingerprint` | Fingerprint |
| `build_id` | `Build.ID` | `ro.build.id`, `ro.system.build.id`, `ro.vendor.build.id`, `ro.product.build.id` | Build ID (e.g. UKQ1.230917.001) |
| `characteristics` | — (properties only) | `ro.build.characteristics` | Characteristics (e.g. tablet) |

> **"Partition variants"** means `ro.product.{odm,vendor,system,system_ext,product,bootimage,system_dlkm,vendor_dlkm,odm_dlkm}.<field>`: different vendor ROMs read device info from different partitions, so the whole family has to be written for every read path to see the same value.
>
> Keys that **do not already exist** on the device are never written — e.g. some ROMs have no `ro.product.system_dlkm.model`, and non-OPPO devices have no `ro.vendor.oplus.market.name`. This keeps properties a real device lacks from appearing, without hurting the spoof. **Exception**: keys written explicitly in `custom_props` are always written.

> **A fingerprint fills in the other fields**: writing only `fingerprint` is enough for a consistent spoof — the module parses it and fills in any of `brand`, `product`, `device`, `android_version`, `build_id` and `incremental` you did **not** set explicitly (values you did write are never overwritten). The derived `incremental` drives both `Build.VERSION.INCREMENTAL` and `ro.build.version.incremental`.

### Android Version and Build Info Spoofing

| Field | Build field | System properties | Examples |
|-------|-------------|-------------------|----------|
| `android_version` | `Build.VERSION.RELEASE` | `ro.build.version.release`, `ro.system.build.version.release`, `ro.vendor.build.version.release`, `ro.product.build.version.release` | `"15"`, `"14"`, `"13"` |
| `sdk_int` | `Build.VERSION.SDK_INT` (integer) | `ro.build.version.sdk`, `ro.system.build.version.sdk`, `ro.vendor.build.version.sdk`, `ro.product.build.version.sdk` | `35`, `34`, `33` |
| `display_id` | `Build.DISPLAY` | `ro.build.display.id` | `PD2546_16.0.9.400(CN01)`; no partition variants |
| `incremental` | `Build.VERSION.INCREMENTAL` | `ro.build.version.incremental` + partition variants | `compiler260228234011` |
| `security_patch` | `Build.VERSION.SECURITY_PATCH` | `ro.build.version.security_patch` + partition variants | `2025-06-05` |

> `security_patch` is written together with all of its partition copies.

### Per-app DPI

Set `dpi` (range `120`–`640`) in a template or `[[apps]]`, for example `dpi = 420`. The current density override is saved first; it is restored right after the target app goes to the background or exits, and reapplied when the app returns to the foreground. If there was no override, it is reset with `wm density reset`.

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
| `"__DELETE__"` | Delete the property (for structured fields: the whole property family; **only actually applied when `companion_resetprop = true`**) |

> **Structured fields** (`brand`, `model`, `fingerprint`, `android_version`, …) with `__DELETE__`: the field is no longer written and the whole group of properties behind it is deleted; `__EMPTY__` writes an empty string (the Build field becomes an empty string too). A few fields also handle vendor aliases (e.g. `marketname`, `security_patch`).
>
> `sdk_int` is written as `sdk_int = "__DELETE__"` (a string marker on a numeric field; the WebUI form only accepts integers, so edit the TOML by hand) — **reads depending on it will fail afterwards, so treat it as high risk**.
>
> ⚠️ **Deletion only runs with `companion_resetprop = true`**: under the default mode the delete list is skipped (`__DELETE__` conflicts with "only this app is affected"). `__EMPTY__` still works in the default mode.

### Feature Switches

| Field | Default | Description |
|-------|---------|-------------|
| `companion_resetprop` | `false` | When `true`, the spoofed values apply **system-wide** (`getprop` and other apps see them) and are restored once the app goes to the background or exits; when `false` (default) spoofing stays inside the app's own process. Enable it only when you need the system-wide effect; see [How It Takes Effect](#how-it-takes-effect) |
| `dpi` | — | Temporarily changes the system display density (range `120`–`640`); restored automatically once the app goes to the background or exits |
| `force_denylist_unmount` | inherits `default_force_denylist_unmount` | Force-enable Zygisk `FORCE_DENYLIST_UNMOUNT` for this app; priority: app > template > global default |

**Notes**:
- All fields except `package` are optional
- When using a template's `packages`, no `[[apps]]` entry is needed (auto-applied)
- An `[[apps]]` record replaces the template entirely; unset fields do not fall through (see [Priority](#priority))
- Fields not listed in the tables above are silently ignored (they do not affect parsing)

## How It Takes Effect

All apps go through the same unified flow (no mode selection needed):

```
① Overwrite the Build fields → ② Rewrite the system properties → ③ Spoof DPI → ④ Unload the module
```

- **Default (recommended)**: spoofing stays **inside the app's own process** — the app reads the spoofed values while other apps, system services and `getprop` still see the real ones; the module unloads right after patching, leaving nothing resident.
- **`companion_resetprop = true`**: the spoofed values become **system-wide** (`getprop` and other apps see them too). They are restored as soon as the app goes to the background or exits, and reapplied when it comes back.

Enable `companion_resetprop` only when detection tooling (usually reading through a `getprop` subprocess) must also see the spoofed values — the trade-off is that the spoofed properties are globally visible while the app runs.

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
