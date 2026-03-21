import { defineStore } from 'pinia'
import { computed, ref } from 'vue'
import type { InstalledApp } from '../types'
import { getAppsInfo, getInstalledApps } from '../utils/ksu'
import { normalizePackageName } from '../utils/package'

interface LoadInstalledAppsOptions {
  includeSystem?: boolean
  resolvePackages?: string[]
}

export const useAppsStore = defineStore('apps', () => {
  const installedApps = ref<InstalledApp[]>([])
  const resolvedPackageInfo = ref<Record<string, InstalledApp>>({})
  const pendingRequests = ref(0)
  const loading = computed(() => pendingRequests.value > 0)
  const error = ref<string | null>(null)
  const searchQuery = ref('')
  const hasLoadedUserApps = ref(false)
  const hasLoadedSystemApps = ref(false)
  const resolvedPackages = new Set<string>()
  const resolvingPackages = new Set<string>()

  let userAppsLoadPromise: Promise<void> | null = null
  let systemAppsLoadPromise: Promise<void> | null = null

  async function runWithLoading<T>(task: () => Promise<T>) {
    pendingRequests.value += 1
    try {
      return await task()
    } finally {
      pendingRequests.value = Math.max(0, pendingRequests.value - 1)
    }
  }

  function mergeInstalledApp(
    existing: InstalledApp | undefined,
    incoming: InstalledApp
  ): InstalledApp {
    return {
      packageName: incoming.packageName,
      appName: incoming.appName || existing?.appName || incoming.packageName,
      icon: incoming.icon || existing?.icon || '',
      versionName: incoming.versionName || existing?.versionName || '',
      versionCode: incoming.versionCode ?? existing?.versionCode ?? 0,
      installed: incoming.installed ?? existing?.installed,
      isSystem: incoming.isSystem ?? existing?.isSystem,
    }
  }

  function upsertInstalledApps(apps: InstalledApp[]) {
    if (apps.length === 0) return

    const appMap = new Map(installedApps.value.map((app) => [app.packageName, app]))
    for (const app of apps) {
      appMap.set(app.packageName, mergeInstalledApp(appMap.get(app.packageName), app))
      resolvedPackages.add(normalizePackageName(app.packageName))
    }

    installedApps.value = Array.from(appMap.values())
  }

  function upsertResolvedPackageInfo(apps: InstalledApp[]) {
    if (apps.length === 0) return

    const nextInfo = { ...resolvedPackageInfo.value }
    for (const app of apps) {
      nextInfo[app.packageName] = mergeInstalledApp(nextInfo[app.packageName], app)
      resolvedPackages.add(normalizePackageName(app.packageName))
    }

    resolvedPackageInfo.value = nextInfo
  }

  async function resolvePackagesInfo(packageNames: string[]) {
    const unresolved = Array.from(
      new Set(
        packageNames
          .map((pkg) => pkg.trim())
          .filter(Boolean)
          .filter((pkg) => {
            const normalized = normalizePackageName(pkg)
            return !resolvedPackages.has(normalized) && !resolvingPackages.has(normalized)
          })
      )
    )

    if (unresolved.length === 0) {
      return
    }

    const normalizedPackages = unresolved.map((pkg) => normalizePackageName(pkg))
    normalizedPackages.forEach((pkg) => resolvingPackages.add(pkg))

    await runWithLoading(async () => {
      const apps = await getAppsInfo(unresolved)
      normalizedPackages.forEach((pkg) => resolvedPackages.add(pkg))
      upsertResolvedPackageInfo(apps)
    }).finally(() => {
      normalizedPackages.forEach((pkg) => resolvingPackages.delete(pkg))
    })
  }

  async function ensureUserAppsLoaded() {
    if (hasLoadedUserApps.value) {
      return
    }

    if (!userAppsLoadPromise) {
      userAppsLoadPromise = runWithLoading(async () => {
        error.value = null
        upsertInstalledApps(
          (await getInstalledApps({ packageType: 'user' })).map((app) => ({
            ...app,
            installed: app.installed ?? true,
            isSystem: app.isSystem ?? false,
          }))
        )
        hasLoadedUserApps.value = true
      })
        .catch((e) => {
          error.value = e instanceof Error ? e.message : String(e)
          throw e
        })
        .finally(() => {
          userAppsLoadPromise = null
        })
    }

    return userAppsLoadPromise
  }

  async function ensureSystemAppsLoaded() {
    if (hasLoadedSystemApps.value) {
      return
    }

    await ensureUserAppsLoaded()

    if (!systemAppsLoadPromise) {
      systemAppsLoadPromise = runWithLoading(async () => {
        error.value = null
        upsertInstalledApps(
          (await getInstalledApps({ packageType: 'system' })).map((app) => ({
            ...app,
            installed: app.installed ?? true,
            isSystem: app.isSystem ?? true,
          }))
        )
        hasLoadedSystemApps.value = true
      })
        .catch((e) => {
          error.value = e instanceof Error ? e.message : String(e)
          throw e
        })
        .finally(() => {
          systemAppsLoadPromise = null
        })
    }

    return systemAppsLoadPromise
  }

  // 加载已安装应用列表
  async function loadInstalledApps(options: LoadInstalledAppsOptions = {}) {
    const { includeSystem = false, resolvePackages = [] } = options

    try {
      await ensureUserAppsLoaded()
      if (includeSystem) {
        await ensureSystemAppsLoaded()
      }
      await resolvePackagesInfo(resolvePackages)
    } catch (e) {
      error.value = e instanceof Error ? e.message : String(e)
    }
  }

  // 搜索应用
  function searchApps(query: string) {
    searchQuery.value = query
  }

  // 获取过滤后的应用列表
  function getFilteredApps(): InstalledApp[] {
    if (!searchQuery.value) {
      return installedApps.value
    }
    const q = searchQuery.value.toLowerCase()
    return installedApps.value.filter(
      (app: InstalledApp) =>
        app.packageName.toLowerCase().includes(q) || app.appName.toLowerCase().includes(q)
    )
  }

  return {
    installedApps,
    resolvedPackageInfo,
    loading,
    error,
    searchQuery,
    hasLoadedUserApps,
    hasLoadedSystemApps,
    ensureUserAppsLoaded,
    ensureSystemAppsLoaded,
    loadInstalledApps,
    resolvePackagesInfo,
    searchApps,
    getFilteredApps,
  }
})
