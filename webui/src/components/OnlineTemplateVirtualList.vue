<template>
  <div ref="viewportRef" class="virtual-list" @scroll="handleScroll">
    <div class="virtual-spacer" :style="{ height: `${totalHeight}px` }">
      <article
        v-for="entry in visibleEntries"
        :key="entry.item.id"
        class="virtual-row"
        :style="{ top: `${entry.top}px`, height: `${entry.height}px` }"
      >
        <div class="template-card">
          <div class="card-head">
            <div class="card-head-top">
              <div class="card-title-group">
                <h3 class="card-title">{{ entry.item.displayName }}</h3>
                <p class="card-subtitle">
                  {{ t(`templates.categories.${entry.item.category}`) }}
                  <span v-if="entry.item.brand">· {{ entry.item.brand }}</span>
                </p>
              </div>
              <span :class="['status-pill', `status-pill--${entry.item.detailStatus}`]">
                {{ getStatusLabel(entry.item.detailStatus) }}
              </span>
            </div>
            <p
              v-if="
                entry.item.detailStatus === 'ready' &&
                entry.item.detail &&
                getSummaryText(entry.item.detail).length > 0
              "
              class="card-summary"
            >
              {{ getSummaryText(entry.item.detail) }}
            </p>
          </div>

          <div
            v-if="entry.item.detailStatus === 'ready' && entry.item.detail"
            class="card-body card-body--ready"
          >
            <div
              v-if="
                entry.item.detail.meta?.author ||
                entry.item.detail.meta?.description ||
                entry.item.detail.meta?.version ||
                entry.item.detail.meta?.version_code
              "
              class="meta-panel"
            >
              <div v-if="entry.item.detail.meta?.author" class="meta-field">
                <span class="detail-label">{{ t('templates.labels.author') }}</span>
                <p class="meta-text meta-text--author">
                  {{ entry.item.detail.meta.author }}
                </p>
              </div>

              <div v-if="entry.item.detail.meta?.description" class="meta-field">
                <span class="detail-label">{{ t('templates.labels.description') }}</span>
                <p class="meta-text meta-text--description">
                  {{ entry.item.detail.meta.description }}
                </p>
              </div>

              <div class="meta-row">
                <span v-if="entry.item.detail.meta?.version" class="meta-pill">
                  {{ t('templates.labels.version') }} {{ entry.item.detail.meta.version }}
                </span>
                <span v-if="entry.item.detail.meta?.version_code" class="meta-pill">
                  {{ t('templates.labels.version_code') }} {{ entry.item.detail.meta.version_code }}
                </span>
              </div>
            </div>
          </div>

          <div v-else-if="entry.item.detailStatus === 'error'" class="card-body card-body--error">
            <AlertTriangle :size="18" />
            <p class="error-copy">
              {{ entry.item.detailError || t('templates.online.errors.detail_failed') }}
            </p>
            <button class="inline-btn" type="button" @click="emit('retry', entry.item.id)">
              <RefreshCw :size="14" />
              {{ t('templates.online.retry_detail') }}
            </button>
          </div>

          <div v-else class="card-body card-body--loading">
            <div class="skeleton-line skeleton-line--lg"></div>
            <div class="skeleton-grid">
              <div class="skeleton-line"></div>
              <div class="skeleton-line"></div>
              <div class="skeleton-line"></div>
              <div class="skeleton-line"></div>
            </div>
            <div class="skeleton-line skeleton-line--sm"></div>
          </div>

          <div class="card-actions">
            <button
              v-if="entry.item.detailStatus === 'error'"
              class="action-btn action-btn--ghost"
              type="button"
              @click="emit('retry', entry.item.id)"
            >
              <RefreshCw :size="16" />
              {{ t('templates.online.retry_detail') }}
            </button>
            <button
              class="action-btn action-btn--primary"
              type="button"
              :disabled="
                entry.item.detailStatus !== 'ready' || importingIds.includes(entry.item.id)
              "
              @click="emit('import', entry.item.id)"
            >
              <LoaderCircle
                v-if="importingIds.includes(entry.item.id)"
                :size="16"
                class="spin-icon"
              />
              <Download v-else :size="16" />
              {{
                importingIds.includes(entry.item.id)
                  ? t('templates.online.importing')
                  : t('templates.online.import')
              }}
            </button>
          </div>
        </div>
      </article>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed, onMounted, onUnmounted, ref, watch } from 'vue'
