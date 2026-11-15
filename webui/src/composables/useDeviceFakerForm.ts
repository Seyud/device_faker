import { inject, provide, ref, type InjectionKey, type Ref } from 'vue'
import type { CustomProps, Template, AppConfig } from '../types'

export const DEVICE_FAKER_FORM_KEY: InjectionKey<Ref<DeviceFakerFormData>> =
  Symbol('deviceFakerForm')

export interface CustomPropEntry {
  key: string
  value: string
}

export interface DeviceFakerFormData {
  manufacturer: string
  brand: string
  model: string
  device: string
  product: string
  hardware: string
  board: string
  soc_model: string
  gpu_renderer: string
  gpu_vendor: string
  gpu_vendor_id: string
  gpu_device_id: string
  gpu_driver_version: string
  gpu_gl_version: string
  name: string
  marketname: string
  fingerprint: string
  build_id: string
  display_id: string
  incremental: string
  security_patch: string
  android_version: string
  sdk_int: string
  dpi: string
  characteristics: string
  force_denylist_unmount: boolean | undefined
  companion_resetprop: boolean | undefined
  packages: string[]
  customProps: CustomPropEntry[]
}

function customPropsToEntries(customProps?: CustomProps): CustomPropEntry[] {
  if (!customProps) return []
  return Object.entries(customProps).map(([key, value]) => ({ key, value: value ?? '' }))
}

function entriesToCustomProps(entries: CustomPropEntry[]): CustomProps {
  const result: CustomProps = {}

  for (const entry of entries) {
    const key = entry.key.trim()
    if (!key) continue
    // 同名 key 后写的覆盖先写的
    result[key] = entry.value
  }

  // 空对象表示"表单已管理且用户清空"，避免 merge 时保留旧 custom_props；
  // 空对象在 sanitizeConfigForSave 的 normalizeCustomProps 中被丢弃，不写入 TOML
  return result
}

function createEmptyFormData(): DeviceFakerFormData {
  return {
    manufacturer: '',
    brand: '',
    model: '',
    device: '',
    product: '',
    hardware: '',
    board: '',
    soc_model: '',
    gpu_renderer: '',
    gpu_vendor: '',
    gpu_vendor_id: '',
    gpu_device_id: '',
    gpu_driver_version: '',
    gpu_gl_version: '',
    name: '',
    marketname: '',
    fingerprint: '',
    build_id: '',
    display_id: '',
    incremental: '',
    security_patch: '',
    android_version: '',
    sdk_int: '',
    dpi: '',
    characteristics: '',
    force_denylist_unmount: undefined,
    companion_resetprop: undefined,
    packages: [],
    customProps: [],
  }
}

function parseOptionalGpuIdInput(raw: string): number | string | undefined {
  const trimmed = raw.trim()
  if (!trimmed) return undefined
  if (/^0[xX][0-9a-fA-F]+$/.test(trimmed)) return trimmed
  if (/^\d+$/.test(trimmed)) return Number(trimmed)
  return undefined
}

function formatOptionalGpuIdInput(val?: number | string): string {
  if (val === undefined) return ''
  if (typeof val === 'number') return `0x${val.toString(16).toUpperCase()}`
  return val
}

function applyOptionalGpuFieldsToTarget(
  target: Partial<Template>,
  formData: DeviceFakerFormData
): void {
  const stringFields: Array<[keyof Pick<Template, 'gpu_renderer' | 'gpu_vendor' | 'gpu_gl_version'>, string]> = [
    ['gpu_renderer', formData.gpu_renderer],
    ['gpu_vendor', formData.gpu_vendor],
    ['gpu_gl_version', formData.gpu_gl_version],
  ]
  for (const [key, value] of stringFields) {
    const trimmed = value.trim()
    if (trimmed) {
      target[key] = trimmed
    } else {
      delete target[key]
    }
  }

  const idFields: Array<[keyof Pick<Template, 'gpu_vendor_id' | 'gpu_device_id' | 'gpu_driver_version'>, string]> = [
    ['gpu_vendor_id', formData.gpu_vendor_id],
    ['gpu_device_id', formData.gpu_device_id],
    ['gpu_driver_version', formData.gpu_driver_version],
  ]
  for (const [key, value] of idFields) {
    const parsed = parseOptionalGpuIdInput(value)
    if (parsed !== undefined) {
      target[key] = parsed
    } else {
      delete target[key]
    }
  }
}

