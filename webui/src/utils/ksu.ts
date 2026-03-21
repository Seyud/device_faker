import { exec, listPackages, getPackagesInfo } from 'kernelsu-alt'
import { normalizePackageName } from './package'
import type { InstalledApp } from '../types'

type PackageQueryType = 'user' | 'system' | 'all'

interface GetInstalledAppsOptions {
  includeSystem?: boolean
  packageType?: PackageQueryType
}

interface GetAppsInfoOptions {
  fallbackType?: PackageQueryType
  assumeInstalled?: boolean
}

interface KernelSUPackageInfo {
  packageName: string
  versionName?: string
  versionCode?: number
  appLabel?: string
  isSystem?: boolean
}

const getPackagesInfoBatch = getPackagesInfo as unknown as (
  pkg: string | string[]
) => Promise<KernelSUPackageInfo | KernelSUPackageInfo[]>

// 执行命令
export async function execCommand(command: string): Promise<string> {
  // 开发模式下的模拟数据
  if (import.meta.env?.DEV) {
    return new Promise((resolve) => {
      setTimeout(() => resolve(''), 100)
    })
  }

  // 使用 kernelsu-alt 的 exec
  const result = await exec(command)
  if (result.errno === 0) {
    return result.stdout || ''
  } else {
    throw new Error(result.stderr || `Command failed with error code ${result.errno}`)
  }
}

// 读取文件
export async function readFile(path: string): Promise<string> {
  // 开发模式返回模拟数据
  if (import.meta.env?.DEV) {
    const { mockConfig, mockModuleProp } = await import('./mockData')
    if (path.includes('config.toml')) {
      return mockConfig
    }
    if (path.includes('module.prop')) {
      return mockModuleProp
    }
    return ''
  }

  const content = await execCommand(`cat ${path}`)
  return content.trim()
}

