<template>
  <div class="apps-list">
    <template v-if="loading">
      <div v-for="i in 15" :key="i" class="app-card skeleton-item">
        <div class="app-icon-container">
          <div class="skeleton-loader icon-placeholder"></div>
        </div>
        <div class="app-info">
          <div class="skeleton-loader name-placeholder"></div>
          <div class="skeleton-loader package-placeholder"></div>
          <div class="app-status-group">
            <div class="skeleton-loader badge-placeholder"></div>
            <div class="skeleton-loader badge-placeholder short"></div>
          </div>
        </div>
      </div>
    </template>

    <template v-else>
      <div v-for="app in apps" :key="app.packageName" class="app-card" @click="emit('select', app)">
        <div class="app-icon-container" :data-package="app.packageName">
          <div
            v-if="
              !appIcons[app.packageName] ||
              (appIcons[app.packageName] !== 'fallback' && !iconLoaded[app.packageName])
            "
            class="skeleton-loader icon-placeholder"
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
          <div class="app-status-group">
            <p v-if="app.configured && app.inherited" class="app-status inherited">
              <Link :size="14" />
              {{ t('apps.status.inherited') }}
            </p>
            <p v-else-if="app.configured" class="app-status configured">
              <Check :size="14" />
              {{ t('apps.status.configured') }}
            </p>
            <p v-else class="app-status unconfigured">{{ t('apps.status.unconfigured') }}</p>
            <p v-if="!isInstalled(app)" class="app-status not-installed">
              {{ t('apps.status.not_installed') }}
            </p>
          </div>
        </div>
        <div class="app-actions">
          <ChevronRight :size="20" />
        </div>
      </div>

      <div v-if="apps.length === 0" class="empty-state">
        <Smartphone :size="64" class="empty-icon" />
        <p class="empty-text">{{ emptyText }}</p>
      </div>
    </template>
  </div>
</template>

<script setup lang="ts">
import { computed, nextTick, onMounted, onUnmounted, watch } from 'vue'
import { Smartphone, ChevronRight, Check, Link } from '@lucide/vue'
import { useI18n } from '../../utils/i18n'
import { useAppIcons } from '../../composables/useAppIcons'
import type { InstalledApp } from '../../types'

type AppListItem = InstalledApp & { configured: boolean; inherited: boolean }

const props = defineProps<{
  apps: AppListItem[]
  emptyText: string
  loading: boolean
}>()

const emit = defineEmits<{ select: [AppListItem] }>()

const { t } = useI18n()
const { appIcons, iconLoaded, onIconLoad, onIconError, setupIconObserver, teardownIconObserver } =
  useAppIcons()

const isInstalled = (app: InstalledApp) => app.installed !== false

const loading = computed(() => props.loading)

watch(
  () => [props.apps, props.loading],
  () => {
    nextTick(() => setupIconObserver())
  }
)

onMounted(() => {
  setupIconObserver()
})

onUnmounted(() => {
  teardownIconObserver()
})
</script>

<style scoped>
.apps-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

/*
 * 列表卡片不使用 .glass-effect 毛玻璃:backdrop-filter 是 WebView 滚动中
 * 最昂贵的绘制操作,几百张卡逐帧实时高斯模糊必然掉帧。
 * 注意:不要写 backdrop-filter:none 去覆盖全局类——压缩器会把初始值的标准
 * 属性删掉只留 -webkit- 前缀(Chromium 已移除该别名),导致覆盖失效;
 * 直接从模板上摘掉 glass-effect 类才是可靠做法。
 */
.apps-list .app-card {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 1rem;
  border-radius: 0.75rem;
  background: var(--card-bg);
  border: 1px solid var(--border);
  box-shadow: 0 1px 4px var(--shadow);
  /* 只过渡反馈需要的属性;box-shadow/背景过渡在滚动列表里是纯浪费 */
  transition: transform 0.15s ease;
  -webkit-tap-highlight-color: transparent;
  user-select: none;
  -webkit-user-select: none;
  /* 跳过视口外卡片的布局/绘制(近似虚拟化),auto 关键字记录上次实际高度 */
  contain: layout style paint;
  content-visibility: auto;
  contain-intrinsic-size: auto 96px;
}

.apps-list .app-card:active {
  transform: scale(0.98);
  box-shadow: 0 1px 3px var(--shadow);
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

.app-status-group {
  display: flex;
  align-items: center;
  gap: 0.35rem;
  flex-wrap: wrap;
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

.app-status.inherited {
  background: rgba(59, 130, 246, 0.12);
  color: #3b82f6;
}

.app-status.unconfigured {
  background: rgba(156, 163, 175, 0.1);
  color: var(--text-secondary);
}

.app-status.not-installed {
  background: rgba(239, 68, 68, 0.12);
  color: #ef4444;
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

.skeleton-item {
  pointer-events: none;
}

.skeleton-loader {
  background: linear-gradient(90deg, var(--border) 25%, var(--card-bg) 50%, var(--border) 75%);
  background-size: 200% 100%;
  animation: shimmer 1.5s infinite linear;
  border-radius: 0.25rem;
  opacity: 0.7;
}

.dark .skeleton-loader {
  background: linear-gradient(
    90deg,
    rgba(255, 255, 255, 0.05) 25%,
    rgba(255, 255, 255, 0.1) 50%,
    rgba(255, 255, 255, 0.05) 75%
  );
}

.icon-placeholder {
  position: absolute;
  top: 0;
  left: 0;
  width: 100%;
  height: 100%;
  border-radius: 0.75rem;
  z-index: 1;
}

.name-placeholder {
  height: 1rem;
  width: 60%;
  margin-bottom: 0.5rem;
}

.package-placeholder {
  height: 0.75rem;
  width: 40%;
  margin-bottom: 0.5rem;
}

.badge-placeholder {
  height: 1.25rem;
  width: 5rem;
  display: inline-block;
  margin-right: 0.5rem;
  border-radius: 0.25rem;
}

.badge-placeholder.short {
  width: 3rem;
}
</style>
