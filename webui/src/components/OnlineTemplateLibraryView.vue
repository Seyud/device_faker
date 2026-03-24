<template>
  <section class="online-library-view">
    <header class="library-hero">
      <button class="hero-back" type="button" @click="emit('close')">
        <ArrowLeft :size="18" />
        {{ t('templates.online.back') }}
      </button>

      <div class="hero-copy">
        <p class="hero-kicker">{{ t('templates.actions.online') }}</p>
        <h2 class="hero-title">{{ t('templates.online.title') }}</h2>
        <p class="hero-subtitle">{{ t('templates.online.subtitle') }}</p>
      </div>

      <div class="hero-actions">
        <button
          class="toolbar-btn"
          type="button"
          :disabled="onlineTemplatesStore.isRefreshing"
          @click="handleRefresh"
        >
          <RefreshCw :size="16" :class="{ 'spin-icon': onlineTemplatesStore.isRefreshing }" />
          {{ t('templates.online.refresh') }}
        </button>
        <button
          v-if="onlineTemplatesStore.failedDetailCount > 0"
          class="toolbar-btn toolbar-btn--ghost"
          type="button"
          :disabled="onlineTemplatesStore.detailsStatus === 'loading'"
          @click="onlineTemplatesStore.retryFailedDetails"
        >
          <RotateCw :size="16" />
          {{ t('templates.online.retry_failed') }}
        </button>
      </div>
    </header>

    <section class="control-panel">
      <label class="search-shell">
        <Search :size="18" class="search-icon" />
        <input
          v-model="searchInput"
          type="text"
          class="search-input"
          :placeholder="t('templates.online.search_placeholder')"
        />
      </label>

      <div class="chip-group">
        <button
          :class="['filter-chip', { active: onlineTemplatesStore.selectedCategory === 'all' }]"
          type="button"
          @click="onlineTemplatesStore.setSelectedCategory('all')"
        >
          {{ t('templates.categories.all') }}
        </button>
        <button
          v-for="(_, category) in onlineTemplatesStore.templateCategories"
          :key="category"
          :class="['filter-chip', { active: onlineTemplatesStore.selectedCategory === category }]"
          type="button"
          @click="onlineTemplatesStore.setSelectedCategory(category)"
        >
          {{ t(`templates.categories.${category}`) }}
        </button>
      </div>

      <div
        v-if="onlineTemplatesStore.availableBrands.length > 0"
        class="chip-group chip-group--brand"
      >
        <button
          :class="['brand-chip', { active: onlineTemplatesStore.selectedBrand === null }]"
          type="button"
          @click="onlineTemplatesStore.setSelectedBrand(null)"
        >
          {{ t('templates.categories.all') }}
        </button>
        <button
          v-for="brand in onlineTemplatesStore.availableBrands"
          :key="brand"
          :class="['brand-chip', { active: onlineTemplatesStore.selectedBrand === brand }]"
          type="button"
          @click="onlineTemplatesStore.setSelectedBrand(brand)"
        >
          {{ brand }}
        </button>
      </div>
    </section>

    <section class="status-strip">
      <div class="status-badges">
        <span class="source-badge">
          {{ t('templates.online.status.source', { source: currentSourceLabel }) }}
        </span>
        <span
          v-if="onlineTemplatesStore.isFallbackSource"
          class="source-badge source-badge--fallback"
        >
          {{ t('templates.online.status.fallback') }}
        </span>
        <span
          v-if="onlineTemplatesStore.showingCachedData"
          class="source-badge source-badge--cache"
        >
          {{ t('templates.online.status.cached') }}
        </span>
      </div>

      <div class="progress-shell">
        <div class="progress-copy">
          <strong>{{ t('templates.online.progress_label') }}</strong>
          <span>{{ progressCopy }}</span>
        </div>
        <div class="progress-track">
          <div class="progress-value" :style="{ width: `${progressPercent}%` }"></div>
        </div>
      </div>
    </section>

    <div v-if="onlineTemplatesStore.loadError" class="warning-banner">
      <AlertTriangle :size="18" />
      <p>{{ onlineTemplatesStore.loadError }}</p>
      <button class="inline-action" type="button" @click="onlineTemplatesStore.clearLoadError">
        {{ t('common.confirm') }}
      </button>
    </div>

    <div
      v-if="onlineTemplatesStore.indexStatus === 'loading' && !onlineTemplatesStore.hasAnyData"
      class="full-state"
    >
      <div class="state-card">
        <LoaderCircle :size="26" class="spin-icon" />
        <p>{{ t('templates.online.loading') }}</p>
        <div class="state-grid">
          <span v-for="index in 6" :key="index" class="state-skeleton"></span>
        </div>
      </div>
    </div>

    <div
      v-else-if="onlineTemplatesStore.indexStatus === 'error' && !onlineTemplatesStore.hasAnyData"
      class="full-state"
    >
      <div class="state-card state-card--error">
        <CircleAlert :size="28" />
        <p>{{ t('templates.online.errors.no_templates') }}</p>
        <button class="toolbar-btn" type="button" @click="handleRefresh">
          <RefreshCw :size="16" />
          {{ t('templates.online.refresh') }}
        </button>
      </div>
    </div>

    <div v-else-if="onlineTemplatesStore.filteredRecords.length === 0" class="full-state">
      <div class="state-card">
        <Database :size="26" />
        <p>{{ emptyCopy }}</p>
        <button
          v-if="hasActiveFilters"
          class="toolbar-btn toolbar-btn--ghost"
          type="button"
          @click="clearFilters"
        >
          <X :size="16" />
          {{ t('templates.online.clear_filters') }}
        </button>
      </div>
    </div>

    <div v-else class="result-shell">
      <OnlineTemplateVirtualList
        ref="virtualListRef"
        :items="onlineTemplatesStore.filteredRecords"
        :importing-ids="onlineTemplatesStore.importingIds"
        @retry="onlineTemplatesStore.retryTemplateDetail"
        @import="onlineTemplatesStore.importTemplate"
      />
    </div>
  </section>
