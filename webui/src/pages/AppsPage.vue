<template>
  <div
    class="apps-page"
    :class="{ 'apps-page--background-loading': backgroundLoading }"
    :aria-busy="initialPageLoading || backgroundLoading ? 'true' : 'false'"
  >
    <template v-if="initialPageLoading">
      <AppsPageSkeleton />
    </template>

    <template v-else>
      <AppFilters
        v-model:search-query="searchInput"
        v-model:filter-type="filterType"
        v-model:show-system-apps="showSystemApps"
        :total-count="visibleApps.length"
        :configured-count="configuredCount"
        :loading="false"
      />

      <AppList :apps="filteredApps" :empty-text="emptyText" :loading="false" @select="openConfig" />
    </template>

    <AppConfigDialog
      v-if="configDialogVisible"
      v-model="configDialogVisible"
      :app="currentApp"
      @saved="handleConfigSaved"
    />
  </div>
</template>

<script setup lang="ts">
import { computed, defineAsyncComponent, onMounted, onUnmounted, ref, watch } from 'vue'
import AppFilters from '../components/apps/AppFilters.vue'
import AppList from '../components/apps/AppList.vue'
import AppsPageSkeleton from '../components/apps/AppsPageSkeleton.vue'
import { useAppsStore } from '../stores/apps'
import { useConfigStore } from '../stores/config'
import { useSettingsStore } from '../stores/settings'
import { useModalHistory } from '../composables/useModalHistory'
import { useI18n } from '../utils/i18n'
import {
  installedLookupKeys,
  normalizePackageName,
  parsePackageUser,
  runtimeConfigKeys,
} from '../utils/package'
import type { InstalledApp } from '../types'

type FilterType = 'all' | 'configured'
type AppListItem = InstalledApp & {
  configured: boolean
  /** 预计算的小写包名/应用名,避免每次键入搜索词都对全量列表做 toLowerCase */
  pkgLower: string
  nameLower: string
  /**
   * 预计算的排序 key:归一化包名 \0 userId(补零) \0 已安装优先。
   * 排序退化为单次字符串比较,不再在比较器里跑正则(normalizePackageName)。
   */
  sortKey: string
}

function buildSortKey(packageName: string, installed: boolean): string {
  const { base, userId } = parsePackageUser(packageName)
  return `${base}\u0000${String(userId).padStart(8, '0')}\u0000${installed ? '0' : '1'}`
}

function buildListEntry(app: InstalledApp, configured: boolean, installed: boolean): AppListItem {
  const appName = app.appName || app.packageName
  return {
    ...app,
    appName,
    configured,
    pkgLower: app.packageName.toLowerCase(),
    nameLower: appName.toLowerCase(),
    sortKey: buildSortKey(app.packageName, installed),
  }
}

const AppConfigDialog = defineAsyncComponent(() => import('../components/apps/AppConfigDialog.vue'))

const configStore = useConfigStore()
const appsStore = useAppsStore()
const settingsStore = useSettingsStore()
const { t } = useI18n()

const searchInput = ref('')
const debouncedSearch = ref('')
let searchDebounceTimer: ReturnType<typeof setTimeout> | null = null

// 搜索防抖:每个键入字符只更新原始输入,150ms 静默期后才触发全量过滤+排序
watch(searchInput, (value) => {
  if (searchDebounceTimer !== null) {
    clearTimeout(searchDebounceTimer)
  }
  searchDebounceTimer = setTimeout(() => {
    searchDebounceTimer = null
    debouncedSearch.value = value
  }, 150)
})

onUnmounted(() => {
  if (searchDebounceTimer !== null) {
    clearTimeout(searchDebounceTimer)
    searchDebounceTimer = null
  }
})

const filterType = ref<FilterType>('all')
const configDialogVisible = ref(false)
const currentApp = ref<InstalledApp | null>(null)

useModalHistory(configDialogVisible, () => {
  configDialogVisible.value = false
})
const showSystemApps = computed({
  get: () => settingsStore.showSystemApps,
  set: (value: boolean) => settingsStore.setShowSystemApps(value),
})

// 首次进入页面时仅等待用户应用列表，附加补全任务改为后台执行
const isInitializing = ref(!appsStore.hasLoadedUserApps)
const installedApps = computed(() => appsStore.installedApps)
const resolvedPackageInfo = computed(() => appsStore.resolvedPackageInfo)

const installedPackageState = computed(() => {
  const exactPackages = new Set<string>()

  for (const app of installedApps.value) {
    if (app.installed === false) continue
    exactPackages.add(app.packageName)
  }

  return {
    exactPackages,
  }
})

const configuredPackageState = computed(() => {
  const exactPackages = new Set<string>()
  const configuredAppsMap = new Map<string, InstalledApp>()

  for (const appConfig of configStore.getApps()) {
    exactPackages.add(appConfig.package)
    configuredAppsMap.set(appConfig.package, {
      packageName: appConfig.package,
      appName: appConfig.package,
      installed: false,
    })
  }

  for (const template of Object.values(configStore.getTemplates())) {
    if (!template.packages) continue
    for (const pkg of template.packages) {
      exactPackages.add(pkg)
      if (configuredAppsMap.has(pkg)) continue

      configuredAppsMap.set(pkg, {
        packageName: pkg,
        appName: pkg,
        installed: false,
      })
    }
  }

  return {
    packages: Array.from(exactPackages),
    exactPackages,
    configuredApps: Array.from(configuredAppsMap.values()),
  }
})

