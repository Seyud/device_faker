export type OnlineTemplateSource = 'gitee' | 'github'
export type OnlineTemplateLoadState = 'idle' | 'loading' | 'ready' | 'error'
export type OnlineTemplateDetailsState = 'idle' | 'loading' | 'partial' | 'complete' | 'error'
export type TemplateCategory = 'common' | 'gaming' | 'transcend'
export type TemplateCategoryFilter = TemplateCategory | 'all'

export interface CustomProps {
  [key: string]: string
}

/**
 * 数值型设置（目前只有 sdk_int）也支持 TOML 的字符串标记 `__DELETE__`
 * （删除对应属性族，见 docs/CONFIG.md「特殊标记值」）。
 * WebUI 表单只输入整数；该标记为手改 TOML 的写法，此处仅保证保存/同步时不被丢弃。
 */
export type NumericSetting = number | '__DELETE__'

// 设备信息接口
export interface DeviceInfo {
  manufacturer?: string
  brand?: string
  model?: string
  device?: string
  product?: string
  hardware?: string
  board?: string
  soc_model?: string
  name?: string
  marketname?: string
  fingerprint?: string
  build_id?: string
  display_id?: string
  incremental?: string
  security_patch?: string
  characteristics?: string
  android_version?: string
  sdk_int?: NumericSetting
  dpi?: number
  custom_props?: CustomProps
  force_denylist_unmount?: boolean
  companion_resetprop?: boolean
}

// 机型模板接口
export interface Template extends DeviceInfo {
  packages?: string[]
  version?: string
  version_code?: number
  author?: string
  description?: string
}

// 应用配置接口
export interface AppConfig extends DeviceInfo {
  package: string
}

export interface TemplateMeta {
  version?: string
  version_code?: number
  author?: string
  description?: string
}

export interface OnlineTemplateIndexItem {
  id: string
  name: string
  displayName: string
  category: TemplateCategory
  brand: string | null
  path: string
  sha?: string
  source: OnlineTemplateSource
  contentUrl: string
}

export interface OnlineTemplateDetail {
  template: Template
  meta?: TemplateMeta
}

export interface OnlineTemplateDetailState {
  status: OnlineTemplateLoadState
  detail?: OnlineTemplateDetail
  error?: string | null
  updatedAt?: number
  version?: string
}

export interface OnlineTemplateRecord extends OnlineTemplateIndexItem {
  detailStatus: OnlineTemplateLoadState
  detail?: OnlineTemplateDetail
  detailError?: string | null
}

export interface OnlineTemplateProgress {
  total: number
  resolved: number
  succeeded: number
  failed: number
}

export interface OnlineTemplateLoadSession {
  id: number
  preferredSource: OnlineTemplateSource
  resolvedSource?: OnlineTemplateSource
  startedAt: number
}

export interface OnlineTemplateCacheEntry<T> {
  schemaVersion: number
  createdAt: number
  expiresAt: number
  data: T
  version?: string
}

// 配置文件接口
export interface Config {
  default_force_denylist_unmount?: boolean
  debug?: boolean
  templates?: Record<string, Template>
  apps?: AppConfig[]
}

// 已安装应用接口
export interface InstalledApp {
  packageName: string
  appName: string
  icon?: string
  versionName?: string
  versionCode?: number
  installed?: boolean
  isSystem?: boolean
}

// 设置接口
export interface Settings {
  theme: 'system' | 'light' | 'dark'
  language: 'system' | 'zh' | 'en' | 'tr'
  showSystemApps: boolean
  onlineTemplateSource: OnlineTemplateSource
}