</template>

<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue'
import {
  AlertTriangle,
  ArrowLeft,
  CircleAlert,
  Database,
  LoaderCircle,
  RefreshCw,
  RotateCw,
  Search,
  X,
} from 'lucide-vue-next'
import OnlineTemplateVirtualList from './OnlineTemplateVirtualList.vue'
import { useOnlineTemplatesStore } from '../stores/onlineTemplates'
import { useI18n } from '../utils/i18n'

type VirtualListExpose = {
  scrollToTop: () => void
}

const emit = defineEmits<{
  close: []
}>()

const onlineTemplatesStore = useOnlineTemplatesStore()
const { t } = useI18n()
const searchInput = ref(onlineTemplatesStore.keyword)
const virtualListRef = ref<VirtualListExpose | null>(null)

const currentSourceLabel = computed(() => {
  const source = onlineTemplatesStore.activeSource || onlineTemplatesStore.preferredSource
  return t(`templates.online.sources.${source}`)
})

const hasActiveFilters = computed(() => {
  return Boolean(
    searchInput.value.trim() ||
    onlineTemplatesStore.selectedBrand ||
    onlineTemplatesStore.selectedCategory !== 'all'
  )
})

const progressPercent = computed(() => {
  if (onlineTemplatesStore.progress.total === 0) {
    return 0
  }

  return Math.round(
    (onlineTemplatesStore.progress.resolved / onlineTemplatesStore.progress.total) * 100
  )
})

const progressCopy = computed(() => {
  if (onlineTemplatesStore.progress.total === 0) {
    return t('templates.online.progress_pending')
  }

  if (onlineTemplatesStore.allDetailsResolved) {
    return t('templates.online.progress_complete', {
      count: onlineTemplatesStore.progress.succeeded,
      total: onlineTemplatesStore.progress.total,
      failed: onlineTemplatesStore.progress.failed,
    })
  }

  return t('templates.online.progress_partial', {
    resolved: onlineTemplatesStore.progress.resolved,
    total: onlineTemplatesStore.progress.total,
    pending: onlineTemplatesStore.pendingDetailCount,
  })
})

const emptyCopy = computed(() => {
  if (hasActiveFilters.value) {
    return t('templates.online.empty_filtered')
  }

  return t('templates.online.empty_library')
})

function handleRefresh() {
  void onlineTemplatesStore.reloadCatalog()
}