import { AlertTriangle, Download, LoaderCircle, RefreshCw } from 'lucide-vue-next'
import type { OnlineTemplateDetail, OnlineTemplateRecord, OnlineTemplateLoadState } from '../types'
import { useI18n } from '../utils/i18n'

const DEFAULT_OVERSCAN = 5
const ROW_GAP = 12

const props = defineProps<{
  items: OnlineTemplateRecord[]
  importingIds: string[]
  itemHeight?: number
  overscan?: number
}>()

const emit = defineEmits<{
  retry: [string]
  import: [string]
}>()

const { t } = useI18n()
const viewportRef = ref<HTMLElement | null>(null)
const viewportHeight = ref(0)
const viewportWidth = ref(0)
const scrollTop = ref(0)
let cleanupResizeObserver: (() => void) | null = null

const overscan = computed(() => props.overscan ?? DEFAULT_OVERSCAN)

function estimateLineCount(text: string, charsPerLine: number, clamp?: number) {
  const trimmed = text.trim()
  if (!trimmed) {
    return 0
  }

  const lines = Math.max(1, Math.ceil(trimmed.length / charsPerLine))
  return clamp ? Math.min(lines, clamp) : lines
}

function estimateCardHeight(item: OnlineTemplateRecord) {
  if (props.itemHeight) {
    return props.itemHeight
  }

  const isMobile = viewportWidth.value <= 640
  const titleLines = estimateLineCount(item.displayName, isMobile ? 20 : 28, 2)
  const headerTitleHeight = titleLines * 24
  const subtitleHeight = 20
  const summaryText =
    item.detailStatus === 'ready' && item.detail ? getSummaryText(item.detail) : ''
  const summaryLines = estimateLineCount(summaryText, isMobile ? 22 : 34, isMobile ? 3 : 2)
  const summaryHeight = summaryLines > 0 ? summaryLines * 18 + 8 : 0
  const headerHeight = headerTitleHeight + subtitleHeight + summaryHeight
  const actionHeight = isMobile ? 52 : 44

  if (item.detailStatus === 'ready' && item.detail) {
    const meta = item.detail.meta
    const hasAuthor = Boolean(meta?.author)
    const hasDescription = Boolean(meta?.description)
    const hasVersion = Boolean(meta?.version || meta?.version_code)

    let metaHeight = 0
    if (hasAuthor || hasDescription || hasVersion) {
      metaHeight += 28

      if (hasAuthor) {
        metaHeight += 46
      }

      if (hasDescription && meta?.description) {
        metaHeight += 22
        metaHeight += estimateLineCount(meta.description, isMobile ? 24 : 38) * 18
      }

      if (hasVersion) {
        metaHeight += 34
      }

      metaHeight += 18
    }

    return 32 + headerHeight + metaHeight + actionHeight
  }

  return 32 + headerHeight + 92 + actionHeight
}

type VirtualEntry = {
  item: OnlineTemplateRecord
  top: number
  height: number
  bottom: number
}

const itemMetrics = computed<VirtualEntry[]>(() => {
  let offset = 0

  return props.items.map((item) => {
    const height = estimateCardHeight(item)
    const entry = {
      item,
      top: offset,
      height,
      bottom: offset + height + ROW_GAP,
    }
    offset = entry.bottom
    return entry
  })
})

const totalHeight = computed(() => {
  const last = itemMetrics.value[itemMetrics.value.length - 1]
  return last ? last.top + last.height : 0
})

function findIndexByOffset(offset: number) {
  let low = 0
  let high = itemMetrics.value.length - 1
  let answer = itemMetrics.value.length

  while (low <= high) {
    const mid = Math.floor((low + high) / 2)
    if (itemMetrics.value[mid].bottom > offset) {
      answer = mid
      high = mid - 1
    } else {
      low = mid + 1
    }
  }

  return answer === itemMetrics.value.length ? Math.max(0, answer - 1) : answer
}

const startIndex = computed(() => {
  return Math.max(0, findIndexByOffset(scrollTop.value) - overscan.value)
})

const endIndex = computed(() => {
  const endOffset = scrollTop.value + viewportHeight.value
  return Math.min(itemMetrics.value.length, findIndexByOffset(endOffset) + overscan.value + 1)
})

