<template>
  <div class="apps-page">
    <div class="page-header">
      <h2 class="page-title">{{ t('apps.title') }}</h2>
      <el-input
        v-model="searchQuery"
        :placeholder="t('apps.search_placeholder')"
        prefix-icon="Search"
        clearable
        style="max-width: 300px"
      />
    </div>

    <div class="filter-tabs">
      <button :class="['filter-tab', { active: filterType === 'all' }]" @click="filterType = 'all'">
        {{ t('apps.tabs.all') }} ({{ installedApps.length }})
      </button>
      <button
        :class="['filter-tab', { active: filterType === 'configured' }]"
        @click="filterType = 'configured'"
      >
        {{ t('apps.tabs.configured') }} ({{ configuredCount }})
      </button>
      <button
        :class="['filter-tab', { active: filterType === 'unconfigured' }]"
        @click="filterType = 'unconfigured'"
      >
        {{ t('apps.tabs.unconfigured') }} ({{ unconfiguredCount }})
      </button>
    </div>

    <div
      v-loading="loading"
      element-loading-text=""
      element-loading-spinner="el-icon-loading"
      element-loading-background="transparent"
      class="apps-list"
    >
      <div
        v-for="app in filteredApps"
        :key="app.packageName"
        class="app-card glass-effect"
        @click="configureApp(app)"
      >
        <div class="app-icon-container" :data-package="app.packageName">
          <div
            v-if="
              !appIcons[app.packageName] ||
              (appIcons[app.packageName] !== 'fallback' && !iconLoaded[app.packageName])
            "
            class="icon-loader"
          ></div>
          <img
            v-if="appIcons[app.packageName] && appIcons[app.packageName] !== 'fallback'"
            :src="appIcons[app.packageName]"
            class="app-icon-img"
            :class="{ loaded: iconLoaded[app.packageName] }"
            alt=""
            loading="lazy"
            @load="onIconLoad(app.packageName)"
            @error="onIconError(app.packageName)"
          />
          <Smartphone
            v-if="appIcons[app.packageName] === 'fallback'"
            :size="40"
            class="app-icon-fallback"
          />
        </div>
        <div class="app-info">
          <h3 class="app-name">{{ app.appName }}</h3>
          <p class="app-package">{{ app.packageName }}</p>
          <p v-if="getAppConfig(app.packageName)" class="app-status configured">
            <Check :size="14" />
            {{ t('apps.status.configured') }}
          </p>
          <p v-else class="app-status unconfigured">{{ t('apps.status.unconfigured') }}</p>
        </div>
        <div class="app-actions">
          <ChevronRight :size="20" />
        </div>
      </div>

      <div v-if="!loading && filteredApps.length === 0" class="empty-state">
        <Smartphone :size="64" class="empty-icon" />
        <p class="empty-text">{{ emptyText }}</p>
      </div>
    </div>

    <!-- 配置对话框 -->
    <el-dialog
      v-model="configDialogVisible"
      :title="t('apps.dialog.config_title', { name: currentApp?.appName || '' })"
      width="90%"
      :close-on-click-modal="false"
      :append-to-body="true"
      :destroy-on-close="true"
      :z-index="2001"
      class="app-config-dialog"
      modal-class="app-config-modal"
    >
      <div class="config-options">
        <el-radio-group v-model="configMode">
          <el-radio label="template">{{ t('apps.dialog.mode_template') }}</el-radio>
          <el-radio label="custom">{{ t('apps.dialog.mode_custom') }}</el-radio>
          <el-radio label="remove">{{ t('apps.dialog.mode_remove') }}</el-radio>
        </el-radio-group>
      </div>

      <div v-if="configMode === 'template'" class="template-selector">
        <el-select
          v-model="selectedTemplate"
          :placeholder="t('apps.dialog.select_template_placeholder')"
          style="width: 100%"
        >
          <el-option
            v-for="(template, name) in templates"
            :key="name"
            :label="`${name} - ${template.brand} ${template.model}`"
            :value="name"
          />
        </el-select>
      </div>

      <div v-if="configMode === 'custom'" class="custom-config">
        <el-form :model="customFormData" label-width="120px" label-position="top">
          <el-form-item :label="t('templates.fields.manufacturer')">
            <el-input v-model="customFormData.manufacturer" placeholder="例如：ZTE" />
          </el-form-item>
          <el-form-item :label="t('templates.fields.brand')">
            <el-input v-model="customFormData.brand" placeholder="例如：nubia" />
          </el-form-item>
          <el-form-item :label="t('templates.fields.model')">
            <el-input v-model="customFormData.model" placeholder="例如：25010PN30C，NX769J" />
          </el-form-item>
          <el-form-item :label="t('templates.fields.device')">
            <el-input v-model="customFormData.device" placeholder="例如：xuanyuan，NX769J" />
          </el-form-item>
          <el-form-item :label="t('templates.fields.product')">
            <el-input v-model="customFormData.product" placeholder="例如：xuanyuan，NX769J" />
          </el-form-item>
          <el-form-item :label="t('templates.fields.name_field')">
            <el-input v-model="customFormData.name" placeholder="例如：xuanyuan" />
          </el-form-item>
          <el-form-item :label="t('templates.fields.market_name')">
            <el-input v-model="customFormData.marketname" placeholder="例如：REDMAGIC 9 Pro" />
          </el-form-item>
          <el-form-item :label="t('templates.fields.fingerprint')">
            <el-input
              v-model="customFormData.fingerprint"
              type="textarea"
              :rows="3"
              placeholder="例如：nubia/NX769J/NX769J:14/UKQ1.230917.001/20240813.173312:user/release-keys"
            />
          </el-form-item>
          <el-form-item :label="t('templates.fields.mode')">
            <el-select
              v-model="customFormData.mode"
              :placeholder="t('templates.placeholders.mode')"
              clearable
              style="width: 100%"
            >
              <el-option :label="t('templates.options.mode_lite')" value="lite" />
              <el-option :label="t('templates.options.mode_full')" value="full" />
            </el-select>
          </el-form-item>
        </el-form>
      </div>

      <div v-if="configMode === 'remove'" class="remove-hint">
        <p>{{ t('apps.dialog.remove_hint') }}</p>
      </div>

      <template #footer>
        <el-button @click="configDialogVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" @click="saveAppConfig">{{ t('common.confirm') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onActivated, onUnmounted, watch, nextTick } from 'vue'
import { Smartphone, ChevronRight, Check } from 'lucide-vue-next'
import { ElMessage } from 'element-plus'
import { useConfigStore } from '../stores/config'
import { useAppsStore } from '../stores/apps'
import { useI18n } from '../utils/i18n'
import type { InstalledApp, AppConfig } from '../types'

const configStore = useConfigStore()
const appsStore = useAppsStore()
const { t } = useI18n()

const searchQuery = ref('')
const filterType = ref<'all' | 'configured' | 'unconfigured'>('all')
const loading = computed(() => appsStore.loading)
const installedApps = computed(() => appsStore.installedApps)
const templates = computed(() => configStore.getTemplates())
const appIcons = ref<Record<string, string>>({})
const iconLoaded = ref<Record<string, boolean>>({})
const iconObserver = ref<IntersectionObserver | null>(null)

const configuredCount = computed(() => {
  return installedApps.value.filter((app: InstalledApp) =>
    configStore.isPackageConfigured(app.packageName)
  ).length
})

const unconfiguredCount = computed(() => {
  return installedApps.value.length - configuredCount.value
})

const filteredApps = computed(() => {
  let apps = installedApps.value

  // 搜索过滤
  if (searchQuery.value) {
    const q = searchQuery.value.toLowerCase()
    apps = apps.filter(
      (app: InstalledApp) =>
        app.packageName.toLowerCase().includes(q) || app.appName.toLowerCase().includes(q)
    )
  }

  // 配置状态过滤
  if (filterType.value === 'configured') {
    apps = apps.filter((app: InstalledApp) => configStore.isPackageConfigured(app.packageName))
  } else if (filterType.value === 'unconfigured') {
    apps = apps.filter((app: InstalledApp) => !configStore.isPackageConfigured(app.packageName))
  }

  return apps
})

const emptyText = computed(() => {
  if (searchQuery.value) {
    return t('apps.empty.search')
  }
  if (filterType.value === 'configured') {
    return t('apps.empty.configured')
  }
  if (filterType.value === 'unconfigured') {
    return t('apps.empty.unconfigured')
  }
  return t('apps.empty.all')
})

const configDialogVisible = ref(false)
const currentApp = ref<InstalledApp | null>(null)
const configMode = ref<'template' | 'custom' | 'remove'>('template')
const selectedTemplate = ref('')
const customFormData = ref({
  manufacturer: '',
  brand: '',
  model: '',
  device: '',
  product: '',
  name: '',
  marketname: '',
  fingerprint: '',
  mode: undefined as 'lite' | 'full' | undefined,
})

function getAppConfig(packageName: string) {
  return configStore.getPackageConfig(packageName)
}

function configureApp(app: InstalledApp) {
  currentApp.value = app
  const existingConfig = getAppConfig(app.packageName)

  if (existingConfig) {
    if ('source' in existingConfig) {
      // 来自模板
      configMode.value = 'template'
      selectedTemplate.value = existingConfig.source
    } else {
      // 自定义配置
      configMode.value = 'custom'
      customFormData.value = {
        manufacturer: existingConfig.manufacturer || '',
        brand: existingConfig.brand || '',
        model: existingConfig.model || '',
        device: existingConfig.device || '',
        product: existingConfig.product || '',
        name: existingConfig.name || '',
        marketname: existingConfig.marketname || '',
        fingerprint: existingConfig.fingerprint || '',
        mode: existingConfig.mode as 'lite' | 'full' | undefined,
      }
    }
  } else {
    configMode.value = 'template'
    selectedTemplate.value = ''
  }

  configDialogVisible.value = true
}

async function saveAppConfig() {
  if (!currentApp.value) return

  if (configMode.value === 'remove') {
    // 移除配置
    configStore.deleteApp(currentApp.value.packageName)
    // 同时检查并从模板的packages中移除
    const templates = configStore.getTemplates()
    for (const [name, template] of Object.entries(templates)) {
      if (template.packages?.includes(currentApp.value.packageName)) {
        template.packages = template.packages.filter(
          (p: string) => p !== currentApp.value!.packageName
        )
        configStore.setTemplate(name, template)
      }
    }
  } else if (configMode.value === 'template') {
    if (!selectedTemplate.value) {
      ElMessage.error(t('apps.messages.select_template'))
      return
    }
    // 应用模板
    const template = templates.value[selectedTemplate.value]
    if (!template.packages) {
      template.packages = []
    }
    if (!template.packages.includes(currentApp.value.packageName)) {
      template.packages.push(currentApp.value.packageName)
      configStore.setTemplate(selectedTemplate.value, template)
    }
  } else if (configMode.value === 'custom') {
    // 自定义配置
    const appConfig: AppConfig = {
      package: currentApp.value.packageName,
      manufacturer: customFormData.value.manufacturer,
      brand: customFormData.value.brand,
      model: customFormData.value.model,
      device: customFormData.value.device,
      product: customFormData.value.product,
      name: customFormData.value.name,
      marketname: customFormData.value.marketname,
      fingerprint: customFormData.value.fingerprint,
      mode: customFormData.value.mode,
    }
    configStore.setApp(appConfig)
  }

  try {
    await configStore.saveConfig()
    ElMessage.success(t('apps.messages.saved'))
    configDialogVisible.value = false
  } catch {
    ElMessage.error(t('common.failed'))
  }
}

// 加载应用图标
async function loadAppIcon(packageName: string) {
  if (appIcons.value[packageName]) return

  // 首先检查是否有 KernelSU 新的 WebUI 包管理器 API（支持自 v2.1.2）
  if (typeof globalThis.ksu?.getPackagesInfo === 'function') {
    appIcons.value[packageName] = `ksu://icon/${packageName}`
    return
  }

  // 回退到其他图标加载方法
  try {
    // 检查是否在 WebUI-X 环境
    if (typeof window.$packageManager !== 'undefined') {
      const pm = window.$packageManager

      try {
        const stream = pm.getApplicationIcon(packageName, 0, 0)
        if (!stream) {
          appIcons.value[packageName] = 'fallback'
          return
        }

        // 动态加载 wrapInputStream
        await loadWrapInputStream()
        const wrapInputStream = window.wrapInputStream

        if (wrapInputStream) {
          const wrapped = await wrapInputStream(stream)
          const buffer = await wrapped.arrayBuffer()
          const base64 = arrayBufferToBase64(buffer)
          appIcons.value[packageName] = `data:image/png;base64,${base64}`
          // Base64 图片加载也需要触发 load 事件，但这里我们直接标记为 loaded
          // 或者让 img 标签的 @load 处理
          return
        }
      } catch {
        // Fallback to KernelSU API
      }
    }

    // 如果没有可用的 API，停止加载动画
    appIcons.value[packageName] = 'fallback'
  } catch {
    // 标记为失败，停止加载动画
    appIcons.value[packageName] = 'fallback'
  }
}

function onIconLoad(packageName: string) {
  iconLoaded.value[packageName] = true
}

function onIconError(packageName: string) {
  appIcons.value[packageName] = 'fallback'
  iconLoaded.value[packageName] = true // 停止 loading 动画
}

// 动态加载 wrapInputStream 工具
async function loadWrapInputStream() {
  if (typeof window.wrapInputStream === 'undefined') {
    try {
      // 从 KernelSU WebUI 服务器加载
      const moduleUrl = 'https://mui.kernelsu.org/internal/assets/ext/wrapInputStream.mjs'
      const importFn = new Function('url', 'return import(url)')
      const module = await importFn(moduleUrl)
      window.wrapInputStream = module.wrapInputStream
    } catch {
      // wrapInputStream not available
    }
  }
}

// 将 ArrayBuffer 转为 Base64
function arrayBufferToBase64(buffer: ArrayBuffer): string {
  const uint8Array = new Uint8Array(buffer)
  let binary = ''
  uint8Array.forEach((byte) => (binary += String.fromCharCode(byte)))
  return btoa(binary)
}

// 设置图标懒加载观察器
function setupIconObserver() {
  if (iconObserver.value) {
    iconObserver.value.disconnect()
  }

  iconObserver.value = new IntersectionObserver(
    (entries) => {
      entries.forEach((entry) => {
        if (entry.isIntersecting) {
          const container = entry.target as HTMLElement
          const packageName = container.dataset.package
          if (packageName) {
            loadAppIcon(packageName)
            iconObserver.value?.unobserve(container)
          }
        }
      })
    },
    {
      rootMargin: '100px',
      threshold: 0.1,
    }
  )

  // 观察所有图标容器
  nextTick(() => {
    const containers = document.querySelectorAll('.app-icon-container')
    containers.forEach((container) => {
      iconObserver.value?.observe(container)
    })
  })
}

// 监听应用列表变化，重新设置观察器
watch(filteredApps, () => {
  nextTick(() => {
    setupIconObserver()
  })
})

onMounted(async () => {
  // 预加载 wrapInputStream（如果在 WebUI-X 环境）
  if (typeof window.$packageManager !== 'undefined') {
    await loadWrapInputStream()
  }

  // 初次加载应用列表
  if (appsStore.installedApps.length === 0) {
    await appsStore.loadInstalledApps()
  }
  setupIconObserver()
})

onActivated(() => {
  // KeepAlive 激活时，重新设置图标观察器，但不重新加载应用列表
  setupIconObserver()
})

onUnmounted(() => {
  // 清理观察器
  if (iconObserver.value) {
    iconObserver.value.disconnect()
    iconObserver.value = null
  }
})
</script>

<style scoped>
.apps-page {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  width: 100%;
  max-width: 100%;
  box-sizing: border-box;
  /* 确保宽度稳定，不受滚动条影响 */
  overflow: hidden;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  gap: 1rem;
  margin-bottom: 0.5rem;
}

.page-title {
  font-size: 1.5rem;
  font-weight: 600;
  color: var(--text);
}

.filter-tabs {
  display: flex;
  gap: 0.5rem;
  overflow-x: auto;
  padding-bottom: 0.5rem;
}

.filter-tab {
  padding: 0.5rem 1rem;
  background: var(--card-bg);
  border: 1px solid var(--border);
  border-radius: 0.5rem;
  color: var(--text-secondary);
  font-size: 0.875rem;
  font-weight: 500;
  transition: all 0.2s ease;
  white-space: nowrap;
  -webkit-tap-highlight-color: transparent;
  user-select: none;
  -webkit-user-select: none;
}

.filter-tab:active {
  transform: scale(0.95);
  opacity: 0.8;
}

.filter-tab.active {
  background: var(--primary);
  color: white;
  border-color: var(--primary);
}

.apps-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.app-card {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 1rem;
  border-radius: 0.75rem;
  box-shadow: 0 2px 8px var(--shadow);
  transition: all 0.2s ease;
  -webkit-tap-highlight-color: transparent;
  user-select: none;
  -webkit-user-select: none;
}

.app-card:active {
  transform: scale(0.98);
  box-shadow: 0 1px 4px var(--shadow);
}

.app-icon-container {
  position: relative;
  display: flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  flex-shrink: 0;
  border-radius: 0.75rem;
  overflow: hidden;
}

.icon-loader {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  background: linear-gradient(90deg, var(--card-bg), var(--border), var(--card-bg));
  background-size: 200% 100%;
  animation: shimmer 1.2s infinite linear;
  border-radius: 0.75rem;
}

@keyframes shimmer {
  0% {
    background-position: -200% 0;
  }
  100% {
    background-position: 200% 0;
  }
}

.app-icon-img {
  width: 100%;
  height: 100%;
  object-fit: cover;
  border-radius: 0.75rem;
  opacity: 0;
  transition: opacity 0.3s ease;
}

.app-icon-img.loaded {
  opacity: 1;
}

.app-icon-fallback {
  color: var(--primary);
}

.app-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  background: var(--background);
  border-radius: 0.75rem;
  color: var(--primary);
}