export function formDataToTemplate(formData: DeviceFakerFormData, base?: Template): Template {
  const template: Template = {
    ...(base || {}),
    manufacturer: formData.manufacturer,
    brand: formData.brand,
    model: formData.model,
    device: formData.device,
    product: formData.product,
    hardware: formData.hardware,
    board: formData.board,
    soc_model: formData.soc_model,
    fingerprint: formData.fingerprint,
  }

  applyOptionalGpuFieldsToTarget(template, formData)

  if (formData.android_version) {
    template.android_version = formData.android_version
  } else {
    delete template.android_version
  }

  if (formData.build_id) {
    template.build_id = formData.build_id
  } else {
    delete template.build_id
  }

  if (formData.display_id) {
    template.display_id = formData.display_id
  } else {
    delete template.display_id
  }

  if (formData.incremental) {
    template.incremental = formData.incremental
  } else {
    delete template.incremental
  }

  if (formData.security_patch) {
    template.security_patch = formData.security_patch
  } else {
    delete template.security_patch
  }

  if (formData.sdk_int) {
    const sdkInt = Number(formData.sdk_int)
    if (!isNaN(sdkInt)) {
      template.sdk_int = sdkInt
    } else {
      delete template.sdk_int
    }
  } else {
    delete template.sdk_int
  }

  if (formData.dpi) {
    const dpi = Number(formData.dpi)
    if (Number.isInteger(dpi) && dpi >= 120 && dpi <= 640) {
      template.dpi = dpi
    } else {
      delete template.dpi
    }
  } else {
    delete template.dpi
  }

  if (formData.name) {
    template.name = formData.name
  } else {
    delete template.name
  }

  if (formData.marketname) {
    template.marketname = formData.marketname
  } else {
    delete template.marketname
  }

  if (formData.characteristics) {
    template.characteristics = formData.characteristics
  } else {
    delete template.characteristics
  }

  if (formData.force_denylist_unmount !== undefined) {
    template.force_denylist_unmount = formData.force_denylist_unmount
  } else {
    delete template.force_denylist_unmount
  }

  if (formData.companion_resetprop !== undefined) {
    template.companion_resetprop = formData.companion_resetprop
  } else {
    delete template.companion_resetprop
  }

  if (formData.packages.length > 0) {
    template.packages = formData.packages
  } else {
    delete template.packages
  }

  template.custom_props = entriesToCustomProps(formData.customProps)

  return template
}

export function templateToFormData(template: Template): DeviceFakerFormData {
  return {
    manufacturer: template.manufacturer || '',
    brand: template.brand || '',
    model: template.model || '',
    device: template.device || '',
    product: template.product || '',
    hardware: template.hardware || '',
    board: template.board || '',
    soc_model: template.soc_model || '',
    gpu_renderer: template.gpu_renderer || '',
    gpu_vendor: template.gpu_vendor || '',
    gpu_vendor_id: formatOptionalGpuIdInput(template.gpu_vendor_id),
    gpu_device_id: formatOptionalGpuIdInput(template.gpu_device_id),
    gpu_driver_version: formatOptionalGpuIdInput(template.gpu_driver_version),
    gpu_gl_version: template.gpu_gl_version || '',
    name: template.name || '',
    marketname: template.marketname || '',
    fingerprint: template.fingerprint || '',
    build_id: template.build_id || '',
    display_id: template.display_id || '',
    incremental: template.incremental || '',
    security_patch: template.security_patch || '',
    android_version: template.android_version || '',
    sdk_int: template.sdk_int ? String(template.sdk_int) : '',
    dpi: template.dpi ? String(template.dpi) : '',
    characteristics: template.characteristics || '',
    force_denylist_unmount: template.force_denylist_unmount,
    companion_resetprop: template.companion_resetprop,
    packages: template.packages || [],
    customProps: customPropsToEntries(template.custom_props),
  }
}