const visibleEntries = computed(() => itemMetrics.value.slice(startIndex.value, endIndex.value))

function handleScroll(event: Event) {
  const target = event.target as HTMLElement
  scrollTop.value = target.scrollTop
}

function syncViewportHeight() {
  viewportHeight.value = viewportRef.value?.clientHeight || 0
  viewportWidth.value = viewportRef.value?.clientWidth || 0
}

function scrollToTop() {
  if (!viewportRef.value) {
    return
  }

  viewportRef.value.scrollTop = 0
  scrollTop.value = 0
}

function getStatusLabel(status: OnlineTemplateLoadState) {
  switch (status) {
    case 'ready':
      return t('templates.online.status.ready')
    case 'error':
      return t('templates.online.status.failed')
    case 'loading':
      return t('templates.online.status.loading')
    default:
      return t('templates.online.status.pending')
  }
}

function getSummaryText(detail: OnlineTemplateDetail) {
  const parts = [detail.template.model, detail.template.marketname, detail.template.device].filter(
    (value): value is string => Boolean(value && value.trim())
  )

  return parts.join(' · ')
}

watch(totalHeight, () => {
  const maxScrollTop = Math.max(0, totalHeight.value - viewportHeight.value)
  if (scrollTop.value > maxScrollTop && viewportRef.value) {
    viewportRef.value.scrollTop = maxScrollTop
    scrollTop.value = maxScrollTop
  }
})

onMounted(() => {
  syncViewportHeight()

  if (viewportRef.value && typeof window.ResizeObserver !== 'undefined') {
    const observer = new window.ResizeObserver(() => {
      syncViewportHeight()
    })
    observer.observe(viewportRef.value)
    cleanupResizeObserver = () => observer.disconnect()
  }
})

onUnmounted(() => {
  cleanupResizeObserver?.()
  cleanupResizeObserver = null
})

defineExpose({
  scrollToTop,
})
</script>

<style scoped>
.virtual-list {
  position: relative;
  height: 100%;
  overflow-y: auto;
  padding-right: 0.25rem;
  scrollbar-width: thin;
}

.virtual-spacer {
  position: relative;
  width: 100%;
}

.virtual-row {
  position: absolute;
  left: 0;
  right: 0;
}

.template-card {
  display: flex;
  flex-direction: column;
  gap: 0.875rem;
  height: 100%;
  padding: 1rem;
  border-radius: 1rem;
  background:
    linear-gradient(145deg, rgba(255, 255, 255, 0.94), rgba(247, 250, 255, 0.88)), var(--card);
  border: 1px solid rgba(148, 163, 184, 0.22);
  box-shadow: 0 18px 40px rgba(15, 23, 42, 0.08);
}

.dark .template-card {
  background: linear-gradient(145deg, rgba(15, 23, 42, 0.88), rgba(30, 41, 59, 0.78)), var(--card);
  border-color: rgba(148, 163, 184, 0.16);
  box-shadow: 0 20px 44px rgba(2, 6, 23, 0.42);
}

.card-head,
.card-actions {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.75rem;
}

.card-head {
  flex-direction: column;
}

.card-head-top {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.75rem;
  width: 100%;
}

.card-title-group {
  min-width: 0;
}

.card-title {
  margin: 0;
  font-size: 1rem;
  font-weight: 700;
  color: var(--text);
  word-break: break-word;
}

.card-subtitle {
  margin: 0.35rem 0 0;
  font-size: 0.8rem;
  color: var(--text-secondary);
}

.card-summary {
  margin: 0;
  font-size: 0.82rem;
  line-height: 1.45;
  color: var(--text);
  display: -webkit-box;
  -webkit-box-orient: vertical;
  -webkit-line-clamp: 2;
  overflow: hidden;
}

.status-pill {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  min-width: 4.75rem;
  padding: 0.375rem 0.625rem;
  border-radius: 999px;
  font-size: 0.72rem;
  font-weight: 700;
  letter-spacing: 0.02em;
  white-space: nowrap;
}

.status-pill--idle,
.status-pill--loading {
  background: rgba(14, 165, 233, 0.12);
  color: #0369a1;
}

.status-pill--ready {
  background: rgba(16, 185, 129, 0.16);
  color: #047857;
}

.status-pill--error {
  background: rgba(239, 68, 68, 0.12);
  color: #b91c1c;
}