.app-info {
  flex: 1;
  min-width: 0;
}

.app-name {
  font-size: 1rem;
  font-weight: 600;
  color: var(--text);
  margin-bottom: 0.25rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.app-package {
  font-size: 0.75rem;
  color: var(--text-secondary);
  margin-bottom: 0.25rem;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.app-status {
  display: inline-flex;
  align-items: center;
  gap: 0.25rem;
  font-size: 0.75rem;
  padding: 0.125rem 0.5rem;
  border-radius: 0.25rem;
}

.app-status.configured {
  background: rgba(34, 197, 94, 0.1);
  color: #22c55e;
}

.app-status.unconfigured {
  background: rgba(156, 163, 175, 0.1);
  color: var(--text-secondary);
}

.app-actions {
  display: flex;
  align-items: center;
  color: var(--text-secondary);
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 3rem 1rem;
  text-align: center;
}

.empty-icon {
  color: var(--text-secondary);
  opacity: 0.3;
  margin-bottom: 1rem;
}

.empty-text {
  font-size: 1.125rem;
  font-weight: 500;
  color: var(--text);
}

.config-options {
  margin-bottom: 1.5rem;
}

.template-selector,
.custom-config {
  margin-top: 1rem;
}

.remove-hint {
  padding: 1.5rem;
  text-align: center;
  color: var(--text-secondary);
}

/* Dialog 样式优化 */
.app-config-dialog :deep(.el-dialog) {
  margin-top: 5vh !important;
  margin-bottom: 80px !important; /* 为底栏留出足够空间 */
  max-height: calc(100vh - 80px - 10vh) !important; /* 减去底栏高度和顶部边距 */
  display: flex;
  flex-direction: column;
  /* 增强的毛玻璃效果 */
  background: rgba(255, 255, 255, 0.15) !important;
  backdrop-filter: blur(40px) saturate(150%) brightness(1.1);
  -webkit-backdrop-filter: blur(40px) saturate(150%) brightness(1.1);
  border: 1px solid rgba(255, 255, 255, 0.4);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.15);
}

/* 深色模式下的毛玻璃效果 */
@media (prefers-color-scheme: dark) {
  .app-config-dialog :deep(.el-dialog) {
    background: rgba(20, 20, 20, 0.6) !important;
    backdrop-filter: blur(40px) saturate(150%) brightness(0.9);
    -webkit-backdrop-filter: blur(40px) saturate(150%) brightness(0.9);
    border: 1px solid rgba(255, 255, 255, 0.15);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }
}

.app-config-dialog :deep(.el-dialog__body) {
  flex: 1;
  overflow-y: auto;
  padding: 1.5rem;
  padding-bottom: 2rem; /* 增加底部内边距 */
  max-height: calc(100vh - 200px); /* 确保有足够的滚动空间 */
  background: transparent;
  /* 隐藏滚动条 */
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.app-config-dialog :deep(.el-dialog__body::-webkit-scrollbar) {
  display: none;
}

.app-config-dialog :deep(.el-dialog__header) {
  background: rgba(255, 255, 255, 0.1);
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
  border-bottom: 1px solid rgba(255, 255, 255, 0.2);
}

@media (prefers-color-scheme: dark) {
  .app-config-dialog :deep(.el-dialog__header) {
    background: rgba(0, 0, 0, 0.2);
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }
}

.app-config-dialog :deep(.el-dialog__footer) {
  padding: 1rem 1.5rem;
  border-top: 1px solid rgba(255, 255, 255, 0.2);
  background: rgba(255, 255, 255, 0.15);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  flex-shrink: 0; /* 防止底部按钮区域被压缩 */
}

@media (prefers-color-scheme: dark) {
  .app-config-dialog :deep(.el-dialog__footer) {
    border-top: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(0, 0, 0, 0.3);
  }
}

/* 确保 Dialog 遮罩层在底栏之上，并添加模糊效果 */
.app-config-dialog :deep(.el-overlay) {
  z-index: 2000 !important;
  backdrop-filter: blur(8px) brightness(0.7) !important;
  -webkit-backdrop-filter: blur(8px) brightness(0.7) !important;
  background-color: rgba(0, 0, 0, 0.6) !important;
}

@media (prefers-color-scheme: dark) {
  .app-config-dialog :deep(.el-overlay) {
    backdrop-filter: blur(8px) brightness(0.5) !important;
    -webkit-backdrop-filter: blur(8px) brightness(0.5) !important;
    background-color: rgba(0, 0, 0, 0.7) !important;
  }
}
</style>

<style>
/* 全局样式：Dialog 遮罩层毛玻璃效果 */
.app-config-modal {
  backdrop-filter: blur(12px) saturate(120%) !important;
  -webkit-backdrop-filter: blur(12px) saturate(120%) !important;
  background-color: rgba(0, 0, 0, 0.25) !important;
}

@media (prefers-color-scheme: dark) {
  .app-config-modal {
    backdrop-filter: blur(12px) saturate(120%) !important;
    -webkit-backdrop-filter: blur(12px) saturate(120%) !important;
    background-color: rgba(0, 0, 0, 0.4) !important;
  }
}
</style>