export function appConfigToFormData(appConfig: AppConfig): DeviceFakerFormData {
  return {
    manufacturer: appConfig.manufacturer || '',
    brand: appConfig.brand || '',
    model: appConfig.model || '',
    device: appConfig.device || '',
    product: appConfig.product || '',
    hardware: appConfig.hardware || '',
    board: appConfig.board || '',
    soc_model: appConfig.soc_model || '',
    gpu_renderer: appConfig.gpu_renderer || '',
    gpu_vendor: appConfig.gpu_vendor || '',
    gpu_vendor_id: formatOptionalGpuIdInput(appConfig.gpu_vendor_id),
    gpu_device_id: formatOptionalGpuIdInput(appConfig.gpu_device_id),
    gpu_driver_version: formatOptionalGpuIdInput(appConfig.gpu_driver_version),
    gpu_gl_version: appConfig.gpu_gl_version || '',
    name: appConfig.name || '',
    marketname: appConfig.marketname || '',
    fingerprint: appConfig.fingerprint || '',
    build_id: appConfig.build_id || '',
    display_id: appConfig.display_id || '',
    incremental: appConfig.incremental || '',
    security_patch: appConfig.security_patch || '',
    android_version: appConfig.android_version || '',
    sdk_int: appConfig.sdk_int ? String(appConfig.sdk_int) : '',
    dpi: appConfig.dpi ? String(appConfig.dpi) : '',
    characteristics: appConfig.characteristics || '',
    force_denylist_unmount: appConfig.force_denylist_unmount,
    companion_resetprop: appConfig.companion_resetprop,
    packages: [],
    customProps: customPropsToEntries(appConfig.custom_props),
  }
}

export function formDataToAppConfig(formData: DeviceFakerFormData, packageName: string): AppConfig {
  const appConfig: AppConfig = {
    package: packageName,
    manufacturer: formData.manufacturer,
    brand: formData.brand,
    model: formData.model,
    device: formData.device,
    product: formData.product,
    hardware: formData.hardware,
    board: formData.board,
    soc_model: formData.soc_model,
    name: formData.name,
    marketname: formData.marketname,
    fingerprint: formData.fingerprint,
    build_id: formData.build_id,
    display_id: formData.display_id,
    incremental: formData.incremental,
    security_patch: formData.security_patch,
    android_version: formData.android_version,
    sdk_int: formData.sdk_int ? Number(formData.sdk_int) : undefined,
    dpi:
      formData.dpi &&
      Number.isInteger(Number(formData.dpi)) &&
      Number(formData.dpi) >= 120 &&
      Number(formData.dpi) <= 640
        ? Number(formData.dpi)
        : undefined,
    characteristics: formData.characteristics,
    force_denylist_unmount: formData.force_denylist_unmount,
    companion_resetprop: formData.companion_resetprop,
    custom_props: entriesToCustomProps(formData.customProps),
  }
  applyOptionalGpuFieldsToTarget(appConfig, formData)
  return appConfig
}

export function useDeviceFakerForm() {
  const formData = ref<DeviceFakerFormData>(createEmptyFormData())

  function resetForm() {
    formData.value = createEmptyFormData()
  }

  function fillFromTemplate(template: Template) {
    formData.value = templateToFormData(template)
  }

  function fillFromAppConfig(appConfig: AppConfig) {
    formData.value = appConfigToFormData(appConfig)
  }

  function toTemplate(base?: Template): Template {
    return formDataToTemplate(formData.value, base)
  }

  function toAppConfig(packageName: string): AppConfig {
    return formDataToAppConfig(formData.value, packageName)
  }

  return {
    formData,
    resetForm,
    fillFromTemplate,
    fillFromAppConfig,
    toTemplate,
    toAppConfig,
  }
}

export function provideDeviceFakerForm() {
  const form = useDeviceFakerForm()
  provide(DEVICE_FAKER_FORM_KEY, form.formData)
  return form
}

export function useDeviceFakerFormField() {
  const formData = inject(DEVICE_FAKER_FORM_KEY)
  if (!formData) {
    throw new Error(
      'useDeviceFakerFormField must be used within a provider of DEVICE_FAKER_FORM_KEY'
    )
  }
  return formData
}
