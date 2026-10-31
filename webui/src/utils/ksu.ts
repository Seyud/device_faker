import { exec, listPackages, getPackagesInfo, pm, io } from 'kernelsu-alt'
import { normalizePackageName, parsePackageUser } from './package'
import type { InstalledApp } from '../types'

/**
 * 旧版兼容清单（TODO：确认最低支持 manager 版本后整体删除）
 *
 * 判断条件统一是 `kernelsu-alt` 在模块加载时求值出的 `pm.support` / `io.support`：
 *
 * 1. 文件 IO（`io.support`）：
 *    - `readFile` → `cat`
 *    - `writeFile` → `writeFileViaShell`（heredoc + `mv`）
 *    - `fileExists` → `test -f`
 *    - `mkdir` → `mkdir -p`
 * 2. 包枚举（`pm.support`）：`getUserIds` / `listPackagesForUser` /
 *    `getPackageListViaLegacyApi`（`pm list users` + 每用户一次 `pm list packages --user`）
 *    与 `fetchPackageInfos` 里旧 `getPackagesInfo` 分支。
 *
 * 全仓 grep `TODO(legacy-compat)` 可列出所有待删除分支。
 *
 * 与之相对，下面这些 exec 用法**不是**兼容分支、不要当成待删除项：
 * - 深链启动（`am start -d ...`，FollowDialog / TranslatorsDialog）
 * - 在线模板库的 curl/wget/busybox 兜底抓取（shell 无 CORS 限制）
 * - 「读当前设备配置」的 `getprop`（ZIP/属性→模板的转换已在
 *   `utils/propsToTemplate.ts` 里纯前端完成，不再调用 device_faker_cli）
 */

type PackageQueryType = 'user' | 'system' | 'all'

interface GetInstalledAppsOptions {
  includeSystem?: boolean
  packageType?: PackageQueryType
}

interface GetAppsInfoOptions {
  fallbackType?: PackageQueryType
  assumeInstalled?: boolean
}

/**
 * KernelSU `getPackagesInfo` / `pm.getPackagesInfo` 返回的条目。
 *
 * - `pm`（KernelSU 3.3.1+）按 uid 返回：同一包名在不同用户下各一条，带 `userId`；
 * - 旧 `getPackagesInfo` 每个包名一条，没有 `userId`（可退化为 `uid / 100000`）；
 * - 解析失败时返回 `{ packageName, error }`。
 */
interface KernelSUPackageInfo {
  packageName: string
  versionName?: string
  versionCode?: number
  appLabel?: string
  isSystem?: boolean | null
  uid?: number | null
  userId?: number | null
  error?: string
}

const getPackagesInfoBatch = getPackagesInfo as unknown as (
  pkg: string | string[]
) => Promise<KernelSUPackageInfo | KernelSUPackageInfo[]>

const getPackagesInfoViaPm = pm.getPackagesInfo as unknown as (
  pkg: string[]
) => Promise<KernelSUPackageInfo[]>

/** 归一化后的条目：包名去掉 `@userId` 后缀，并补齐所属用户 ID。 */
type PackageInfoEntry = KernelSUPackageInfo & { base: string; userId: number }

/** 包信息索引：按「实例键 base@userId」与「包名 base」两套视图查表。 */
interface PackageInfoIndex {
  /** `base@userId` → 该用户下的条目（同键保留最小 uid） */
  perInstance: Map<string, PackageInfoEntry>
  /** `base` → 跨用户的最小 uid 条目，供分身元数据继承 */
  perBase: Map<string, PackageInfoEntry>
}

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

  // TODO(legacy-compat): io 接口不可用时回退 `cat`，兼容期结束后删除该分支
  const content = io.support ? await readFileViaIo(path) : await execCommand(`cat ${path}`)
  return content.trim()
}

/** 经 io 接口整读一个文件（base64 分块读出后按 UTF-8 解码）。 */
async function readFileViaIo(path: string): Promise<string> {
  const reader = io.FileInputStream()
  const id = reader.open(path)
  if (!id) {
    throw new Error(`Failed to open ${path}`)
  }

  try {
    // 每个 read() 返回的是「该块字节自己的 base64」，各自带 `=` 填充，
    // 因此必须**逐块解码再拼字节**，不能把 base64 串直接相连（会拼出非法 base64）。
    // 逐块 atob 得到的是字节级二进制字符串，跨块的多字节 UTF-8 字符不会被打断，
    // 最后一次性按 UTF-8 解码即可。
    let binary = ''
    for (;;) {
      const chunk = reader.read(id)
      if (!chunk) break
      binary += atob(chunk)
    }
    return decodeBinaryUtf8(binary)
  } finally {
    reader.close(id)
  }
}

