# Advanced Usage

## How It Takes Effect

All apps go through the same unified flow (no mode selection needed):

```
① Overwrite the Build fields → ② Rewrite the system properties → ③ Spoof DPI → ④ Unload the module
```

- **Default (recommended)**: spoofing stays **inside the app's own process** — the app reads the spoofed values while other apps and `getprop` still see the real ones; the module unloads right after patching, leaving nothing resident.
- **`companion_resetprop = true`**: the spoofed values become **system-wide** (`getprop` and other apps see them too) and are restored when the app goes to the background or exits, then reapplied on return.

Enable `companion_resetprop` only when `getprop` subprocesses or other apps must see the spoofed values as well.

## Debugging and Logs

```toml
debug = true
```

When enabled, Info-level logs are output (Error-only when disabled), written to `/data/adb/device_faker/logs/device_faker.log`. Keep it disabled during normal use to avoid leaving unnecessary traces.

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
