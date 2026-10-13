# Field Reference

## Device Information Fields

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

::: info What are partition variants?
They are `ro.product.{odm,vendor,system,system_ext,product,bootimage,system_dlkm,vendor_dlkm,odm_dlkm}.<field>`. Bionic property reads route by prefix to partition-specific property areas — OnePlus/OPPO and other devices read these variants, so writing them all keeps reads consistent across devices.

`system_dlkm` / `vendor_dlkm` / `odm_dlkm` are **"fill only if already present"**: these dynamic kernel module partitions only export these copies on some Android 13+ ROMs, so keys that do not exist on the device are not written (otherwise they would appear as enumerable "ghost properties" in a full `getprop` dump that no real device has). The same applies to `ro.{system_dlkm,vendor_dlkm,odm_dlkm}.build.*`. **The whole marketing-name (`marketname`) family follows the same rule**: `ro.product.marketname`, its partition copies, and the OnePlus/OPPO alias `ro.vendor.oplus.market.name` are written only where they already exist (OEMs typically export just a few copies). **Exception**: keys written explicitly in `custom_props` are exempt and always written.
:::

## Android Version Spoofing

| Field | Build field | System properties | Examples |
|-------|-------------|-------------------|----------|
| `android_version` | `Build.VERSION.RELEASE` | `ro.build.version.release`, `ro.system.build.version.release`, `ro.vendor.build.version.release`, `ro.product.build.version.release` | `"15"`, `"14"`, `"13"` |
| `sdk_int` | `Build.VERSION.SDK_INT` (integer) | `ro.build.version.sdk`, `ro.system.build.version.sdk`, `ro.vendor.build.version.sdk`, `ro.product.build.version.sdk` | `35`, `34`, `33` |

## Per-app DPI

Set `dpi` (range `120`–`640`) in a template or `[[apps]]`, for example `dpi = 420`. The companion saves the current density override and runs `wm density 420`. It restores the original value about 2 seconds after the target app goes to the background or exits, and reapplies it when the app returns to the foreground. If there was no override, it restores with `wm density reset`.

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
| `"__DELETE__"` | Delete the property (via companion resetprop) |

::: warning Marker values behave differently in structured fields
`__DELETE__` triggers deletion of the corresponding property, but the literal `__DELETE__` string is also written into the property and the Build field; `__EMPTY__` is written literally as-is. **Use marker values only in `custom_props`**; for structured fields, leaving the value empty (`""` or omitted) means "leave unchanged".
:::

## Feature Switches

| Field | Default | Description |
|-------|---------|-------------|
| `companion_resetprop` | `false` | When `true`, skips COW and sends all properties through the companion `resetprop` (writes the property area directly, without going through `property_service`) for system-wide consistent reads; when `false` (default), COW takes priority and only affects the current process's memory. See [Advanced Usage](./advanced.md#property-spoofing-mechanism) |
| `dpi` | — | Temporarily sets system display density via `wm density`; range `120`–`640`; the companion saves and restores the original override |
| `force_denylist_unmount` | inherits `default_force_denylist_unmount` | Force-enable Zygisk `FORCE_DENYLIST_UNMOUNT` for this app; priority: app > template > global default |

**Notes**:

- All fields except `package` are optional
- When using a template's `packages`, no `[[apps]]` entry is needed (auto-applied)
- An `[[apps]]` record replaces the template entirely; unset fields do not fall through (see [Basic Configuration](./index.md#priority))
- Fields not listed in the tables above are silently ignored (they do not affect parsing)
