import type { Template } from '../types'
import type { ZipEntry } from './zipReader'

/**
 * 「属性文本 → 机型模板」的纯前端实现，语义对齐 `device_faker_cli` 的
 * `parse_property_text` / `build_template` / `derive_template_name`。
 *
 * 放在 WebUI 里的好处：用户选的 ZIP / `getprop` 输出完全在 WebView 内处理，
 * 不需要上传到 `/data/local/tmp`、也不需要 fork `device_faker_cli`。
 * 字段映射刻意保持与 CLI 一致（平面 key 优先级列表，不含分区变体展开），
 * 改这里之前先对照 `device_faker_cli/src/converter.rs`。
 */

const MANUFACTURER_KEYS = [
  'ro.product.manufacturer',
  'ro.product.system.manufacturer',
  'ro.product.vendor.manufacturer',
  'ro.product.odm.manufacturer',
]
const BRAND_KEYS = [
  'ro.product.brand',
  'ro.product.system.brand',
  'ro.product.vendor.brand',
  'ro.product.odm.brand',
]
const MODEL_KEYS = [
  'ro.product.model',
  'ro.product.system.model',
  'ro.product.vendor.model',
  'ro.product.odm.model',
]
const MARKETNAME_KEYS = ['ro.product.marketname', 'ro.product.vendor.marketname']
const NAME_KEYS = [
  'ro.product.name',
  'ro.product.system.name',
  'ro.product.vendor.name',
  'ro.product.odm.name',
]
const DEVICE_KEYS = [
  'ro.product.device',
  'ro.product.system.device',
  'ro.product.vendor.device',
  'ro.product.odm.device',
]
const PRODUCT_KEYS = [
  'ro.product.product',
  'ro.product.system.product',
  'ro.product.vendor.product',
  'ro.product.odm.product',
]
const FINGERPRINT_KEYS = [
  'ro.build.fingerprint',
  'ro.system.build.fingerprint',
  'ro.vendor.build.fingerprint',
  'ro.product.build.fingerprint',
]
const BUILD_ID_KEYS = [
  'ro.build.id',
  'ro.system.build.id',
  'ro.vendor.build.id',
  'ro.product.build.id',
]
const CHARACTERISTICS_KEYS = [
  'ro.build.characteristics',
  'ro.system.build.characteristics',
  'ro.vendor.build.characteristics',
  'ro.product.build.characteristics',
]
const ANDROID_VERSION_KEYS = [
  'ro.build.version.release',
  'ro.system.build.version.release',
  'ro.vendor.build.version.release',
  'ro.product.build.version.release',
]
const SDK_INT_KEYS = [
  'ro.build.version.sdk',
  'ro.system.build.version.sdk',
  'ro.vendor.build.version.sdk',
  'ro.product.build.version.sdk',
]
const HARDWARE_KEYS = ['ro.hardware']
const BOARD_KEYS = ['ro.product.board']
const SECURITY_PATCH_KEYS = [
  'ro.build.version.security_patch',
  'ro.system.build.version.security_patch',
  'ro.vendor.build.version.security_patch',
  'ro.product.build.version.security_patch',
  // vendor 分区独有的裸名副本（无 .version. 段）。
  'ro.vendor.build.security_patch',
]

export interface ConvertedTemplate {
  /** 建议的模板名（与 CLI 的 derive_template_name 一致） */
  name: string
  /** 可直接写入配置的模板对象 */
  template: Template
  /** 预览/复制用的 TOML 文本 */
  toml: string
}

/**
 * 解析 `system.prop` 风格文本，也支持 `getprop` 的 `[key]: [value]` 输出。
 * 重复 key 后者覆盖前者；空行与 `#` 注释跳过。
 */
export function parsePropertyText(content: string): Map<string, string> {
  const properties = new Map<string, string>()

  for (const line of content.split('\n')) {
    const trimmed = line.trim()
    if (!trimmed || trimmed.startsWith('#')) {
      continue
    }

    const getpropPair = parseGetpropLine(trimmed)
    if (getpropPair) {
      properties.set(getpropPair[0], getpropPair[1])
      continue
    }

    const equalsIndex = trimmed.indexOf('=')
    if (equalsIndex >= 0) {
      const key = trimmed.slice(0, equalsIndex).trim()
      if (!key) {
        continue
      }
      properties.set(key, trimmed.slice(equalsIndex + 1).trim())
    }
  }

  return properties
}