function escapeShellPath(path: string): string {
  return path.replace(/'/g, "'\\''")
}

function hasHeredocDelimiter(content: string, delimiter: string): boolean {
  const normalized = content.replace(/\r\n/g, '\n')
  if (normalized.includes(`\n${delimiter}\n`)) return true
  if (normalized.startsWith(`${delimiter}\n`)) return true
  if (normalized.endsWith(`\n${delimiter}`)) return true
  return false
}

function pickHeredocDelimiter(content: string): string {
  const base = 'EOF_DEVICE_FAKER'
  let delimiter = base
  let attempt = 0
  while (hasHeredocDelimiter(content, delimiter)) {
    attempt += 1
    delimiter = `${base}_${attempt}_${Math.random().toString(36).slice(2, 8)}`
  }
  return delimiter
}

// 写入文件
export async function writeFile(path: string, content: string): Promise<void> {
  const delimiter = pickHeredocDelimiter(content)
  const escapedPath = escapeShellPath(path)
  const tempPath = escapeShellPath(`${path}.tmp.${Date.now()}`)
  const script = [
    `cat << '${delimiter}' > '${tempPath}'`,
    content,
    delimiter,
    `sync '${tempPath}' || true`,
    `mv -f '${tempPath}' '${escapedPath}'`,
  ].join('\n')

  try {
    await execCommand(script)
  } catch (err) {
    await execCommand(`rm -f '${tempPath}'`).catch(() => {})
    throw err
  }
}

/**
 * 使用 WebUI-X $packageManager API 获取已安装应用列表
 */
async function getInstalledAppsViaWebUIX(): Promise<string[]> {
  if (typeof window.$packageManager === 'undefined') {
    return []
  }

  try {
    // 获取用户应用 (userId=0)
    const packagesJson = window.$packageManager.getInstalledPackages(0, 0)
    if (!packagesJson) return []

    const packages: string[] = JSON.parse(packagesJson)
    return packages
  } catch {
    return []
  }
}

/**
 * 使用 kernelsu-alt 的 listPackages API 获取已安装应用列表
 */
async function getInstalledAppsViaKernelSU(type: PackageQueryType): Promise<string[]> {
  try {
    if (type === 'all') {
      const [userPkgs, systemPkgs] = await Promise.all([
        listPackages('user').catch(() => []),
        listPackages('system').catch(() => []),
      ])
      return [...userPkgs, ...systemPkgs]
    }

    return await listPackages(type)
  } catch {
    return []
  }
}

/**
 * 使用 WebUI-X $packageManager API 获取单个应用信息
 */
function getAppInfoViaWebUIX(
  packageName: string
): { appName: string; versionName: string; versionCode: number } | null {
  if (typeof window.$packageManager === 'undefined') {
    return null
  }

  try {
    const info = window.$packageManager.getApplicationInfo(packageName, 0, 0)
    return {
      appName: info.getLabel() || packageName,
      versionName: info.getVersionName() || '',
      versionCode: info.getVersionCode() || 0,
    }
  } catch {
    return null
  }
}

/**
 * 使用 kernelsu-alt 的 getPackagesInfo API 获取应用信息
 */
async function getAppInfoViaKernelSU(packageNames: string[]): Promise<{
  exactInfo: Map<string, KernelSUPackageInfo>
  normalizedInfo: Map<string, KernelSUPackageInfo>
}> {
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  if (typeof (globalThis as any).ksu?.getPackagesInfo === 'undefined') {
    return {
      exactInfo: new Map(),
      normalizedInfo: new Map(),
    }
  }

  try {
    const info = await getPackagesInfoBatch(packageNames)
    const infos = Array.isArray(info) ? info : [info]
    const exactInfo = new Map<string, KernelSUPackageInfo>()
    const normalizedInfo = new Map<string, KernelSUPackageInfo>()

    for (const item of infos) {
      exactInfo.set(item.packageName, item)
      normalizedInfo.set(normalizePackageName(item.packageName), item)
    }

    return {
      exactInfo,
      normalizedInfo,
    }
  } catch {
    return {
      exactInfo: new Map(),
      normalizedInfo: new Map(),
    }
  }
}

async function getPackageList(type: PackageQueryType): Promise<string[]> {
  let packageList = await getInstalledAppsViaKernelSU(type)

  // WebUI-X 只能获取全量列表，作为兜底方案使用
  if (packageList.length === 0 && type === 'all' && typeof window.$packageManager !== 'undefined') {
    packageList = await getInstalledAppsViaWebUIX()
  }

  if (
    packageList.length === 0 &&
    type === 'user' &&
    typeof window.$packageManager !== 'undefined'
  ) {
    packageList = await getInstalledAppsViaWebUIX()
  }

  return Array.from(new Set(packageList))
}

export async function getAppsInfo(packageNames: string[], options: GetAppsInfoOptions = {}) {
  const uniquePackages = Array.from(new Set(packageNames.map((pkg) => pkg.trim()).filter(Boolean)))

  if (uniquePackages.length === 0) {
    return []
  }

  const { fallbackType, assumeInstalled = false } = options
  const kernelSUInfo = await getAppInfoViaKernelSU(uniquePackages)

  return uniquePackages.map<InstalledApp>((packageName) => {
    const normalizedPackage = normalizePackageName(packageName)
    const hasUserSuffix = /@\d+$/.test(packageName)
    const exactInfo = kernelSUInfo.exactInfo.get(packageName)
    const fallbackInfo =
      !exactInfo && hasUserSuffix ? kernelSUInfo.normalizedInfo.get(normalizedPackage) : undefined
    const exactWebUIXInfo = exactInfo ? null : getAppInfoViaWebUIX(packageName)
    const fallbackWebUIXInfo =
      !exactInfo && !exactWebUIXInfo && hasUserSuffix
        ? getAppInfoViaWebUIX(normalizedPackage)
        : null
    const displayInfo = exactInfo || exactWebUIXInfo || fallbackInfo || fallbackWebUIXInfo
    const appName =
      displayInfo && 'appLabel' in displayInfo
        ? displayInfo.appLabel || packageName
        : displayInfo && 'appName' in displayInfo
          ? displayInfo.appName || packageName
          : packageName

    return {
      packageName,
      appName,
      icon: '',
      versionName: displayInfo?.versionName || '',
      versionCode: displayInfo?.versionCode || 0,
      installed: assumeInstalled || Boolean(exactInfo || exactWebUIXInfo),
      isSystem:
        exactInfo?.isSystem ??
        (fallbackType === 'system' ? true : fallbackType === 'user' ? false : undefined),
    }
  })
}

// 获取已安装应用列表
export async function getInstalledApps(options: GetInstalledAppsOptions = {}) {
  // 开发模式返回模拟数据
  if (import.meta.env?.DEV) {
    const { mockInstalledApps } = await import('./mockData')
    const packageType = options.packageType || (options.includeSystem ? 'all' : 'user')
    return mockInstalledApps.filter((app) => {
      if (packageType === 'system') return app.isSystem === true
      if (packageType === 'user') return app.isSystem !== true
      return true
    })
  }

  try {
    const packageType = options.packageType || (options.includeSystem ? 'all' : 'user')
    const packageList = await getPackageList(packageType)

    // 如果仍然没有获取到应用列表，返回空列表
    if (packageList.length === 0) {
      return []
    }

    return await getAppsInfo(packageList, { fallbackType: packageType, assumeInstalled: true })
  } catch {
    return []
  }
}

export async function fileExists(path: string): Promise<boolean> {
  try {
    await execCommand(`test -f ${path}`)
    return true
  } catch {
    return false
  }
}

// 创建目录
export async function mkdir(path: string): Promise<void> {
  await execCommand(`mkdir -p ${path}`)
}

export default {
  execCommand,
  readFile,
  writeFile,
  getInstalledApps,
  fileExists,
  mkdir,
}
