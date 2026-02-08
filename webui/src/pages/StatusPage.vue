<template>
  <div class="status-page">
    <div class="status-card glass-effect">
      <h2 class="card-title">{{ t('status.items.module_status') }}</h2>

      <div class="status-grid">
        <div class="status-item">
          <div class="status-icon gradient-icon-1">
            <Shield :size="32" />
          </div>
          <div class="status-info">
            <span class="status-label">{{ t('status.items.module_version') }}</span>
            <span class="status-value">{{ moduleVersionMain }}</span>
            <span v-if="moduleVersionBuild" class="status-build">{{ moduleVersionBuild }}</span>
          </div>
        </div>

        <div class="status-item">
          <div class="status-icon gradient-icon-2">
            <Smartphone :size="32" />
          </div>
          <div class="status-info">
            <span class="status-label">{{ t('status.items.impersonated_apps_count') }}</span>
            <span class="status-value">{{ deviceFakerCount }}</span>
          </div>
        </div>

        <div class="status-item">
          <div class="status-icon gradient-icon-3">
            <FileText :size="32" />
          </div>
          <div class="status-info">
            <span class="status-label">{{ t('status.items.templates_count') }}</span>
            <span class="status-value">{{ templateCount }}</span>
          </div>
        </div>

        <div class="status-item clickable" @click="handleToggleWorkMode">
          <div class="status-icon gradient-icon-4">
            <Settings :size="32" />
          </div>
          <div class="status-info">
            <span class="status-label">{{ t('status.items.work_mode') }}</span>
            <span class="status-value">{{ workMode }}</span>
          </div>
        </div>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onActivated } from 'vue'
import { Shield, Smartphone, FileText, Settings } from 'lucide-vue-next'
import { useConfigStore } from '../stores/config'
import { useI18n } from '../utils/i18n'

const configStore = useConfigStore()
const { t } = useI18n()

// 直接使用 store 中的 computed 属性，避免重复计算
const moduleVersion = computed(() => configStore.moduleVersion)
const moduleVersionMain = computed(() => {
  const v = moduleVersion.value
  const idx = v.indexOf('(')
  return idx > 0 ? v.substring(0, idx).trim() : v
})
const moduleVersionBuild = computed(() => {
  const v = moduleVersion.value
  const match = v.match(/\((.+)\)/)
  return match ? match[1] : ''
})
const deviceFakerCount = computed(() => configStore.deviceFakerCount)
const templateCount = computed(() => configStore.templateCount)
const workMode = computed(() => {
  const mode = configStore.config.default_mode || 'lite'
  return mode === 'lite' ? t('status.mode.lite') : t('status.mode.full')
})

async function handleToggleWorkMode() {
  await configStore.toggleWorkMode()
}

// KeepAlive 激活时的钩子
onActivated(() => {
  // 页面激活
})
</script>

<style scoped>
.status-page {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  width: 100%;
  max-width: 100%;
  box-sizing: border-box;
  /* 确保宽度稳定，不受滚动条影响 */
  overflow: hidden;
}

.status-card {
  padding: 1.5rem;
  border-radius: 1rem;
  box-shadow: 0 4px 12px var(--shadow);
  position: relative;
  overflow: hidden;
}

.status-card::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(135deg, rgba(14, 165, 233, 0.03) 0%, rgba(168, 85, 247, 0.03) 100%);
  pointer-events: none;
}

.card-title {
  font-size: 1.25rem;
  font-weight: 600;
  margin-bottom: 1.5rem;
  color: var(--text);
  position: relative;
}

.status-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 1rem;
}

.status-item {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 1rem;
  background: var(--background);
  border-radius: 0.75rem;
  transition: all 0.15s ease;
  -webkit-tap-highlight-color: transparent;
}

.status-item.clickable {
  user-select: none;
  -webkit-user-select: none;
}

.status-item.clickable:active {
  background: linear-gradient(135deg, rgba(14, 165, 233, 0.1) 0%, rgba(168, 85, 247, 0.1) 100%);
  transform: scale(0.98);
  opacity: 0.9;
}

.status-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  border-radius: 12px;
  position: relative;
}

.gradient-icon-1 {
  background: linear-gradient(135deg, #0ea5e9 0%, #38bdf8 100%);
  color: white;
  box-shadow: 0 4px 12px rgba(14, 165, 233, 0.3);
}

.gradient-icon-2 {
  background: linear-gradient(135deg, #06b6d4 0%, #0ea5e9 100%);
  color: white;
  box-shadow: 0 4px 12px rgba(6, 182, 212, 0.3);
}

.gradient-icon-3 {
  background: linear-gradient(135deg, #8b5cf6 0%, #a855f7 100%);
  color: white;
  box-shadow: 0 4px 12px rgba(139, 92, 246, 0.3);
}

.gradient-icon-4 {
  background: linear-gradient(135deg, #a855f7 0%, #c084fc 100%);
  color: white;
  box-shadow: 0 4px 12px rgba(168, 85, 247, 0.3);
}

.status-info {
  display: flex;
  flex-direction: column;
}

.status-label {
  font-size: 0.875rem;
  color: var(--text-secondary);
}

.status-value {
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--text);
}

.status-build {
  font-size: 0.75rem;
  color: var(--text-secondary);
  opacity: 0.7;
  font-family: monospace;
}
</style>
