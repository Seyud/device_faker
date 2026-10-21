import type { AppConfig, Template } from '../types'

// Normalize package name by stripping trailing user suffix like "com.foo@10"
// 仅用于展示信息复用（图标/应用名）和排序，不得用于「是否已配置」判定。
export function normalizePackageName(packageName: string): string {
  const match = packageName.match(/^(.*)@\d+$/)
  return match ? match[1] : packageName
}

// Parse package name into base name and userId, e.g. "com.foo@999" -> { base: "com.foo", userId: 999 }
export function parsePackageUser(packageName: string): { base: string; userId: number } {
  const match = packageName.match(/^(.*)@(\d+)$/)
  if (!match) return { base: packageName, userId: 0 }
  return { base: match[1], userId: Number(match[2]) }
}

/**
 * 运行时会依次查找的配置键，镜像 src/lib.rs：
 *   get_merged_config("{base}@{userId}") or_else get_merged_config("{base}")
 *
 * WebUI 列表把 user 0 枚举为裸包名（ksu.ts），因此裸名对应运行时的 base@0 + base。
 * 配置项必须精确命中其中之一才算应用到该实例；不得把 @998 当成 @999。
 *
 * 注意：**裸 base 键是双语义**——既是 user 0 实例的键，也是所有非零用户（分身）的
 * 回退键。因此「解析」和「写回/删除」必须用两套不同的键集合（见 ownConfigKeys）。
 */
export function runtimeConfigKeys(packageName: string): string[] {
  const hasUserSuffix = /@\d+$/.test(packageName)
  const { base } = parsePackageUser(packageName)

  if (!hasUserSuffix) {
    return [`${base}@0`, base]
  }

  return [packageName, base]
}

/**
 * 实例身份键：user 0 的裸名与 `pkg@0` 属于**同一个实例**；非零用户按 `pkg@uid` 区分。
 * 仅用于 UI 去重与展示，不参与运行时查找（运行时仍按 runtimeConfigKeys 逐键匹配）。
 */
export function packageInstanceKey(packageName: string): string {
  const { base, userId } = parsePackageUser(packageName)
  return `${base}@${userId}`
}

/** 实例的展示包名：user 0 统一显示裸名（`pkg@0` 与裸名是同一实例，不该出现两行）。 */
export function packageDisplayName(packageName: string): string {
  const { base, userId } = parsePackageUser(packageName)
  return userId === 0 ? base : `${base}@${userId}`
}

/**
 * 配置键的作用范围（镜像运行时语义，供 UI 明示）：
 * - `user0`：`pkg@0` —— 仅 user 0（精确键优先于裸名，分身不会命中）
 * - `fallback`：裸 `pkg` —— user 0 **以及所有分身**的回退配置
 * - `clone`：`pkg@uid`（uid ≠ 0）—— 仅该分身
 */
export type ConfigKeyScope = 'user0' | 'fallback' | 'clone'

export function configKeyScope(key: string): ConfigKeyScope {
  const match = key.match(/@(\d+)$/)
  if (!match) {
    return 'fallback'
  }

  return Number(match[1]) === 0 ? 'user0' : 'clone'
}

/**
 * 该实例**自有**（可安全写/删）的配置键。
 *
 * - 非零用户 `pkg@uid`：只有 `pkg@uid`。裸 `pkg` 是主应用（user 0）的键、且是分身回退键，
 *   改它会影响主应用和所有其他分身 ⇒ 绝不允许由分身编辑去增删。
 * - user 0 实例（列表里是裸包名，也可能被写成 `pkg@0`）：`pkg@0` + 裸 `pkg`。
 *
 * 与 runtimeConfigKeys 的区别：runtimeConfigKeys 是「运行时**读取**顺序」（含回退），
 * ownConfigKeys 是「本实例**拥有**的条目」（编辑/删除只能碰这些）。
 */
