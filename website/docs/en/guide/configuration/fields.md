# Field Reference

## Device Information Fields

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

::: info What are partition variants?
They are `ro.product.{odm,vendor,system,system_ext,product,bootimage,system_dlkm,vendor_dlkm,odm_dlkm}.<field>`: different vendor ROMs read device info from different partitions, so the whole family has to be written for every read path to see the same value.

Keys that **do not already exist** on the device are never written — e.g. some ROMs have no `ro.product.system_dlkm.model`, and non-OPPO devices have no `ro.vendor.oplus.market.name`. This keeps properties a real device lacks from appearing, without hurting the spoof. **Exception**: keys written explicitly in `custom_props` are always written.
:::

::: tip A fingerprint fills in the other fields
Writing only `fingerprint` is enough for a consistent spoof: the module parses it and fills in any of `brand`, `product`, `device`, `android_version`, `build_id` and `incremental` you did **not** set explicitly (values you did write are never overwritten). The derived `incremental` drives both `Build.VERSION.INCREMENTAL` and `ro.build.version.incremental`.
:::

## Android Version and Build Info Spoofing

| Field | Build field | System properties | Examples |
|-------|-------------|-------------------|----------|
| `android_version` | `Build.VERSION.RELEASE` | `ro.build.version.release`, `ro.system.build.version.release`, `ro.vendor.build.version.release`, `ro.product.build.version.release` | `"15"`, `"14"`, `"13"` |
| `sdk_int` | `Build.VERSION.SDK_INT` (integer) | `ro.build.version.sdk`, `ro.system.build.version.sdk`, `ro.vendor.build.version.sdk`, `ro.product.build.version.sdk` | `35`, `34`, `33` |
| `display_id` | `Build.DISPLAY` | `ro.build.display.id` | `PD2546_16.0.9.400(CN01)`; no partition variants |
| `incremental` | `Build.VERSION.INCREMENTAL` | `ro.build.version.incremental` + partition variants | `compiler260228234011` |
| `security_patch` | `Build.VERSION.SECURITY_PATCH` | `ro.build.version.security_patch` + partition variants | `2025-06-05` |

> `security_patch` is written together with all of its partition copies.
> `sdk_int` is numeric, but accepts the string marker `sdk_int = "__DELETE__"` to delete the whole `ro.build.version.sdk` family (risky; the WebUI has no field for it, edit the TOML by hand).

## Per-app DPI

Set `dpi` (range `120`–`640`) in a template or `[[apps]]`, for example `dpi = 420`. The current density is saved first; it is restored right after the target app goes to the background or exits, and reapplied when the app returns to the foreground. If there was nothing to restore, the density is reset.

## Custom Properties

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

## Special Marker Values

| Marker value | Meaning |
|--------------|---------|
| Normal string | Set to that value |
| `""` | Set to an empty string (equivalent to `__EMPTY__`) |
| `"__EMPTY__"` | Set to an empty string |
| `"__DELETE__"` | Delete the property (**only with `companion_resetprop = true`**) |

::: warning Marker values behave differently in structured fields
`__DELETE__` triggers deletion of the corresponding property (**also only with `companion_resetprop = true`**), while the literal `__DELETE__` string is also written into the property and the Build field; `__EMPTY__` is written literally as-is. **Use marker values only in `custom_props`**; for structured fields, leaving the value empty (`""` or omitted) means "leave unchanged".
:::

## Feature Switches

| Field | Default | Description |
|-------|---------|-------------|
| `companion_resetprop` | `false` | When `true`, the spoofed values apply **system-wide** (`getprop` and other apps see them) and are restored once the app goes to the background or exits; when `false` (default) spoofing stays inside the app's own process. See [Advanced Usage](./advanced.md#how-it-takes-effect) |
| `dpi` | — | Temporarily changes the system display density (range `120`–`640`); restored automatically once the app goes to the background or exits |
| `force_denylist_unmount` | inherits `default_force_denylist_unmount` | Force-enable Zygisk `FORCE_DENYLIST_UNMOUNT` for this app; priority: app > template > global default |

**Notes**:

- All fields except `package` are optional
- When using a template's `packages`, no `[[apps]]` entry is needed (auto-applied)
- An `[[apps]]` record replaces the template entirely; unset fields do not fall through (see [Basic Configuration](./index.md#priority))
- Fields not listed in the tables above are silently ignored (they do not affect parsing)
