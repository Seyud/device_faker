# WebUI

Device Faker provides a modern web-based management interface. Open it directly from the Magisk / KernelSU / APatch module page — no need to hand-edit TOML.

## How to Open

After installing the module, find **Device Faker** in your module manager and tap the "WebUI" or "Settings" entry.

::: info Compatibility
The WebUI is implemented using the pure KernelSU WebUI API (the old WebUI-X API has been removed). It works with the WebUI entry points in KernelSU, Magisk and APatch.
:::

## Page Overview

| Page | Main Features |
|------|---------------|
| **Apps** | View installed apps, toggle system apps, see config status, edit app config |
| **Templates** | Create/edit/delete device templates, apply to multiple packages, import/export, rename |
| **Settings** | Theme and language, config conversion tool, debug toggle |
| **Status** | Runtime overview, log summary, version info |

## App Management

### App List and Filtering

- Lists installed apps and shows whether each one is currently configured
- Toggle "Show system apps" to configure system apps too
- Search by package name or app label

### Multi-user and Cloned Apps

Multi-user instances of the same app (e.g. `com.foo` and `com.foo@999`) are deduplicated **per instance**: `com.foo` and `com.foo@0` are the same instance and no longer show up as two rows.

- A clone that only matches the main app config through the fallback shows a blue **"Inherited from main app"** badge instead of "Configured", so you never mistake it for having its own config
- The config dialog states the scope being edited: main app only (user 0) / main app + all clones (fallback) / that clone only (user N)
- Use `com.foo@0` to spoil only the main app and keep clones inheriting nothing; use the bare `com.foo` to cover the main app and all its clones

### Editing App Config

Tap an app to enter the config editor:

- Fill in fields directly (manufacturer, brand, model, device codename, etc.)
- Apply an existing [device template](./configuration/index.md#method-one-device-templates) in one click
- Set toggles such as `companion_resetprop`, `force_denylist_unmount`, `dpi`
- Add custom properties in the `custom_props` section

### Applying a Template to an App

In the app editor, choose "Apply template" to populate all fields from a template at once, then tweak as needed.

## Template Management

### Create and Edit Templates

- Fill in device fields and save as a named template
- Saved templates can be reused across multiple apps

### Apply to Multiple Packages

When editing a template, add package names to its `packages` list. Those packages will automatically use the template once saved.

::: warning One package, one template
When several templates declare the same package, only the one written **first** in `config.toml` wins at runtime. A banner at the top of the Templates page lists which template takes effect and which is ignored; deduplicate package names before merging templates.
:::

### Import / Export

- **Export**: tick the templates to export (multi-select), generate TOML text and copy it in one click
- **Import**: paste TOML text to import templates
- **Read this device**: run `getprop` and turn the current device properties into a template you can save or copy

### Online Template Library

The WebUI has a built-in online template source. Import community templates with one click:

- Repository: [device_faker_config](https://github.com/Seyud/device_faker_config)
- Browse by brand/model category
- Search by brand, model, template name or **template description**; space-separated keywords must all match

::: tip Contribute
If you have a well-organized device config, submit it to `device_faker_config` to help others.
:::

### Rename Existing Templates

Templates can be renamed. After renaming, all app configs that reference the template are automatically updated.

## Settings Page

- **Display settings**: theme (follow system / light / dark) and UI language (follow system / 简体中文 / English / Türkçe)
- **Tools → Config conversion**: pick a zip from another device-spoofing module; the WebUI parses its `system.prop` inside the WebView and converts it into a template (nothing written to disk, no CLI calls). Review the preview, then "Save as template". Handy when migrating from an older module
- **Module → Debug mode**: the `config.toml` `debug` switch; Info-level records only reach the log file when it is on

## Gestures and Navigation

- **Swipe back**: swipe from the edge to go back one level in multi-level pages (app → editor → field details)
- **Swipe between tabs**: quickly switch between bottom tabs with left/right swipes

## Multiple Languages

The WebUI supports language switching. Currently available:

- 简体中文
- English
- Türkçe

The UI language follows the system language by default, and can also be switched manually in the WebUI Settings page.

## Next Steps

- [Basic Configuration](./configuration/index.md) — understand the TOML fields behind the WebUI
- [FAQ](./faq.md) — check here first if something in the WebUI does not work