export function ownConfigKeys(packageName: string): string[] {
  const hasUserSuffix = /@\d+$/.test(packageName)
  const { base, userId } = parsePackageUser(packageName)

  if (!hasUserSuffix || userId === 0) {
    return [`${base}@0`, base]
  }

  return [packageName]
}

/** key 是否属于该实例自有键（false = 继承自主应用的回退键）。 */
export function isOwnConfigKey(packageName: string, key: string): boolean {
  return ownConfigKeys(packageName).includes(key)
}

/**
 * 写回配置时应使用的键。
 *
 * 优先复用已有命中键（避免裸名与 `pkg@0` 双写），但**继承来的回退键不算命中**：
 * 分身 `pkg@999` 当前靠裸 `pkg` 回退命中时，必须写 `pkg@999`，否则会把主应用的键
 * 改掉（历史 bug：分身改模板把主应用配置清空）。
 */
export function configWriteKey(packageName: string, preferredKey?: string | null): string {
  const own = ownConfigKeys(packageName)
  if (preferredKey && own.includes(preferredKey)) {
    return preferredKey
  }

  const hasUserSuffix = /@\d+$/.test(packageName)
  const { base, userId } = parsePackageUser(packageName)

  return hasUserSuffix && userId !== 0 ? packageName : base
}

/** 从键列表里挑出属于该实例的键（按自有键优先级：`pkg@0` 先于裸 `pkg`）。 */
export function findOwnConfigKey(
  keys: readonly string[] | undefined,
  packageName: string
): string | null {
  if (!keys?.length) {
    return null
  }

  for (const key of ownConfigKeys(packageName)) {
    if (keys.includes(key)) {
      return key
    }
  }

  return null
}

/**
 * 把该实例从模板 packages 中移除：只剔除自有键，其他实例（主应用/其他分身）的键原样保留。
 */
export function filterOutOwnConfigKeys(keys: readonly string[], packageName: string): string[] {
  const own = new Set(ownConfigKeys(packageName))
  return keys.filter((key) => !own.has(key))
}

/** configPackages 中是否存在运行时会作用到 packageName 的项（仅用于「已配置」判定，不得用于写删）。 */
export function anyConfigApplies(configPackages: readonly string[], packageName: string): boolean {
  const keys = runtimeConfigKeys(packageName)
  return configPackages.some((pkg) => keys.includes(pkg))
}

/**
 * 「是否已安装」的查找键。
 * `pkg@0` 是模块配置约定；pm 对 user 0 列出的是裸包名，二者视为同一实例。
 * 其他 @userId 必须精确匹配，不能因为装了主应用就认为克隆已安装。
 */
export function installedLookupKeys(packageName: string): string[] {
  const hasUserSuffix = /@\d+$/.test(packageName)
  const { base, userId } = parsePackageUser(packageName)

  if (hasUserSuffix && userId === 0) {
    return [packageName, base]
  }

  return [packageName]
}

/** 模板条目：与 utils/config.ts 的 TemplateEntry 结构一致，避免循环依赖。 */
export interface PackageTemplateEntry {
  name: string
  template: Template
}

/** [[apps]] 条目命中的结果。inherited = 命中的是主应用的回退键（本实例未单独配置）。 */
export interface ResolvedAppConfig {
  key: string
  app: AppConfig
  inherited: boolean
}

/** 模板 packages 命中的结果。inherited 同上。 */
export interface ResolvedTemplateConfig {
  key: string
  templateName: string
  template: Template
  inherited: boolean
}

export type ResolvedPackageConfig =
  ({ kind: 'app' } & ResolvedAppConfig) | ({ kind: 'template' } & ResolvedTemplateConfig)

/**
 * 按运行时优先级解析该实例命中的 [[apps]] 条目：键优先（`pkg@uid` 先于裸 `pkg`）。
 *
 * 注意：这是**单类型**查询，会跨全部 runtime 键找 app 条目。要判断「本实例到底用哪份
 * 配置」请用 resolvePackageMatches——单独用本函数会跨键混用，与运行时不一致：
 * 分身已在 `pkg@uid` 键被模板命中时，运行时不会再回退裸 base 的 [[apps]] 条目，
 * 而本函数仍会把它当成「继承的自定义配置」。
 */
