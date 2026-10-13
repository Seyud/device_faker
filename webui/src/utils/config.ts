import type {
  AppConfig,
  Config,
  CustomProps,
  DeviceInfo,
  NumericSetting,
  Template,
  TemplateMeta,
} from '../types'

type UnknownRecord = Record<string, unknown>

function isRecord(value: unknown): value is UnknownRecord {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function asOptionalString(value: unknown): string | undefined {
  if (typeof value !== 'string') {
    return undefined
  }

  return value.trim().length > 0 ? value : undefined
}

function asOptionalBoolean(value: unknown): boolean | undefined {
  return typeof value === 'boolean' ? value : undefined
}

function asOptionalInteger(value: unknown): number | undefined {
  if (typeof value !== 'number' || !Number.isInteger(value) || value < 0) {
    return undefined
  }

  return value
}

/**
 * 数值型设置：整数照常归一化；TOML 手写的 `__DELETE__` 标记原样保留
 * （否则保存/同步配置时会静默丢掉该标记，见 docs/CONFIG.md「特殊标记值」）。
 */
function asOptionalNumericSetting(value: unknown): NumericSetting | undefined {
  if (value === '__DELETE__') {
    return value
  }

  return asOptionalInteger(value)
}

function normalizePackages(value: unknown): string[] | undefined {
  if (!Array.isArray(value)) {
    return undefined
  }

  const packages = value.filter(
    (item): item is string => typeof item === 'string' && item.trim().length > 0
  )
  return packages.length > 0 ? packages : undefined
}

function normalizeCustomProps(value: unknown): CustomProps | undefined {
  if (!isRecord(value)) {
    return undefined
  }

  const customProps: CustomProps = {}

  for (const [key, entryValue] of Object.entries(value)) {
    if (typeof entryValue !== 'string' || key.trim().length === 0) {
      continue
    }

    customProps[key] = entryValue
  }

  return Object.keys(customProps).length > 0 ? customProps : undefined
}

function normalizeDeviceInfoFields(source: UnknownRecord): Partial<DeviceInfo> {
  const normalized: Partial<DeviceInfo> = {}

  const manufacturer = asOptionalString(source.manufacturer)
  if (manufacturer !== undefined) normalized.manufacturer = manufacturer

  const brand = asOptionalString(source.brand)
  if (brand !== undefined) normalized.brand = brand

  const model = asOptionalString(source.model)
  if (model !== undefined) normalized.model = model

  const device = asOptionalString(source.device)
  if (device !== undefined) normalized.device = device

  const product = asOptionalString(source.product)
  if (product !== undefined) normalized.product = product

  const hardware = asOptionalString(source.hardware)
  if (hardware !== undefined) normalized.hardware = hardware

  const board = asOptionalString(source.board)
  if (board !== undefined) normalized.board = board

  const socModel = asOptionalString(source.soc_model)
  if (socModel !== undefined) normalized.soc_model = socModel

  const name = asOptionalString(source.name)
  if (name !== undefined) normalized.name = name

  const marketname = asOptionalString(source.marketname)
  if (marketname !== undefined) normalized.marketname = marketname

  const fingerprint = asOptionalString(source.fingerprint)
  if (fingerprint !== undefined) normalized.fingerprint = fingerprint

  const buildId = asOptionalString(source.build_id)
  if (buildId !== undefined) normalized.build_id = buildId

  const displayId = asOptionalString(source.display_id)
  if (displayId !== undefined) normalized.display_id = displayId

  const incremental = asOptionalString(source.incremental)
  if (incremental !== undefined) normalized.incremental = incremental

  const securityPatch = asOptionalString(source.security_patch)
  if (securityPatch !== undefined) normalized.security_patch = securityPatch

  const characteristics = asOptionalString(source.characteristics)
  if (characteristics !== undefined) normalized.characteristics = characteristics

  const androidVersion = asOptionalString(source.android_version)
  if (androidVersion !== undefined) normalized.android_version = androidVersion

  const sdkInt = asOptionalNumericSetting(source.sdk_int)
  if (sdkInt !== undefined) normalized.sdk_int = sdkInt

  const dpi = asOptionalInteger(source.dpi)
  if (dpi !== undefined && dpi >= 120 && dpi <= 640) normalized.dpi = dpi

  const customProps = normalizeCustomProps(source.custom_props)
  if (customProps !== undefined) normalized.custom_props = customProps

  const forceDenylistUnmount = asOptionalBoolean(source.force_denylist_unmount)
  if (forceDenylistUnmount !== undefined) {
    normalized.force_denylist_unmount = forceDenylistUnmount
  }

  const companionResetprop = asOptionalBoolean(source.companion_resetprop)
  if (companionResetprop !== undefined) {
    normalized.companion_resetprop = companionResetprop
  }

  return normalized
}

export function sanitizeTemplate(input: unknown): Template {
  const source = isRecord(input) ? input : {}
  const normalized: Template = {
    ...normalizeDeviceInfoFields(source),
  }

  const packages = normalizePackages(source.packages)
  if (packages !== undefined) {
    normalized.packages = packages
  }

  const meta = extractTemplateMeta(source)
  if (meta !== undefined) {
    Object.assign(normalized, meta)
  }

  return normalized
}

export function sanitizeAppConfig(input: unknown): AppConfig | null {
  const source = isRecord(input) ? input : {}
  const packageName = asOptionalString(source.package)

  if (!packageName) {
    return null
  }

  const normalized: AppConfig = {
    package: packageName,
    ...normalizeDeviceInfoFields(source),
  }

  return normalized
}

export function sanitizeConfigForSave(input: Config): Config {
  const normalized: Config = {}

  if (input.default_force_denylist_unmount === true) {
    normalized.default_force_denylist_unmount = true
  }

  if (input.debug === true) {
    normalized.debug = true
  }

  if (input.templates) {
    const templates = Object.entries(input.templates).reduce<Record<string, Template>>(
      (result, [name, template]) => {
        const templateName = name.trim()
        if (!templateName) {
          return result
        }

        const sanitizedTemplate = sanitizeTemplate(template)
        if (Object.keys(sanitizedTemplate).length > 0) {
          result[templateName] = sanitizedTemplate
        }

        return result
      },
      {}
    )

    if (Object.keys(templates).length > 0) {
      normalized.templates = templates
    }
  }

  if (input.apps) {
    const apps = input.apps
      .map((appConfig) => sanitizeAppConfig(appConfig))
      .filter((appConfig): appConfig is AppConfig => appConfig !== null)

    if (apps.length > 0) {
      normalized.apps = apps
    }
  }

  return normalized
}

export type TemplateEntry = { name: string; template: Template }

/**
 * 找出被多个模板同时声明的包名。
 *
 * 运行时（src/config.rs find_template_for_package）只取书写顺序最先的那一个模板，
 * 其余被静默丢弃——这是「明明配了 A 机型却生效了 B 机型」的成因。
 * 返回按包名字母序排列的条目，便于 UI 稳定展示。
 *
 * @param entries 模板条目，**必须按 config.toml 的书写顺序传入**（UI 顺序即运行时优先级）
 */
export function findConflictingPackages(
  entries: readonly TemplateEntry[]
): Array<{ packageName: string; templates: string[] }> {
  const claimants = new Map<string, string[]>()

  entries.forEach(({ name, template }) => {
    for (const pkg of template.packages ?? []) {
      const list = claimants.get(pkg)
      if (list) {
        // 同一模板内重复写同一个包名不算冲突
        if (!list.includes(name)) list.push(name)
      } else {
        claimants.set(pkg, [name])
      }
    }
  })

  return Array.from(claimants.entries())
    .filter(([, names]) => names.length > 1)
    .map(([packageName, templates]) => ({ packageName, templates }))
    .sort((a, b) => a.packageName.localeCompare(b.packageName))
}

/**
 * 某个包名被哪些模板声明（按传入顺序）。长度 > 1 即为配置歧义。
 */
export function templatesClaimingPackage(
  entries: readonly TemplateEntry[],
  packageName: string
): string[] {
  return entries
    .filter(({ template }) => (template.packages ?? []).includes(packageName))
    .map(({ name }) => name)
}

export function mergeTemplateWithExisting(
  existing: Template | undefined,
  next: Template
): Template {
  const merged = {
    ...(existing || {}),
    ...next,
  } as Template

  if (next.custom_props === undefined && existing?.custom_props !== undefined) {
    merged.custom_props = existing.custom_props
  }

  return merged
}

export function mergeAppConfigWithExisting(
  existing: AppConfig | undefined,
  next: AppConfig
): AppConfig {
  const merged = {
    ...(existing || {}),
    ...next,
  } as AppConfig

  if (next.custom_props === undefined && existing?.custom_props !== undefined) {
    merged.custom_props = existing.custom_props
  }

  return merged
}

export function extractTemplateMeta(input: unknown): TemplateMeta | undefined {
  const source = isRecord(input) ? input : {}
  const meta: TemplateMeta = {}

  const version = asOptionalString(source.version)
  if (version !== undefined) {
    meta.version = version
  }

  const versionCode = asOptionalInteger(source.version_code)
  if (versionCode !== undefined) {
    meta.version_code = versionCode
  }

  const author = asOptionalString(source.author)
  if (author !== undefined) {
    meta.author = author
  }

  const description = asOptionalString(source.description)
  if (description !== undefined) {
    meta.description = description
  }

  return Object.keys(meta).length > 0 ? meta : undefined
}