.dark .status-pill--idle,
.dark .status-pill--loading {
  color: #7dd3fc;
}

.dark .status-pill--ready {
  color: #6ee7b7;
}

.dark .status-pill--error {
  color: #fda4af;
}

.card-body {
  min-height: 0;
}

.card-body--ready {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.detail-label {
  display: block;
  font-size: 0.72rem;
  font-weight: 600;
  color: var(--text-secondary);
  margin-bottom: 0.28rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}

.detail-value {
  display: block;
  font-size: 0.9rem;
  color: var(--text);
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.meta-panel {
  display: flex;
  flex-direction: column;
  gap: 0.65rem;
  padding: 0.8rem 0.9rem;
  border-radius: 0.9rem;
  background: rgba(148, 163, 184, 0.1);
  border: 1px solid rgba(148, 163, 184, 0.14);
}

.dark .meta-panel {
  background: rgba(51, 65, 85, 0.28);
  border-color: rgba(148, 163, 184, 0.12);
}

.meta-field {
  min-width: 0;
}

.meta-text {
  margin: 0;
  color: var(--text);
  line-height: 1.45;
}

.meta-text--author {
  font-size: 0.84rem;
  font-weight: 600;
  word-break: break-word;
}

.meta-text--description {
  font-size: 0.81rem;
  color: var(--text-secondary);
  white-space: pre-wrap;
  word-break: break-word;
}

.meta-row {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.meta-pill {
  display: inline-flex;
  align-items: center;
  padding: 0.28rem 0.55rem;
  border-radius: 999px;
  background: rgba(99, 102, 241, 0.1);
  color: var(--text);
  font-size: 0.72rem;
  font-weight: 600;
}

.card-body--error {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 0.6rem;
  justify-content: center;
  color: #dc2626;
}

.error-copy {
  margin: 0;
  font-size: 0.84rem;
  line-height: 1.45;
}

.inline-btn,
.action-btn {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  gap: 0.45rem;
  padding: 0.72rem 0.95rem;
  border-radius: 0.875rem;
  border: 1px solid transparent;
  font-size: 0.84rem;
  font-weight: 700;
  cursor: pointer;
  transition:
    transform 0.18s ease,
    box-shadow 0.18s ease,
    background 0.18s ease,
    border-color 0.18s ease,
    opacity 0.18s ease;
}

.inline-btn,
.action-btn--ghost {
  background: rgba(148, 163, 184, 0.12);
  color: var(--text);
  border-color: rgba(148, 163, 184, 0.22);
}

.action-btn--primary {
  background: linear-gradient(135deg, #0f766e, #0f9a8a);
  color: white;
  box-shadow: 0 16px 28px rgba(15, 118, 110, 0.22);
}

.action-btn--primary:disabled {
  cursor: not-allowed;
  opacity: 0.55;
  box-shadow: none;
}

.action-btn--ghost:hover,
.action-btn--primary:hover,
.inline-btn:hover {
  transform: translateY(-1px);
}

.card-actions {
  align-items: center;
}

.card-body--loading {
  display: flex;
  flex-direction: column;
  gap: 0.7rem;
  justify-content: center;
}

.skeleton-grid {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 0.6rem;
}

.skeleton-line {
  height: 0.8rem;
  border-radius: 999px;
  background: linear-gradient(
    90deg,
    rgba(148, 163, 184, 0.16),
    rgba(148, 163, 184, 0.28),
    rgba(148, 163, 184, 0.16)
  );
  background-size: 220% 100%;
  animation: pulse-slide 1.4s linear infinite;
}

.skeleton-line--lg {
  width: 52%;
  height: 1rem;
}

.skeleton-line--sm {
  width: 72%;
}

.spin-icon {
  animation: rotate 0.9s linear infinite;
}

@keyframes pulse-slide {
  from {
    background-position: 200% 0;
  }

  to {
    background-position: -20% 0;
  }
}

@keyframes rotate {
  from {
    transform: rotate(0deg);
  }

  to {
    transform: rotate(360deg);
  }
}

@media (max-width: 640px) {
  .card-summary {
    -webkit-line-clamp: 3;
  }

  .card-actions {
    flex-direction: column;
    align-items: stretch;
  }

  .action-btn {
    width: 100%;
  }
}
</style>