function clearFilters() {
  searchInput.value = ''
  onlineTemplatesStore.setKeyword('')
  onlineTemplatesStore.setSelectedBrand(null)
  onlineTemplatesStore.setSelectedCategory('all')
}

watch(searchInput, (value) => {
  onlineTemplatesStore.setKeyword(value)
})

watch(
  () => [
    onlineTemplatesStore.keyword,
    onlineTemplatesStore.selectedCategory,
    onlineTemplatesStore.selectedBrand,
  ],
  () => {
    virtualListRef.value?.scrollToTop()
  }
)

onMounted(() => {
  void onlineTemplatesStore.ensureCatalogLoaded()
})
</script>

<style scoped>
.online-library-view {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  min-height: calc(100vh - 12rem);
}

.library-hero,
.control-panel,
.status-strip,
.warning-banner {
  border-radius: 1.25rem;
  border: 1px solid rgba(148, 163, 184, 0.2);
  background:
    linear-gradient(160deg, rgba(255, 255, 255, 0.96), rgba(244, 248, 255, 0.9)), var(--card);
  box-shadow: 0 20px 45px rgba(15, 23, 42, 0.07);
}

.dark .library-hero,
.dark .control-panel,
.dark .status-strip,
.dark .warning-banner {
  background: linear-gradient(160deg, rgba(15, 23, 42, 0.88), rgba(30, 41, 59, 0.82)), var(--card);
  border-color: rgba(148, 163, 184, 0.14);
  box-shadow: 0 22px 48px rgba(2, 6, 23, 0.34);
}

.library-hero {
  display: grid;
  grid-template-columns: auto 1fr auto;
  gap: 1rem;
  padding: 1.2rem;
  align-items: center;
}

.hero-back,
.toolbar-btn,
.inline-action {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.45rem;
  border-radius: 0.9rem;
  border: 1px solid transparent;
  cursor: pointer;
  font-size: 0.84rem;
  font-weight: 700;
  transition:
    transform 0.18s ease,
    opacity 0.18s ease,
    background 0.18s ease,
    border-color 0.18s ease;
}

.hero-back,
.toolbar-btn--ghost,
.inline-action {
  padding: 0.82rem 0.96rem;
  background: rgba(148, 163, 184, 0.14);
  color: var(--text);
  border-color: rgba(148, 163, 184, 0.2);
}

