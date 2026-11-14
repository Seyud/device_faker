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
            <span class="status-transition-slot">
              <Transition name="status-swap">
                <span
                  v-if="moduleMetaReady"
                  key="module-version"
                  class="status-value status-value--resolved"
                >
                  {{ moduleVersionDisplay }}
                </span>
                <span
                  v-else
                  key="module-version-skeleton"
                  class="status-value-skeleton status-value-skeleton--wide"
                ></span>
              </Transition>
            </span>
            <span class="status-transition-slot status-transition-slot--build">
              <Transition name="status-swap">
                <span v-if="moduleVersionBuild" key="module-build" class="status-build">{{
                  moduleVersionBuild
                }}</span>
                <span
                  v-else-if="!moduleMetaReady"
                  key="module-build-skeleton"
                  class="status-build-skeleton"
                ></span>
                <span v-else key="module-build-empty" class="status-build-placeholder"></span>
              </Transition>
            </span>
          </div>
        </div>

        <div class="status-item">
          <div class="status-icon gradient-icon-2">
            <Smartphone :size="32" />
          </div>
          <div class="status-info">
            <span class="status-label">{{ t('status.items.impersonated_apps_count') }}</span>
            <span class="status-transition-slot">
              <Transition name="status-swap">
                <span
                  v-if="configReady"
                  key="device-faker-count"
                  class="status-value status-value--resolved"
                >
                  {{ deviceFakerCountDisplay }}
                </span>
                <span
                  v-else
                  key="device-faker-count-skeleton"
                  class="status-value-skeleton status-value-skeleton--short"
                ></span>
              </Transition>
            </span>
          </div>
        </div>

        <div class="status-item">
          <div class="status-icon gradient-icon-3">
            <FileText :size="32" />
          </div>
          <div class="status-info">
            <span class="status-label">{{ t('status.items.templates_count') }}</span>
            <span class="status-transition-slot">
              <Transition name="status-swap">
                <span
                  v-if="configReady"
                  key="template-count"
                  class="status-value status-value--resolved"
                >
                  {{ templateCountDisplay }}
                </span>
                <span
                  v-else
                  key="template-count-skeleton"
                  class="status-value-skeleton status-value-skeleton--short"
                ></span>
              </Transition>
            </span>
          </div>
        </div>

        <div class="status-item clickable" @click="followDialogVisible = true">
          <div class="status-icon gradient-icon-5">
            <HeartHandshake :size="32" />
          </div>
          <div class="status-info">
            <span class="status-label">{{ t('status.follow.title') }}</span>
            <span class="status-value">{{ t('status.follow.action') }}</span>
            <span class="status-build">{{ t('status.follow.channels') }}</span>
          </div>
        </div>

        <div class="status-item clickable" @click="translatorsDialogVisible = true">
          <div class="status-icon gradient-icon-5">
            <Languages :size="32" />
          </div>
          <div class="status-info">
            <span class="status-label">{{ t('status.translators.title') }}</span>
            <span class="status-value">{{ t('status.translators.action') }}</span>
          </div>
        </div>
      </div>
    </div>

    <!-- 弹窗按需异步加载:避免 el-dialog/el-button 及其样式进入首屏关键路径 -->
    <FollowDialog v-if="followDialogVisible" v-model="followDialogVisible" />
    <TranslatorsDialog v-if="translatorsDialogVisible" v-model="translatorsDialogVisible" />
  </div>
</template>

<script setup lang="ts">
import { computed, defineAsyncComponent, onActivated, ref } from 'vue'
import { Shield, Smartphone, FileText, Languages, HeartHandshake } from '@lucide/vue'
import { useConfigStore } from '../stores/config'
import { useI18n } from '../utils/i18n'

const FollowDialog = defineAsyncComponent(() => import('../components/status/FollowDialog.vue'))
const TranslatorsDialog = defineAsyncComponent(
  () => import('../components/status/TranslatorsDialog.vue')
)

const configStore = useConfigStore()
const { t } = useI18n()
const followDialogVisible = ref(false)
const translatorsDialogVisible = ref(false)

// 直接使用 store 中的 computed 属性，避免重复计算
const moduleVersion = computed(() => configStore.moduleVersion)
const configReady = computed(() => configStore.configReady)
const moduleMetaReady = computed(() => configStore.moduleMetaReady)
const moduleVersionDisplay = computed(() =>
  moduleMetaReady.value && moduleVersionMain.value ? moduleVersionMain.value : '--'
)
const moduleVersionMain = computed(() => {
  const v = moduleVersion.value
  const idx = v.indexOf('(')
  return idx > 0 ? v.substring(0, idx).trim() : v
})
const moduleVersionBuild = computed(() => {
  if (!moduleMetaReady.value) {
    return ''
  }

  const v = moduleVersion.value
  const match = v.match(/\((.+)\)/)
  return match ? match[1] : ''
})
const deviceFakerCountDisplay = computed(() =>
  configReady.value ? String(configStore.deviceFakerCount) : '--'
)
const templateCountDisplay = computed(() =>
  configReady.value ? String(configStore.templateCount) : '--'
)
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

.status-item.disabled {
  opacity: 0.65;
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
  flex-shrink: 0;
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

.gradient-icon-5 {
  background: linear-gradient(135deg, #f97316 0%, #fb7185 100%);
  color: white;
  box-shadow: 0 4px 12px rgba(249, 115, 22, 0.3);
}

.status-info {
  display: flex;
  flex-direction: column;
  min-width: 0;
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

.status-value--resolved {
  display: inline-flex;
  align-items: center;
}

.status-build {
  font-size: 0.75rem;
  color: var(--text-secondary);
  opacity: 0.7;
  font-family: monospace;
}

.status-build-placeholder {
  display: inline-flex;
  height: 0.75rem;
}

.status-transition-slot {
  display: inline-grid;
  align-items: center;
  justify-items: start;
  min-height: 1.75rem;
}

.status-transition-slot > * {
  grid-area: 1 / 1;
}

.status-transition-slot--build {
  min-height: 0.75rem;
  margin-top: 0.25rem;
}

.status-value-skeleton,
.status-build-skeleton {
  display: inline-flex;
  border-radius: 999px;
  background: linear-gradient(90deg, var(--border) 25%, var(--card-bg) 50%, var(--border) 75%);
  background-size: 200% 100%;
  animation: status-skeleton-shimmer 1.3s linear infinite;
  opacity: 0.8;
}

.status-value-skeleton {
  height: 1.35rem;
  margin-top: 0.15rem;
}

.status-value-skeleton--short {
  width: 2.75rem;
}

.status-value-skeleton--medium {
  width: 5.5rem;
}

.status-value-skeleton--wide {
  width: 7.5rem;
}

.status-build-skeleton {
  width: 4.25rem;
  height: 0.75rem;
  margin-top: 0.25rem;
}

@keyframes status-skeleton-shimmer {
  from {
    background-position: -200% 0;
  }

  to {
    background-position: 200% 0;
  }
}

.status-swap-enter-active,
.status-swap-leave-active {
  transition:
    opacity 0.22s ease,
    transform 0.22s ease;
}

.status-swap-enter-from,
.status-swap-leave-to {
  opacity: 0;
  transform: translateY(6px);
}
</style>
