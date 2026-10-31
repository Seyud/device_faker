import { parse as parseToml, stringify as stringifyToml } from 'smol-toml'
import { sanitizeConfigForSave, sanitizeTemplate } from './config'
import type { Template } from '../types'

const TEMPLATE_SIGNAL_KEYS = [
  'manufacturer',
  'brand',
  'model',
  'device',
  'product',
  'name',
  'marketname',
  'fingerprint',
  'build_id',
  'display_id',
  'incremental',
  'security_patch',
  'characteristics',
  'android_version',
  'sdk_int',
  'custom_props',
]

type UnknownRecord = Record<string, unknown>

function isRecord(value: unknown): value is UnknownRecord {
  return typeof value === 'object' && value !== null && !Array.isArray(value)
}

function hasTemplatePayload(value: unknown): boolean {
  if (!isRecord(value)) {
    return false
  }

  return TEMPLATE_SIGNAL_KEYS.some((key) => value[key] !== undefined)
}

function collectTemplates(source: UnknownRecord): Record<string, Template> {
  return Object.entries(source).reduce<Record<string, Template>>((result, [name, value]) => {
    const normalizedName = name.trim()
    if (!normalizedName || !hasTemplatePayload(value)) {
      return result
    }

    const template = sanitizeTemplate(value)
    if (Object.keys(template).length > 0) {
      result[normalizedName] = template
    }

    return result
  }, {})
}

export function parseTemplatesFromToml(content: string): Record<string, Template> {
  const trimmed = content.trim()
  if (!trimmed) {
    throw new Error('TOML content is empty')
  }

  let parsed: unknown
  try {
    parsed = parseToml(trimmed)
  } catch {
    throw new Error('Invalid TOML content')
  }

  if (isRecord(parsed) && isRecord(parsed.templates)) {
    const templates = collectTemplates(parsed.templates)
    if (Object.keys(templates).length > 0) {
      return templates
    }
  }

  if (hasTemplatePayload(parsed)) {
    const template = sanitizeTemplate(parsed)
    if (Object.keys(template).length > 0) {
      return { imported_template: template }
    }
  }

  if (isRecord(parsed)) {
    const templates = collectTemplates(parsed)
    if (Object.keys(templates).length > 0) {
      return templates
    }
  }

  throw new Error('No valid templates found in TOML content')
}

export function parseFirstTemplateFromToml(content: string): {
  templateData: Template
  defaultName: string
} {
  const templates = parseTemplatesFromToml(content)
  const [defaultName, templateData] = Object.entries(templates)[0] || []

  if (!defaultName || !templateData) {
    throw new Error('No valid template data found')
  }

  return {
    templateData,
    defaultName,
  }
}

export function stringifyTemplatesToToml(templates: Record<string, Template>) {
  if (Object.keys(templates).length === 0) {
    throw new Error('No templates selected')
  }

  const config = sanitizeConfigForSave({
    templates,
  })

  if (!config.templates || Object.keys(config.templates).length === 0) {
    throw new Error('No valid templates selected')
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  return stringifyToml(config as any)
}

export async function copyTextToClipboard(text: string) {
  if (
    typeof navigator !== 'undefined' &&
    navigator.clipboard?.writeText &&
    typeof window !== 'undefined' &&
    window.isSecureContext
  ) {
    try {
      await navigator.clipboard.writeText(text)
      return true
    } catch {
      // fall through to legacy copy path
    }
  }

  if (typeof document === 'undefined') {
    return false
  }

  const textarea = document.createElement('textarea')
  textarea.value = text
  textarea.setAttribute('readonly', '')
  textarea.style.position = 'fixed'
  textarea.style.top = '-9999px'
  textarea.style.left = '-9999px'
  document.body.appendChild(textarea)

  try {
    textarea.select()
    textarea.setSelectionRange(0, text.length)
    return document.execCommand('copy')
  } catch {
    return false
  } finally {
    document.body.removeChild(textarea)
  }
}