/** `[ro.foo.bar]: [value]` → `['ro.foo.bar', 'value']`，非该格式返回 null。 */
function parseGetpropLine(line: string): [string, string] | null {
  const separator = ']: ['
  const separatorIndex = line.indexOf(separator)
  if (separatorIndex < 0) {
    return null
  }

  const keyPart = line.slice(0, separatorIndex)
  const valuePart = line.slice(separatorIndex + separator.length)

  if (!keyPart.startsWith('[') || !valuePart.endsWith(']')) {
    return null
  }

  const key = keyPart.slice(1).trim()
  const value = valuePart.slice(0, -1).trim()
  if (!key) {
    return null
  }

  return [key, value]
}

function readNonEmptyProperty(properties: Map<string, string>, keys: string[]): string | undefined {
  for (const key of keys) {
    const value = properties.get(key)?.trim()
    if (value) {
      return value
    }
  }
  return undefined
}

function parseSdkInt(properties: Map<string, string>): number | undefined {
  const value = readNonEmptyProperty(properties, SDK_INT_KEYS)
  if (!value || !/^\d+$/.test(value)) {
    return undefined
  }
  const parsed = Number(value)
  // Rust 侧是 u32
  return parsed <= 0xffffffff ? parsed : undefined
}

/** 建议模板名：marketname → model → name → device → generated_template。 */
export function deriveTemplateName(properties: Map<string, string>): string {
  return (
    readNonEmptyProperty(properties, MARKETNAME_KEYS) ||
    readNonEmptyProperty(properties, MODEL_KEYS) ||
    readNonEmptyProperty(properties, NAME_KEYS) ||
    readNonEmptyProperty(properties, DEVICE_KEYS) ||
    'generated_template'
  )
}

function buildTemplate(properties: Map<string, string>): Template {
  const name =
    readNonEmptyProperty(properties, NAME_KEYS) || readNonEmptyProperty(properties, DEVICE_KEYS)
  const device = readNonEmptyProperty(properties, DEVICE_KEYS) || name
  const sdkInt = parseSdkInt(properties)

  return {
    packages: [],
    manufacturer: readNonEmptyProperty(properties, MANUFACTURER_KEYS),
    brand: readNonEmptyProperty(properties, BRAND_KEYS),
    marketname: readNonEmptyProperty(properties, MARKETNAME_KEYS),
    model: readNonEmptyProperty(properties, MODEL_KEYS),
    name,
    device,
    product: readNonEmptyProperty(properties, PRODUCT_KEYS),
    hardware: readNonEmptyProperty(properties, HARDWARE_KEYS),
    board: readNonEmptyProperty(properties, BOARD_KEYS),
    fingerprint: readNonEmptyProperty(properties, FINGERPRINT_KEYS),
    build_id: readNonEmptyProperty(properties, BUILD_ID_KEYS),
    security_patch: readNonEmptyProperty(properties, SECURITY_PATCH_KEYS),
    characteristics: readNonEmptyProperty(properties, CHARACTERISTICS_KEYS),
    android_version: readNonEmptyProperty(properties, ANDROID_VERSION_KEYS),
    ...(sdkInt === undefined ? {} : { sdk_int: sdkInt }),
  }
}

/** TOML 输出的字段顺序（对齐 CLI 的 `DeviceTemplateToml` 结构体顺序）。 */
const TOML_FIELD_ORDER = [
  'manufacturer',
  'brand',
  'marketname',
  'model',
  'name',
  'device',
  'product',
  'hardware',
  'board',
  'fingerprint',
  'build_id',
  'security_patch',
  'characteristics',
  'android_version',
  'sdk_int',
] as const

// eslint-disable-next-line no-control-regex
const CONTROL_CHARS = /[\u0000-\u001f\u007f]/

