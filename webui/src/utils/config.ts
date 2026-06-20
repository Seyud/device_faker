import type {
  AppConfig,
  Config,
  CustomProps,
  DeviceInfo,
  SpoofMode,
  Template,
  TemplateMeta,
} from '../types'

type UnknownRecord = Record<string, unknown>

const VALID_MODES: SpoofMode[] = ['lite', 'full', 'companion']

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

function asOptionalMode(value: unknown): SpoofMode | undefined {
  return typeof value === 'string' && VALID_MODES.includes(value as SpoofMode)
    ? (value as SpoofMode)
    : undefined
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

  const name = asOptionalString(source.name)
  if (name !== undefined) normalized.name = name

  const marketname = asOptionalString(source.marketname)
  if (marketname !== undefined) normalized.marketname = marketname

  const fingerprint = asOptionalString(source.fingerprint)
  if (fingerprint !== undefined) normalized.fingerprint = fingerprint

  const buildId = asOptionalString(source.build_id)
  if (buildId !== undefined) normalized.build_id = buildId

  const characteristics = asOptionalString(source.characteristics)
  if (characteristics !== undefined) normalized.characteristics = characteristics

  const androidVersion = asOptionalString(source.android_version)
  if (androidVersion !== undefined) normalized.android_version = androidVersion

  const sdkInt = asOptionalInteger(source.sdk_int)
  if (sdkInt !== undefined) normalized.sdk_int = sdkInt

  const customProps = normalizeCustomProps(source.custom_props)
  if (customProps !== undefined) normalized.custom_props = customProps

  const forceDenylistUnmount = asOptionalBoolean(source.force_denylist_unmount)
  if (forceDenylistUnmount !== undefined) {
    normalized.force_denylist_unmount = forceDenylistUnmount
  }

  const cpuSpoof = asOptionalString(source.cpu_spoof)
  if (cpuSpoof !== undefined) normalized.cpu_spoof = cpuSpoof

  const cpuSpoofCustom = asOptionalString(source.cpu_spoof_custom)
  if (cpuSpoofCustom !== undefined) normalized.cpu_spoof_custom = cpuSpoofCustom

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

  const mode = asOptionalMode(source.mode)
  if (mode !== undefined) {
    normalized.mode = mode
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

  const mode = asOptionalMode(source.mode)
  if (mode !== undefined) {
    normalized.mode = mode
  }

  return normalized
}

export function sanitizeConfigForSave(input: Config): Config {
  const normalized: Config = {}

  if (input.default_mode && input.default_mode !== 'lite') {
    normalized.default_mode = input.default_mode
  }

  if (input.default_force_denylist_unmount === true) {
    normalized.default_force_denylist_unmount = true
  }

  if (input.debug === true) {
    normalized.debug = true
  }

  if (input.default_cpu_spoof) {
    normalized.default_cpu_spoof = input.default_cpu_spoof
  }

  if (input.cpu_presets && Object.keys(input.cpu_presets).length > 0) {
    normalized.cpu_presets = input.cpu_presets
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

  if (
    !normalized.default_mode &&
    !normalized.default_force_denylist_unmount &&
    !normalized.debug &&
    !normalized.templates &&
    !normalized.apps
  ) {
    normalized.default_mode = 'lite'
  }

  return normalized
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
