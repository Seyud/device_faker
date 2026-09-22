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
 */
export function runtimeConfigKeys(packageName: string): string[] {
  const hasUserSuffix = /@\d+$/.test(packageName)
  const { base } = parsePackageUser(packageName)

  if (!hasUserSuffix) {
    return [`${base}@0`, base]
  }

  return [packageName, base]
}

/** 配置项 configPkg 在运行时是否会作用到展示包名 packageName。 */
export function configAppliesToPackage(configPkg: string, packageName: string): boolean {
  return runtimeConfigKeys(packageName).includes(configPkg)
}

/** configPackages 中是否存在运行时会作用到 packageName 的项。 */
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

/** 从 configPackages 中挑出运行时会作用到 packageName 的配置键（按 runtime 优先级）。 */
export function findApplicableConfigKeys(
  configPackages: readonly string[],
  packageName: string
): string[] {
  return runtimeConfigKeys(packageName).filter((key) => configPackages.includes(key))
}
