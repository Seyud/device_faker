# Basic Configuration

Configure a fake device identity and system properties per app. The config file uses TOML format.

## Config File Path

```
/data/adb/device_faker/config/config.toml
```

The config is **hot-reloaded**: the module re-reads the file on every app launch. After editing, just restart the target app — **no system reboot required**.

- If the config file is missing or fails to parse, the app skips spoofing and the module unloads; other apps are unaffected
- Apps not present in any config are not spoofed at all

## Global Settings

```toml
debug = false                        # debug logging (off by default)
default_force_denylist_unmount = false
```

- `debug`: when enabled, Info-level logs are output (Error-only when disabled), written to `/data/adb/device_faker/logs/device_faker.log`; keep it disabled during normal use to avoid leaving unnecessary traces
- `default_force_denylist_unmount`: enables Zygisk's `FORCE_DENYLIST_UNMOUNT` for target apps; can be overridden per template / per app with `force_denylist_unmount`

## Editing the Configuration

::: tip Multi-user support
Append `@userId` to a package name to target a specific user only.

- `userId` is the number in the path `/data/user/<userId>/...` (e.g. `0`, `999`)
- Match priority: `com.example.app@userId` is tried first, then falls back to `com.example.app`
- This applies to both `package` in `apps` and the `packages` list of templates
:::

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

- Centralized management of device identity and package lists
- No need to repeat `[[apps]]`
- See at a glance which apps use which template

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

## Priority

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

## Next Steps

- [Field Reference](./fields.md) — what each field controls
- [Advanced Usage](./advanced.md) — how it takes effect, a full example, debugging