/** 基本字符串（`"..."`）转义。 */
function escapeBasicString(value: string): string {
  return (
    value
      .replace(/\\/g, '\\\\')
      .replace(/"/g, '\\"')
      .replace(/\t/g, '\\t')
      .replace(/\n/g, '\\n')
      .replace(/\f/g, '\\f')
      .replace(/\r/g, '\\r')
      // eslint-disable-next-line no-control-regex
      .replace(/[\u0000-\u001f\u007f]/g, (char) => {
        return `\\u${char.codePointAt(0)!.toString(16).padStart(4, '0')}`
      })
  )
}

/**
 * 值渲染，逐条对齐 `toml` crate（toml_edit 的 `to_string_repr`）的选择：
 * 1. 不需要转义 → 基本字符串 `"..."`
 * 2. 含 `"` 或 `\`、但不含 `'` 与控制字符 → 字面量字符串 `'...'`
 * 3. 引号混排（无控制字符）→ 多行基本字符串 `"""..."""`
 * 4. 其余（含控制字符）→ 基本字符串 + 转义
 */
function tomlString(value: string): string {
  const hasControl = CONTROL_CHARS.test(value)
  const hasDoubleQuote = value.includes('"')
  const hasBackslash = value.includes('\\')

  if (!hasControl && !hasDoubleQuote && !hasBackslash) {
    return `"${value}"`
  }
  if (!hasControl && !value.includes("'")) {
    return `'${value}'`
  }
  if (!hasControl) {
    return `"""${value.replace(/\\/g, '\\\\').replace(/"""/g, '\\"""')}"""`
  }
  return `"${escapeBasicString(value)}"`
}

/** 键渲染：裸键优先，其次字面量 `'...'`，最后基本字符串（键不支持多行写法）。 */
function tomlKey(key: string): string {
  if (/^[A-Za-z0-9_-]+$/.test(key)) {
    return key
  }
  if (!CONTROL_CHARS.test(key) && !key.includes("'")) {
    return `'${key}'`
  }
  return `"${escapeBasicString(key)}"`
}

/** 渲染与 CLI 同形的模板 TOML（仅表头注释不同）。 */
export function renderTemplateToml(name: string, template: Template): string {
  const lines = [
    '# Generated by device_faker WebUI',
    `[templates.${tomlKey(name)}]`,
    'packages = []',
  ]

  for (const field of TOML_FIELD_ORDER) {
    const value = template[field]
    if (value === undefined || value === null || value === '') {
      continue
    }
    lines.push(`${field} = ${typeof value === 'number' ? value : tomlString(String(value))}`)
  }

  return `${lines.join('\n')}\n`
}

function hasTemplatePayload(template: Template): boolean {
  return TOML_FIELD_ORDER.some((field) => {
    const value = template[field]
    return value !== undefined && value !== null && value !== ''
  })
}

/**
 * 属性文本 → 模板。没有识别出任何字段时抛错（与 CLI 的
 * `no supported device properties found in input` 对应）。
 */
export function convertPropertyTextToTemplate(content: string): ConvertedTemplate {
  const properties = parsePropertyText(content)
  const template = buildTemplate(properties)

  if (!hasTemplatePayload(template)) {
    throw new Error('no supported device properties found in input')
  }

  const name = deriveTemplateName(properties)
  return { name, template, toml: renderTemplateToml(name, template) }
}

/**
 * 从 ZIP 条目里找 `system.prop`（大小写不敏感，任意目录层级）。
 *
 * 取**归档顺序里第一个**匹配项：CLI 是解压到临时目录后 `WalkDir` 遍历，
 * 实测命中顺序等于条目写入顺序（`extra/system.prop` 先写入则先命中），
 * 所以这里按归档顺序取首个即可与 CLI 一致。
 */
export function findSystemPropEntry(entries: ZipEntry[]): ZipEntry | undefined {
  return entries.find((entry) => {
    const base = entry.name.slice(entry.name.lastIndexOf('/') + 1)
    return base.toLowerCase() === 'system.prop'
  })
}

/** ZIP 条目 → 模板（等价 `device_faker_cli convert`）。 */
export function convertZipEntriesToTemplate(entries: ZipEntry[]): ConvertedTemplate {
  const entry = findSystemPropEntry(entries)
  if (!entry) {
    throw new Error('system.prop not found in ZIP archive')
  }

  return convertPropertyTextToTemplate(new TextDecoder().decode(entry.data))
}