function isConfiguredPackage(packageName: string) {
  // 与 Rust 一致：配置项须精确命中 runtime 查找键（base@userId 或裸 base）。
  // 禁止把 @0/@998/@999 归一化后互相认成已配置。
  return runtimeConfigKeys(packageName).some((key) =>
    configuredPackageState.value.exactPackages.has(key)
  )
}

function isInstalledPackage(packageName: string) {
  return installedLookupKeys(packageName).some((key) =>
    installedPackageState.value.exactPackages.has(key)
  )
}

function getResolvedPackageInfo(packageName: string) {
  return resolvedPackageInfo.value[packageName]
}

const allApps = computed<AppListItem[]>(() => {
  const result: AppListItem[] = []
  const packageIndex = new Map<string, number>()
  const normalizedIndex = new Map<string, number>()

  // 保留已安装应用的原始顺序
  for (const app of installedApps.value) {
    const normalized = normalizePackageName(app.packageName)
    if (packageIndex.has(app.packageName)) continue

    const entry = buildListEntry(app, isConfiguredPackage(app.packageName), app.installed ?? true)

    const idx = result.length
    result.push(entry)
    packageIndex.set(app.packageName, idx)
    if (!normalizedIndex.has(normalized)) {
      normalizedIndex.set(normalized, idx)
    }
  }

  // 合并配置项：如果包名不同（即使归一化后相同），也应显示为不同应用
  for (const app of configuredPackageState.value.configuredApps) {
    if (packageIndex.has(app.packageName)) continue

    // 查找具有相同归一化包名的已存在应用，复用其展示信息
    const normalized = normalizePackageName(app.packageName)
    const existingIdx = normalizedIndex.get(normalized)
    const existingApp = existingIdx !== undefined ? result[existingIdx] : undefined
    const resolvedInfo = getResolvedPackageInfo(app.packageName)
    const installed = isInstalledPackage(app.packageName)

    const entry: AppListItem = {
      packageName: app.packageName,
      appName: resolvedInfo?.appName || existingApp?.appName || app.packageName,
      icon: resolvedInfo?.icon || existingApp?.icon || '',
      versionName: resolvedInfo?.versionName || existingApp?.versionName || '',
      versionCode: resolvedInfo?.versionCode ?? existingApp?.versionCode ?? 0,
      installed,
      isSystem: existingApp?.isSystem ?? resolvedInfo?.isSystem ?? app.isSystem,
      configured: true,
      pkgLower: app.packageName.toLowerCase(),
      nameLower: (resolvedInfo?.appName || existingApp?.appName || app.packageName).toLowerCase(),
      sortKey: buildSortKey(app.packageName, installed),
    }

    const idx = result.length
    result.push(entry)
    packageIndex.set(app.packageName, idx)
  }

  return result
})

const visibleApps = computed(() =>
  allApps.value.filter((app) => showSystemApps.value || app.isSystem !== true || app.configured)
)

const initialPageLoading = computed(() => isInitializing.value)
const backgroundLoading = computed(() => appsStore.loading && !initialPageLoading.value)

const configuredCount = computed(() => visibleApps.value.filter((app) => app.configured).length)

const filteredApps = computed(() => {
  let apps = visibleApps.value

  const q = debouncedSearch.value.toLowerCase()
  if (q) {
    // 使用预计算的小写字段,键入时不再对全量列表做 toLowerCase
    apps = apps.filter((app) => app.pkgLower.includes(q) || app.nameLower.includes(q))
  }

  if (filterType.value === 'configured') {
    apps = apps.filter((app) => app.configured)
  }

  // 预计算 sortKey 后排序退化为纯字符串比较(归一化包名 → userId → 已安装优先)
  return apps.slice().sort((a, b) => (a.sortKey < b.sortKey ? -1 : a.sortKey > b.sortKey ? 1 : 0))
})

const emptyText = computed(() => {
  if (debouncedSearch.value) return t('apps.empty.search')
  if (filterType.value === 'configured') return t('apps.empty.configured')
  return t('apps.empty.all')
})

function openConfig(app: InstalledApp) {
  currentApp.value = app
  configDialogVisible.value = true
}

function handleConfigSaved() {
  // 预留钩子，未来可在保存后刷新列表或提示
}

async function loadApps(includeSystem: boolean) {
  await appsStore.ensureUserAppsLoaded()
  isInitializing.value = false

  void appsStore.resolvePackagesInfo(configuredPackageState.value.packages)
  if (includeSystem) {
    void appsStore.ensureSystemAppsLoaded()
  }
}

onMounted(async () => {
  await loadApps(showSystemApps.value)
})

watch(showSystemApps, (enabled, previous) => {
  if (enabled && enabled !== previous) {
    void appsStore.ensureSystemAppsLoaded()
  }
})

watch(
  () => configuredPackageState.value.packages,
  (packages) => {
    if (!packages.length) return
    void appsStore.resolvePackagesInfo(packages)
  },
  { deep: false }
)
</script>

<style scoped>
.apps-page {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  box-sizing: border-box;
  overflow: hidden;
}
</style>