function decodeBinaryUtf8(binary: string): string {
  const bytes = new Uint8Array(binary.length)
  for (let i = 0; i < binary.length; i += 1) {
    bytes[i] = binary.charCodeAt(i)
  }
  return new TextDecoder().decode(bytes)
}

function encodeBase64Utf8(text: string): string {
  const bytes = new TextEncoder().encode(text)
  let binary = ''
  const CHUNK = 0x8000
  for (let i = 0; i < bytes.length; i += CHUNK) {
    binary += String.fromCharCode(...bytes.subarray(i, i + CHUNK))
  }
  return btoa(binary)
}

// 写入文件
export async function writeFile(path: string, content: string): Promise<void> {
  // TODO(legacy-compat): io 接口不可用时回退 heredoc 脚本，兼容期结束后删除该分支
  if (!io.support) {
    await writeFileViaShell(path, content)
    return
  }

  await writeFileViaIo(path, content)
}

/**
 * 经 io 接口写入文件：先写临时文件再 `renameTo`，语义对齐旧实现的
 * 「heredoc 到 tmp + `mv -f`」（不会出现半截文件）。
 */
async function writeFileViaIo(path: string, content: string): Promise<void> {
  const dir = path.slice(0, path.lastIndexOf('/'))
  if (dir) {
    io.File(dir).mkdirs()
  }

  const tempPath = `${path}.tmp.${Date.now()}`
  const writer = io.FileOutputStream()
  const id = writer.open(tempPath, false)
  if (!id) {
    throw new Error(`Failed to open ${tempPath} for writing`)
  }

  try {
    if (!writer.write(id, encodeBase64Utf8(content))) {
      throw new Error(`Failed to write ${tempPath}`)
    }
    writer.flush(id)
  } catch (err) {
    writer.close(id)
    io.File(tempPath).delete()
    throw err
  }

  writer.close(id)

  if (!io.File(tempPath).renameTo(path)) {
    io.File(tempPath).delete()
    throw new Error(`Failed to move ${tempPath} to ${path}`)
  }
}

