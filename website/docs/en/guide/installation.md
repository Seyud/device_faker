# Installation

## Requirements

::: danger Magisk's built-in Zygisk is not supported
Device Faker does **not** support Magisk's built-in Zygisk; a third-party Zygisk implementation is required (**ZygiskNext** recommended).

Install it and reboot before installing this module. Otherwise installation aborts with:

> No Zygisk implementation found. Please install ZygiskNext (Magisk built-in Zygisk is not supported)
:::

| Item | Requirement |
|------|-------------|
| Zygisk implementation | Any implementation other than Magisk's built-in Zygisk |
| Architecture | arm64 |

## Download

Grab the latest `device_faker-(release).zip` from the
[Releases](https://github.com/Seyud/device_faker/releases/latest) page.

## Install Steps

1. Make sure a Zygisk implementation is installed (**ZygiskNext** recommended) and you have rebooted
2. Open the Modules page in Magisk / KernelSU / APatch
3. Choose "Install from storage" and select the downloaded zip
4. **Reboot your device** after installation completes

## Updating

Just install the new zip over the existing one. Your existing config file is preserved by default.

## Uninstalling

Remove the module from the Magisk / KernelSU module page and reboot.

::: warning Uninstalling deletes your config
The module ships an uninstall script that runs `rm -rf /data/adb/device_faker`,
which **deletes the config directory along with it**.
If you want to keep your config, back up
`/data/adb/device_faker/config/config.toml` first.
:::

## Next Steps

- [Basic Configuration](./configuration/index.md) — the two ways to write a config
- [WebUI](./webui.md) — configure everything graphically instead of hand-editing TOML
