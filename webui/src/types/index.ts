export type SpoofMode = 'lite' | 'full' | 'resetprop'
export type OnlineTemplateSource = 'gitee' | 'github'

export interface CustomProps {
  [key: string]: string
}

// 设备信息接口
export interface DeviceInfo {
  manufacturer?: string
  brand?: string
  model?: string
  device?: string
  product?: string
  name?: string
  marketname?: string
  fingerprint?: string
  characteristics?: string
  android_version?: string
  sdk_int?: number
  custom_props?: CustomProps
  force_denylist_unmount?: boolean
}

// 机型模板接口
export interface Template extends DeviceInfo {
  packages?: string[]
  mode?: SpoofMode
  version?: string
  version_code?: number
  author?: string
  description?: string
}

// 应用配置接口
export interface AppConfig extends DeviceInfo {
  package: string
  mode?: SpoofMode
}

export interface TemplateMeta {
  version?: string
  version_code?: number
  author?: string
  description?: string
}

// 配置文件接口
export interface Config {
  default_mode?: SpoofMode
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
  language: 'system' | 'zh' | 'en'
  showSystemApps: boolean
  onlineTemplateSource: OnlineTemplateSource
}