function escapeShellPath(path: string): string {
  return path.replace(/'/g, "'\\''")
}

/** 单引号包裹 shell 参数（仅旧版兼容分支拼接命令时使用）。 */
function shellQuotePath(value: string): string {
  return `'${escapeShellPath(value)}'`
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

/** @deprecated 旧版兼容路径（TODO(legacy-compat)：io 接口普及时删除） */
async function writeFileViaShell(path: string, content: string): Promise<void> {
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
 * @deprecated 旧版兼容路径（TODO(legacy-compat)：`pm` 接口普及时删除）。
 *
 * 回退路径（user 0）：使用 kernelsu-alt 的 legacy listPackages API 获取已安装应用列表。
 */
async function getPackageListViaLegacyApi(type: PackageQueryType): Promise<string[]> {
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
 * 把接口原始条目归一化成实例条目；解析失败（带 error）的条目返回 null。
 */
function packageInfoEntryFromRaw(raw: KernelSUPackageInfo): PackageInfoEntry | null {
  if (!raw || typeof raw.packageName !== 'string' || raw.error) {
    return null
  }

  const base = normalizePackageName(raw.packageName)
  const userId =
    typeof raw.userId === 'number'
      ? raw.userId
      : typeof raw.uid === 'number'
        ? Math.floor(raw.uid / 100000)
        : 0

  return { ...raw, base, userId }
}

/**
 * 建立查表索引。
 *
 * `pm` 接口对同一包名会返回多条（每个 uid 一条，uid 升序），因此同键保留首条
 * （最小 uid），避免高分身的条目覆盖主用户条目。
 */
function buildPackageInfoIndex(entries: KernelSUPackageInfo[]): PackageInfoIndex {
  const perInstance = new Map<string, PackageInfoEntry>()
  const perBase = new Map<string, PackageInfoEntry>()

  for (const raw of entries) {
    const entry = packageInfoEntryFromRaw(raw)
    if (!entry) continue

    const instanceKey = `${entry.base}@${entry.userId}`
    if (!perInstance.has(instanceKey)) {
      perInstance.set(instanceKey, entry)
    }
    if (!perBase.has(entry.base)) {
      perBase.set(entry.base, entry)
    }
  }

  return { perInstance, perBase }
}

/**
 * 查询包信息。
 *
 * 优先走 `pm` 接口（每个 uid 一条、带 userId，可区分分身）；环境不支持时回退旧
 * `getPackagesInfo`（仅每个包名一条，分身信息只能按 base 继承）。
 * 注意两者都只认裸包名——带 `@userId` 后缀的实例键查不到任何条目。
 */
async function fetchPackageInfos(packageNames: string[]): Promise<KernelSUPackageInfo[]> {
  if (packageNames.length === 0) {
    return []
  }

  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  if (typeof (globalThis as any).ksu?.getPackagesInfo === 'undefined') {
    return []
  }

  try {
    const raw = pm.support
      ? await getPackagesInfoViaPm(packageNames)
      : // TODO(legacy-compat): 旧 getPackagesInfo（每包一条、无 userId），兼容期结束后删除
        await getPackagesInfoBatch(packageNames)
    return Array.isArray(raw) ? raw : [raw]
  } catch {
    return []
  }
}

/**
 * 把实例键（`base` 或 `base@userId`）连同已查到的元数据装成 InstalledApp。
 */
function toInstalledApp(
  packageName: string,
  index: PackageInfoIndex,
  options: GetAppsInfoOptions = {}
): InstalledApp {
  const { fallbackType, assumeInstalled = false } = options
  const { base, userId } = parsePackageUser(packageName)

  // 优先精确命中该用户实例；分身自身没有条目时回退到该包跨用户条目（元数据继承）
  const displayInfo = index.perInstance.get(`${base}@${userId}`) ?? index.perBase.get(base)

  return {
    packageName,
    appName: displayInfo?.appLabel || packageName,
    icon: '',
    versionName: displayInfo?.versionName || '',
    versionCode: displayInfo?.versionCode ?? 0,
    installed: assumeInstalled || Boolean(displayInfo),
    isSystem:
      displayInfo?.isSystem ??
      (fallbackType === 'system' ? true : fallbackType === 'user' ? false : undefined),
  }
}

// 模块级缓存：设备上的全部用户 ID
let cachedUserIds: number[] | null = null

/**
 * 枚举设备上所有用户 ID（经 pm list users），结果缓存。
 * 失败或无用户时安全降级返回 [0]。
 *
 * TODO(legacy-compat): 仅旧路径使用；`pm` 接口的 `userId` 字段已包含该信息，可删除。
 */
async function getUserIds(): Promise<number[]> {
  if (cachedUserIds !== null) {
    return cachedUserIds
  }

  try {
    const output = await execCommand('pm list users')
    const ids: number[] = []
    const regex = /UserInfo\{(\d+):/g
    let match: RegExpExecArray | null
    while ((match = regex.exec(output)) !== null) {
      ids.push(Number(match[1]))
    }
    cachedUserIds = ids.length > 0 ? ids : [0]
  } catch {
    cachedUserIds = [0]
  }

  return cachedUserIds
}

/**
 * TODO(legacy-compat): 每用户一次 shell fork 的旧实现，`pm` 接口普及时删除。
 *
 * 使用 pm list packages --user <id> 枚举指定用户的应用列表。
 * @param type 应用类型
 * @param userId 用户 ID
 * @returns 包名数组；非零用户的包名追加 @userId 后缀
 */
async function listPackagesForUser(type: PackageQueryType, userId: number): Promise<string[]> {
  const typeFlag = type === 'system' ? '-s' : type === 'user' ? '-3' : ''
  const commandParts = ['pm', 'list', 'packages']
  if (typeFlag) commandParts.push(typeFlag)
  commandParts.push('--user', String(userId))

  try {
    const output = await execCommand(commandParts.join(' '))
    const packages: string[] = []
    for (const line of output.split('\n')) {
      const trimmed = line.trim()
      if (!trimmed.startsWith('package:')) continue
      const name = trimmed.slice('package:'.length)
      if (!name) continue
      // user 0 保持原样，非零用户追加 @userId 后缀
      packages.push(userId === 0 ? name : `${name}@${userId}`)
    }
    return packages
  } catch {
    return []
  }
}

/**
 * 回退路径：枚举已安装应用（按用户展开成实例键）。
 *
 * TODO(legacy-compat): 整条 shell 回退链（getUserIds + listPackagesForUser +
 * getPackageListViaLegacyApi）待 `pm` 接口普及时删除。
 *
 * user 0 走 KernelSU 原生 listPackages，非零用户用 `pm list packages --user <id>`
 * 逐个 fork 枚举。仅当 `pm` 接口不可用（旧 manager / APatch）时才会走到这里。
 */
async function getPackageList(type: PackageQueryType): Promise<string[]> {
  const userIds = await getUserIds()

  // user 0 走 KernelSU 原生 listPackages API
  let packageList = await getPackageListViaLegacyApi(type)

  // 非零用户：用 pm list packages --user <id> 枚举
  const nonZeroUserIds = userIds.filter((id) => id !== 0)
  if (nonZeroUserIds.length > 0) {
    const results = await Promise.all(
      nonZeroUserIds.map((userId) => listPackagesForUser(type, userId))
    )
    packageList = [...packageList, ...results.flat()]
  }

  return Array.from(new Set(packageList))
}

export async function getAppsInfo(packageNames: string[], options: GetAppsInfoOptions = {}) {
  const uniquePackages = Array.from(new Set(packageNames.map((pkg) => pkg.trim()).filter(Boolean)))

  if (uniquePackages.length === 0) {
    return []
  }

  // pm / 旧 getPackagesInfo 都只认裸包名：传 base 名一次拿到该包在所有用户下的条目。
  const basePackages = Array.from(new Set(uniquePackages.map((pkg) => parsePackageUser(pkg).base)))
  const index = buildPackageInfoIndex(await fetchPackageInfos(basePackages))

  return uniquePackages.map((packageName) => toInstalledApp(packageName, index, options))
}

/**
 * `pm` 接口路径：一次 listPackages + 一次 getPackagesInfo 就能同时拿到
 * 「哪些用户装了哪些包」和逐 uid 的元数据，全程不需要 fork root shell。
 *
 * 返回 null 表示需要回退旧路径（接口不可用、返回空列表或逐 uid 条目全缺）。
 */
async function loadInstalledAppsViaPm(type: PackageQueryType): Promise<InstalledApp[] | null> {
  try {
    const names = await pm.listPackages(type)
    if (names.length === 0) {
      return null
    }

    const index = buildPackageInfoIndex(await fetchPackageInfos(names))
    if (index.perInstance.size === 0) {
      return null
    }

    // 实例键：user 0 用裸包名（与 package.ts 的实例身份约定一致），分身带 @userId 后缀
    const instances = new Set<string>()
    for (const entry of index.perInstance.values()) {
      instances.add(entry.userId === 0 ? entry.base : `${entry.base}@${entry.userId}`)
    }
    // 拿不到逐 uid 条目的包（manager 过滤掉的 special 条目等）用裸包名兜底，避免漏行
    for (const name of names) {
      const base = normalizePackageName(name)
      if (!index.perBase.has(base)) {
        instances.add(base)
      }
    }

    return Array.from(instances).map((pkg) =>
      toInstalledApp(pkg, index, { fallbackType: type, assumeInstalled: true })
    )
  } catch {
    return null
  }
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

    // 新 manager（KernelSU 3.3.1+）：pm 接口一次拿全，含分身实例与各自的元数据
    if (pm.support) {
      const viaPm = await loadInstalledAppsViaPm(packageType)
      if (viaPm && viaPm.length > 0) {
        return viaPm
      }
    }

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
  if (io.support) {
    // isFile() 与旧的 `test -f` 同义：只认普通文件（目录返回 false）
    return io.File(path).isFile()
  }

  // TODO(legacy-compat): io 接口不可用时回退 `test -f`，兼容期结束后删除该分支
  try {
    await execCommand(`test -f ${shellQuotePath(path)}`)
    return true
  } catch {
    return false
  }
}

// 创建目录
export async function mkdir(path: string): Promise<void> {
  if (io.support) {
    io.File(path).mkdirs()
    return
  }

  // TODO(legacy-compat): io 接口不可用时回退 `mkdir -p`，兼容期结束后删除该分支
  await execCommand(`mkdir -p ${shellQuotePath(path)}`)
}

export default {
  execCommand,
  readFile,
  writeFile,
  getInstalledApps,
  fileExists,
  mkdir,
}
