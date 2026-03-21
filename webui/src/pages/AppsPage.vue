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
        v-model:search-query="searchQuery"
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
import { computed, defineAsyncComponent, onMounted, ref, watch } from 'vue'
import AppFilters from '../components/apps/AppFilters.vue'
import AppList from '../components/apps/AppList.vue'
import AppsPageSkeleton from '../components/apps/AppsPageSkeleton.vue'
import { useAppsStore } from '../stores/apps'
import { useConfigStore } from '../stores/config'
import { useSettingsStore } from '../stores/settings'
import { useI18n } from '../utils/i18n'
import { normalizePackageName } from '../utils/package'
import type { InstalledApp } from '../types'

type FilterType = 'all' | 'configured'
type AppListItem = InstalledApp & { configured: boolean }

const AppConfigDialog = defineAsyncComponent(() => import('../components/apps/AppConfigDialog.vue'))

const configStore = useConfigStore()
const appsStore = useAppsStore()
const settingsStore = useSettingsStore()
const { t } = useI18n()

const searchQuery = ref('')
const filterType = ref<FilterType>('all')
const configDialogVisible = ref(false)
const currentApp = ref<InstalledApp | null>(null)
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
  const normalizedPackages = new Set<string>()

  for (const app of installedApps.value) {
    if (app.installed === false) continue
    exactPackages.add(app.packageName)
    normalizedPackages.add(normalizePackageName(app.packageName))
  }

  return {
    exactPackages,
    normalizedPackages,
  }
})

const configuredPackageState = computed(() => {
  const exactPackages = new Set<string>()
  const normalizedPackages = new Set<string>()
  const configuredAppsMap = new Map<string, InstalledApp>()

  for (const appConfig of configStore.getApps()) {
    exactPackages.add(appConfig.package)
    normalizedPackages.add(normalizePackageName(appConfig.package))
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
      normalizedPackages.add(normalizePackageName(pkg))
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
    normalizedPackages,
    configuredApps: Array.from(configuredAppsMap.values()),
  }
})

function isConfiguredPackage(packageName: string) {
  if (configuredPackageState.value.exactPackages.has(packageName)) {
    return true
  }

  if (!/@\d+$/.test(packageName)) {
    return false
  }

  return configuredPackageState.value.normalizedPackages.has(normalizePackageName(packageName))
}

function isInstalledPackage(packageName: string) {
  if (installedPackageState.value.exactPackages.has(packageName)) {
    return true
  }

  if (!/@\d+$/.test(packageName)) {
    return false
  }

  return installedPackageState.value.normalizedPackages.has(normalizePackageName(packageName))
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

    const entry = {
      ...app,
      installed: app.installed ?? true,
      configured: isConfiguredPackage(app.packageName),
    }

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

    const entry = {
      packageName: app.packageName,
      appName: resolvedInfo?.appName || existingApp?.appName || app.packageName,
      icon: resolvedInfo?.icon || existingApp?.icon || '',
      versionName: resolvedInfo?.versionName || existingApp?.versionName || '',
      versionCode: resolvedInfo?.versionCode ?? existingApp?.versionCode ?? 0,
      installed: isInstalledPackage(app.packageName),
      isSystem: existingApp?.isSystem ?? resolvedInfo?.isSystem ?? app.isSystem,
      configured: true,
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

  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase()
    apps = apps.filter(
      (app) => app.packageName.toLowerCase().includes(q) || app.appName.toLowerCase().includes(q)
    )
  }

  if (filterType.value === 'configured') {
    apps = apps.filter((app) => app.configured)
  }

  return apps.slice().sort((a, b) => {
    const aInstalled = a.installed !== false
    const bInstalled = b.installed !== false

    if (aInstalled === bInstalled) return 0
    return aInstalled ? -1 : 1
  })
})

const emptyText = computed(() => {
  if (searchQuery.value) return t('apps.empty.search')
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