.toolbar-btn {
  padding: 0.82rem 1rem;
  background: linear-gradient(135deg, #075985, #0f766e);
  color: white;
  box-shadow: 0 16px 30px rgba(8, 145, 178, 0.22);
}

.toolbar-btn:disabled {
  opacity: 0.6;
  cursor: not-allowed;
  box-shadow: none;
}

.hero-back:hover,
.toolbar-btn:hover,
.inline-action:hover {
  transform: translateY(-1px);
}

.hero-copy {
  min-width: 0;
}

.hero-kicker {
  margin: 0 0 0.35rem;
  font-size: 0.75rem;
  font-weight: 800;
  letter-spacing: 0.12em;
  text-transform: uppercase;
  color: #0f766e;
}

.hero-title {
  margin: 0;
  font-size: 1.55rem;
  line-height: 1.15;
  color: var(--text);
}

.hero-subtitle {
  margin: 0.45rem 0 0;
  font-size: 0.92rem;
  line-height: 1.5;
  color: var(--text-secondary);
  max-width: 48rem;
}

.hero-actions {
  display: flex;
  gap: 0.7rem;
  flex-wrap: wrap;
  justify-content: flex-end;
}

.control-panel {
  display: flex;
  flex-direction: column;
  gap: 0.9rem;
  padding: 1rem;
}

.search-shell {
  display: flex;
  align-items: center;
  gap: 0.65rem;
  padding: 0.9rem 1rem;
  border-radius: 1rem;
  background: rgba(148, 163, 184, 0.08);
  border: 1px solid rgba(148, 163, 184, 0.16);
}

.search-icon {
  color: var(--text-secondary);
}

.search-input {
  width: 100%;
  border: none;
  background: transparent;
  color: var(--text);
  font-size: 0.92rem;
  outline: none;
}

.chip-group {
  display: flex;
  gap: 0.65rem;
  overflow-x: auto;
  padding-bottom: 0.15rem;
}

.filter-chip,
.brand-chip {
  flex-shrink: 0;
  padding: 0.65rem 0.9rem;
  border-radius: 999px;
  border: 1px solid rgba(148, 163, 184, 0.18);
  background: rgba(148, 163, 184, 0.1);
  color: var(--text);
  font-size: 0.82rem;
  font-weight: 700;
  cursor: pointer;
  transition:
    background 0.18s ease,
    color 0.18s ease,
    border-color 0.18s ease;
}

.filter-chip.active,
.brand-chip.active {
  background: linear-gradient(135deg, #0f766e, #0f9a8a);
  color: white;
  border-color: transparent;
}

.chip-group--brand .brand-chip.active {
  background: linear-gradient(135deg, #92400e, #b45309);
}

.status-strip {
  display: grid;
  grid-template-columns: auto 1fr;
  gap: 1rem;
  padding: 1rem;
  align-items: center;
}

.status-badges {
  display: flex;
  gap: 0.6rem;
  flex-wrap: wrap;
}

.source-badge {
  display: inline-flex;
  align-items: center;
  padding: 0.42rem 0.72rem;
  border-radius: 999px;
  background: rgba(14, 165, 233, 0.12);
  color: #0c4a6e;
  font-size: 0.78rem;
  font-weight: 700;
}

.source-badge--fallback {
  background: rgba(245, 158, 11, 0.14);
  color: #92400e;
}

.source-badge--cache {
  background: rgba(99, 102, 241, 0.12);
  color: #4338ca;
}

.dark .source-badge {
  color: #7dd3fc;
}

.dark .source-badge--fallback {
  color: #fcd34d;
}

.dark .source-badge--cache {
  color: #c4b5fd;
}

.progress-shell {
  min-width: 0;
}

.progress-copy {
  display: flex;
  justify-content: space-between;
  gap: 0.75rem;
  margin-bottom: 0.55rem;
  font-size: 0.82rem;
  color: var(--text-secondary);
}

.progress-track {
  height: 0.65rem;
  border-radius: 999px;
  background: rgba(148, 163, 184, 0.16);
  overflow: hidden;
}

.progress-value {
  height: 100%;
  border-radius: inherit;
  background: linear-gradient(90deg, #0f766e, #14b8a6);
  transition: width 0.25s ease;
}

.warning-banner {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  padding: 0.95rem 1rem;
  color: #b45309;
}

.warning-banner p {
  flex: 1;
  margin: 0;
  font-size: 0.84rem;
  line-height: 1.45;
}

.full-state,
.result-shell {
  min-height: 26rem;
}

.result-shell {
  height: clamp(28rem, calc(100vh - 24rem), 48rem);
}

.state-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.9rem;
  justify-content: center;
  min-height: 26rem;
  padding: 2rem;
  border-radius: 1.25rem;
  border: 1px dashed rgba(148, 163, 184, 0.28);
  background: rgba(148, 163, 184, 0.06);
  text-align: center;
  color: var(--text-secondary);
}

.state-card--error {
  color: #dc2626;
}

.state-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.75rem;
  width: min(100%, 24rem);
}

.state-skeleton {
  display: block;
  height: 1rem;
  border-radius: 999px;
  background: linear-gradient(
    90deg,
    rgba(148, 163, 184, 0.14),
    rgba(148, 163, 184, 0.26),
    rgba(148, 163, 184, 0.14)
  );
  background-size: 220% 100%;
  animation: pulse-slide 1.4s linear infinite;
}

.spin-icon {
  animation: rotate 0.9s linear infinite;
}

@keyframes rotate {
  from {
    transform: rotate(0deg);
  }

  to {
    transform: rotate(360deg);
  }
}

@keyframes pulse-slide {
  from {
    background-position: 200% 0;
  }

  to {
    background-position: -20% 0;
  }
}

@media (max-width: 760px) {
  .library-hero {
    grid-template-columns: 1fr;
  }

  .hero-actions {
    justify-content: stretch;
  }

  .hero-actions .toolbar-btn,
  .hero-actions .toolbar-btn--ghost {
    width: 100%;
  }

  .status-strip {
    grid-template-columns: 1fr;
  }
}
</style>