export function resolveAppConfig(
  packageName: string,
  apps: readonly AppConfig[]
): ResolvedAppConfig | null {
  for (const key of runtimeConfigKeys(packageName)) {
    const app = apps.find((candidate) => candidate.package === key)
    if (app) {
      return { key, app, inherited: !isOwnConfigKey(packageName, key) }
    }
  }

  return null
}

/**
 * 按运行时优先级解析该实例命中的模板：键优先（`pkg@uid` 先于裸 `pkg`），
 * 同一键内按模板书写顺序取第一个（镜像 src/config.rs find_template_for_package）。
 *
 * 破坏性示例（历史 bug）：模板 A 先写在 packages 里含裸 `pkg`，模板 B 后写含 `pkg@999`。
 * 对 `pkg@999` 而言 B 才是运行时真值；先遍历模板再匹配 runtimeConfigKeys 会错取 A。
 *
 * 注意：同样是**单类型**查询，业务展示请用 resolvePackageMatches（见上）。
 */
export function resolveTemplateConfig(
  packageName: string,
  entries: readonly PackageTemplateEntry[]
): ResolvedTemplateConfig | null {
  for (const key of runtimeConfigKeys(packageName)) {
    const entry = entries.find(({ template }) => template.packages?.includes(key))
    if (entry) {
      return {
        key,
        templateName: entry.name,
        template: entry.template,
        inherited: !isOwnConfigKey(packageName, key),
      }
    }
  }

  return null
}

/** 同一个（首个命中的）配置键上的两类命中；同一键内 [[apps]] 优先于模板（与运行时一致）。 */
export interface ResolvedPackageMatches {
  key: string
  appMatch: ResolvedAppConfig | null
  templateMatch: ResolvedTemplateConfig | null
}

/**
 * 逐键扫描、命中即停：返回**同一个键**上的 [[apps]] / 模板命中。
 *
 * 这是配置对话框唯一应该使用的解析入口：键优先，且不会把「更靠后键上的另一类配置」
 * 当成生效项（运行时命中第一个键就返回，永不跨键混用）。例：分身 `pkg@999` 在
 * `pkg@999` 键上被模板命中时，裸 base 上的 [[apps]] 条目对分身**不生效**。
 */
export function resolvePackageMatches(
  packageName: string,
  apps: readonly AppConfig[],
  entries: readonly PackageTemplateEntry[]
): ResolvedPackageMatches | null {
  for (const key of runtimeConfigKeys(packageName)) {
    const app = apps.find((candidate) => candidate.package === key)
    const entry = entries.find(({ template }) => template.packages?.includes(key))
    if (!app && !entry) {
      continue
    }

    const inherited = !isOwnConfigKey(packageName, key)
    return {
      key,
      appMatch: app ? { key, app, inherited } : null,
      templateMatch: entry
        ? { key, templateName: entry.name, template: entry.template, inherited }
        : null,
    }
  }

  return null
}

/**
 * 按运行时优先级解析该实例的最终配置：键优先，同一键内 [[apps]] 先于模板。
 * 即完全镜像 src/lib.rs + src/config.rs get_merged_config 的查找顺序（不含字段合并）。
 */
export function resolvePackageConfig(
  packageName: string,
  apps: readonly AppConfig[],
  entries: readonly PackageTemplateEntry[]
): ResolvedPackageConfig | null {
  const matches = resolvePackageMatches(packageName, apps, entries)
  if (!matches) {
    return null
  }

  if (matches.appMatch) {
    return { kind: 'app', ...matches.appMatch }
  }

  if (matches.templateMatch) {
    return { kind: 'template', ...matches.templateMatch }
  }

  return null
}
