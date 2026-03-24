import { defineStore } from 'pinia'
import { computed, ref, shallowRef } from 'vue'
import { toast } from 'kernelsu-alt'
import type {
  OnlineTemplateCacheEntry,
  OnlineTemplateDetail,
  OnlineTemplateDetailState,
  OnlineTemplateDetailsState,
  OnlineTemplateIndexItem,
  OnlineTemplateLoadSession,
  OnlineTemplateLoadState,
  OnlineTemplateProgress,
  OnlineTemplateRecord,
  OnlineTemplateSource,
  TemplateCategoryFilter,
} from '../types'
import { useConfigStore } from './config'
import { useSettingsStore } from './settings'
import {
  TEMPLATE_CATEGORIES,
  loadTemplateDetails,
  loadTemplateIndex,
  type TemplateDetailLoadResult,
} from '../utils/onlineTemplates'
import { useLazyMessageBox } from '../utils/elementPlus'
import { useI18n } from '../utils/i18n'

const CACHE_SCHEMA_VERSION = 2
const SNAPSHOT_CACHE_KEY = 'device_faker_online_templates_snapshot_v2'
const DETAIL_CACHE_PREFIX = 'device_faker_online_templates_details_v2'
const NETWORK_REFRESH_INTERVAL_MS = 2 * 60 * 1000
const DETAIL_CACHE_TTL_MS = 12 * 60 * 60 * 1000

interface CatalogSnapshot {
  preferredSource: OnlineTemplateSource
  resolvedSource: OnlineTemplateSource
  items: OnlineTemplateIndexItem[]
}

type PersistedDetailCache = Record<string, OnlineTemplateCacheEntry<OnlineTemplateDetail>>

function isRecord(value: unknown): value is Record<string, unknown> {
  return Boolean(value) && typeof value === 'object' && !Array.isArray(value)
}

function isAbortError(error: unknown): boolean {
  return (
    (error instanceof DOMException && error.name === 'AbortError') ||
    (error instanceof Error && error.name === 'AbortError')
  )
}

function readStorage<T>(key: string): T | null {
  if (typeof localStorage === 'undefined') {
    return null
  }

  try {
    const raw = localStorage.getItem(key)
    if (!raw) {
      return null
    }

    return JSON.parse(raw) as T
  } catch {
    return null
  }
}

function writeStorage<T>(key: string, value: T) {
  if (typeof localStorage === 'undefined') {
    return
  }

  try {
    localStorage.setItem(key, JSON.stringify(value))
  } catch {
    // Ignore cache write failures caused by storage quota or private mode.
  }
}

function removeStorage(key: string) {
  if (typeof localStorage === 'undefined') {
    return
  }

  localStorage.removeItem(key)
}

function createIdleProgress(): OnlineTemplateProgress {
  return {
    total: 0,
    resolved: 0,
    succeeded: 0,
    failed: 0,
  }
}

function createIdleDetailState(): OnlineTemplateDetailState {
  return {
    status: 'idle',
    error: null,
  }
}

function isReadyState(
  state: OnlineTemplateDetailState | undefined
): state is OnlineTemplateDetailState & { detail: OnlineTemplateDetail } {
  return state?.status === 'ready' && Boolean(state.detail)
}

function isExpired(entry: OnlineTemplateCacheEntry<unknown>): boolean {
  return entry.expiresAt <= Date.now()
}

function detailCacheKey(source: OnlineTemplateSource): string {
  return `${DETAIL_CACHE_PREFIX}:${source}`
}

export const useOnlineTemplatesStore = defineStore('online-templates', () => {
  const configStore = useConfigStore()
  const settingsStore = useSettingsStore()
  const { t } = useI18n()
  const getMessageBox = useLazyMessageBox()

  const libraryOpen = ref(false)
  const keyword = ref('')
  const selectedCategory = ref<TemplateCategoryFilter>('all')
  const selectedBrand = ref<string | null>(null)
  const indexStatus = ref<OnlineTemplateLoadState>('idle')
  const detailsStatus = ref<OnlineTemplateDetailsState>('idle')
  const isRefreshing = ref(false)
  const loadError = ref<string | null>(null)
  const activeSource = ref<OnlineTemplateSource | null>(null)
  const session = ref<OnlineTemplateLoadSession | null>(null)
  const progress = ref<OnlineTemplateProgress>(createIdleProgress())
  const lastSuccessfulLoadAt = ref<number | null>(null)
  const showingCachedData = ref(false)
  const importingIds = ref<string[]>([])
  const indexItems = shallowRef<OnlineTemplateIndexItem[]>([])
  const indexById = shallowRef<Record<string, OnlineTemplateIndexItem>>({})
  const detailStateById = shallowRef<Record<string, OnlineTemplateDetailState>>({})

  let currentLoadPromise: Promise<void> | null = null
  let currentDetailsPromise: Promise<void> | null = null
  let indexController: AbortController | null = null
  let detailsController: AbortController | null = null
  let persistTimer: number | null = null
  let persistSource: OnlineTemplateSource | null = null
  let sessionSeed = 0

  const preferredSource = computed(() => settingsStore.onlineTemplateSource)
  const isFallbackSource = computed(() =>
    Boolean(activeSource.value && activeSource.value !== preferredSource.value)
  )

  const filteredIndexItems = computed(() => {
    const needle = keyword.value.trim().toLowerCase()

    return indexItems.value.filter((item) => {
      if (selectedCategory.value !== 'all' && item.category !== selectedCategory.value) {
        return false
      }

      if (selectedBrand.value && item.brand !== selectedBrand.value) {
        return false
      }

      if (!needle) {
        return true
      }

      const fields = [item.name, item.displayName, item.brand || '', item.category]
      return fields.some((field) => field.toLowerCase().includes(needle))
    })
  })

  const availableBrands = computed(() => {
    const brands = new Set<string>()

    for (const item of indexItems.value) {
      if (selectedCategory.value !== 'all' && item.category !== selectedCategory.value) {
        continue
      }

      if (item.brand) {
        brands.add(item.brand)
      }
    }

    return [...brands].sort((left, right) =>
      left.localeCompare(right, undefined, { sensitivity: 'base' })
    )
  })

  const filteredRecords = computed<OnlineTemplateRecord[]>(() => {
    return filteredIndexItems.value.map((item) => {
      const detailState = detailStateById.value[item.id]
      return {
        ...item,
        detailStatus: detailState?.status || 'idle',
        detail: detailState?.detail,
        detailError: detailState?.error || null,
      }
    })
  })

  const hasAnyData = computed(() => indexItems.value.length > 0)
  const failedDetailCount = computed(() => progress.value.failed)
  const pendingDetailCount = computed(() =>
    Math.max(0, progress.value.total - progress.value.resolved)
  )
  const allDetailsResolved = computed(
    () => progress.value.total > 0 && progress.value.resolved === progress.value.total
  )

  function setKeyword(value: string) {
    keyword.value = value
  }

  function setSelectedCategory(category: TemplateCategoryFilter) {
    selectedCategory.value = category
    if (selectedBrand.value && !availableBrands.value.includes(selectedBrand.value)) {
      selectedBrand.value = null
    }
  }

  function setSelectedBrand(brand: string | null) {
    selectedBrand.value = brand
  }

  function openLibrary() {
    libraryOpen.value = true
  }

  function closeLibrary() {
    libraryOpen.value = false
  }

  function markImporting(id: string, importing: boolean) {
    const next = [...importingIds.value]
    const index = next.indexOf(id)

    if (importing && index === -1) {
      next.push(id)
    }

    if (!importing && index !== -1) {
      next.splice(index, 1)
    }

    importingIds.value = next
  }

  function isImporting(id: string) {
    return importingIds.value.includes(id)
  }

  function readSnapshotCache(): OnlineTemplateCacheEntry<CatalogSnapshot> | null {
    const entry = readStorage<OnlineTemplateCacheEntry<CatalogSnapshot>>(SNAPSHOT_CACHE_KEY)
    if (!entry || entry.schemaVersion !== CACHE_SCHEMA_VERSION || !isRecord(entry.data)) {
      return null
    }

    if (!Array.isArray(entry.data.items) || entry.data.items.length === 0) {
      return null
    }

    return entry
  }

  function writeSnapshotCache(
    resolvedSource: OnlineTemplateSource,
    items: OnlineTemplateIndexItem[]
  ) {
    const entry: OnlineTemplateCacheEntry<CatalogSnapshot> = {
      schemaVersion: CACHE_SCHEMA_VERSION,
      createdAt: Date.now(),
      expiresAt: Date.now() + DETAIL_CACHE_TTL_MS,
      data: {
        preferredSource: preferredSource.value,
        resolvedSource,
        items,
      },
      version: resolvedSource,
    }

    writeStorage(SNAPSHOT_CACHE_KEY, entry)
  }

  function readDetailCache(source: OnlineTemplateSource): PersistedDetailCache {
    const entry = readStorage<PersistedDetailCache>(detailCacheKey(source))
    if (!entry || !isRecord(entry)) {
      return {}
    }

    return entry
  }

  function queueDetailCachePersist(source: OnlineTemplateSource) {
    persistSource = source

    if (persistTimer !== null) {
      window.clearTimeout(persistTimer)
    }

    persistTimer = window.setTimeout(() => {
      persistTimer = null
      persistReadyDetails(persistSource)
    }, 180)
  }

  function persistReadyDetails(source: OnlineTemplateSource | null) {
    if (!source) {
      return
    }

    const cache: PersistedDetailCache = {}

    for (const item of indexItems.value) {
      const state = detailStateById.value[item.id]
      if (!isReadyState(state)) {
        continue
      }

      const timestamp = state.updatedAt || Date.now()
      cache[item.id] = {
        schemaVersion: CACHE_SCHEMA_VERSION,
        createdAt: timestamp,
        expiresAt: timestamp + DETAIL_CACHE_TTL_MS,
        data: state.detail,
        version: state.version,
      }
    }

    if (Object.keys(cache).length === 0) {
      removeStorage(detailCacheKey(source))
      return
    }

    writeStorage(detailCacheKey(source), cache)
  }

  function buildIndexMap(
    items: OnlineTemplateIndexItem[]
  ): Record<string, OnlineTemplateIndexItem> {
    return items.reduce<Record<string, OnlineTemplateIndexItem>>((result, item) => {
      result[item.id] = item
      return result
    }, {})
  }

  function buildSeededDetailState(
    items: OnlineTemplateIndexItem[],
    resolvedSource: OnlineTemplateSource
  ): Record<string, OnlineTemplateDetailState> {
    const next: Record<string, OnlineTemplateDetailState> = {}
    const cache = readDetailCache(resolvedSource)

    for (const item of items) {
      const currentState = detailStateById.value[item.id]
      if (isReadyState(currentState) && (!item.sha || currentState.version === item.sha)) {
        next[item.id] = currentState
        continue
      }

      const cachedState = cache[item.id]
      if (
        cachedState &&
        cachedState.schemaVersion === CACHE_SCHEMA_VERSION &&
        ((item.sha && cachedState.version === item.sha) || (!item.sha && !isExpired(cachedState)))
      ) {
        next[item.id] = {
          status: 'ready',
          detail: cachedState.data,
          error: null,
          updatedAt: cachedState.createdAt,
          version: cachedState.version,
        }
        continue
      }

      next[item.id] = createIdleDetailState()
    }

    return next
  }

  function recomputeProgress() {
    const nextProgress = createIdleProgress()
    nextProgress.total = indexItems.value.length

    for (const item of indexItems.value) {
      const state = detailStateById.value[item.id]
      if (!state) {
        continue
      }

      if (state.status === 'ready') {
        nextProgress.resolved += 1
        nextProgress.succeeded += 1
      } else if (state.status === 'error') {
        nextProgress.resolved += 1
        nextProgress.failed += 1
      }
    }

    progress.value = nextProgress
  }

  function updateDetailsStatus() {
    if (indexItems.value.length === 0) {
      detailsStatus.value = 'idle'
      return
    }

    if (progress.value.resolved === progress.value.total) {
      detailsStatus.value =
        progress.value.failed === progress.value.total && progress.value.total > 0
          ? 'error'
          : 'complete'
      return
    }

    detailsStatus.value = progress.value.resolved > 0 ? 'partial' : 'loading'
  }

  function applyDetailChunk(
    results: TemplateDetailLoadResult[],
    resolvedSource: OnlineTemplateSource
  ) {
    if (results.length === 0) {
      return
    }

    const next = { ...detailStateById.value }
    const timestamp = Date.now()

    for (const result of results) {
      next[result.id] = result.detail
        ? {
            status: 'ready',
            detail: result.detail,
            error: null,
            updatedAt: timestamp,
            version: result.version,
          }
        : {
            status: 'error',
            error: result.error || t('templates.online.errors.detail_failed'),
            updatedAt: timestamp,
            version: result.version,
          }
    }

    detailStateById.value = next
    recomputeProgress()
    updateDetailsStatus()
    queueDetailCachePersist(resolvedSource)
  }

  function getPendingDetailItems(items: OnlineTemplateIndexItem[] = indexItems.value) {
    return items.filter((item) => {
      return detailStateById.value[item.id]?.status !== 'ready'
    })
  }

  async function loadDetailBatch(
    items: OnlineTemplateIndexItem[],
    resolvedSource: OnlineTemplateSource
  ) {
    if (items.length === 0) {
      recomputeProgress()
      updateDetailsStatus()
      return
    }

    if (currentDetailsPromise) {
      return currentDetailsPromise
    }

    const currentSessionId = session.value?.id
    const nextStates = { ...detailStateById.value }
    for (const item of items) {
      nextStates[item.id] = {
        ...nextStates[item.id],
        status: 'loading',
        error: null,
      }
    }

    detailStateById.value = nextStates
    recomputeProgress()
    updateDetailsStatus()

    detailsController?.abort()
    detailsController = new AbortController()

    currentDetailsPromise = (async () => {
      try {
        await loadTemplateDetails(items, {
          signal: detailsController?.signal,
          chunkSize: 4,
          onChunk: (results) => {
            if (session.value?.id !== currentSessionId) {
              return
            }

            applyDetailChunk(results, resolvedSource)
          },
        })
      } catch (error) {
        if (!isAbortError(error)) {
          throw error
        }
      } finally {
        if (session.value?.id === currentSessionId) {
          detailsController = null
          currentDetailsPromise = null
          recomputeProgress()
          updateDetailsStatus()
        }
      }
    })()

    return currentDetailsPromise
  }

  function hydrateFromSnapshot(): boolean {
    if (indexItems.value.length > 0) {
      return true
    }

    const snapshot = readSnapshotCache()
    if (!snapshot) {
      return false
    }

    indexItems.value = snapshot.data.items
    indexById.value = buildIndexMap(snapshot.data.items)
    activeSource.value = snapshot.data.resolvedSource
    detailStateById.value = buildSeededDetailState(
      snapshot.data.items,
      snapshot.data.resolvedSource
    )
    showingCachedData.value = true
    indexStatus.value = 'ready'
    loadError.value = null
    recomputeProgress()
    updateDetailsStatus()
    return true
  }

  function resetBrandFilterIfNeeded() {
    if (selectedBrand.value && !availableBrands.value.includes(selectedBrand.value)) {
      selectedBrand.value = null
    }
  }

  function cancelActiveRequests() {
    indexController?.abort()
    detailsController?.abort()
    indexController = null
    detailsController = null
    currentLoadPromise = null
    currentDetailsPromise = null
  }

  async function refreshCatalog(options: { background?: boolean } = {}) {
    if (currentLoadPromise) {
      return currentLoadPromise
    }

    const hasCachedIndex = indexItems.value.length > 0
    const sessionId = ++sessionSeed
    session.value = {
      id: sessionId,
      preferredSource: preferredSource.value,
      startedAt: Date.now(),
    }

    loadError.value = null
    isRefreshing.value = options.background || hasCachedIndex
    if (!hasCachedIndex) {
      indexStatus.value = 'loading'
      detailsStatus.value = 'idle'
    }

    cancelActiveRequests()
    indexController = new AbortController()

    currentLoadPromise = (async () => {
      try {
        const result = await loadTemplateIndex(preferredSource.value, indexController?.signal)
        if (session.value?.id !== sessionId) {
          return
        }

        session.value = {
          ...session.value,
          resolvedSource: result.source,
        }

        activeSource.value = result.source
        indexItems.value = result.items
        indexById.value = buildIndexMap(result.items)
        detailStateById.value = buildSeededDetailState(result.items, result.source)
        lastSuccessfulLoadAt.value = Date.now()
        showingCachedData.value = false
        indexStatus.value = 'ready'
        recomputeProgress()
        updateDetailsStatus()
        resetBrandFilterIfNeeded()
        writeSnapshotCache(result.source, result.items)
        queueDetailCachePersist(result.source)

        const pendingItems = getPendingDetailItems(result.items)
        if (pendingItems.length > 0) {
          await loadDetailBatch(pendingItems, result.source)
        } else {
          detailsStatus.value = 'complete'
        }
      } catch (error) {
        const message =
          error instanceof Error ? error.message : t('templates.online.errors.load_failed')

        if (indexItems.value.length === 0) {
          indexStatus.value = 'error'
          detailsStatus.value = 'error'
        }

        loadError.value = message
      } finally {
        if (session.value?.id === sessionId) {
          indexController = null
          currentLoadPromise = null
          isRefreshing.value = false
          updateDetailsStatus()
        }
      }
    })()

    return currentLoadPromise
  }

  async function ensureCatalogLoaded() {
    const hydrated = hydrateFromSnapshot()
    const shouldRefresh =
      !lastSuccessfulLoadAt.value ||
      Date.now() - lastSuccessfulLoadAt.value > NETWORK_REFRESH_INTERVAL_MS

    if (!hydrated) {
      await refreshCatalog()
      return
    }

    if (activeSource.value) {
      const pendingItems = getPendingDetailItems()
      if (pendingItems.length > 0) {
        void loadDetailBatch(pendingItems, activeSource.value)
      }
    }

    if (shouldRefresh && !currentLoadPromise) {
      void refreshCatalog({ background: true })
    }
  }

  async function reloadCatalog() {
    await refreshCatalog({ background: hasAnyData.value })
  }

  async function retryFailedDetails() {
    if (!activeSource.value || currentDetailsPromise) {
      return
    }

    const items = indexItems.value.filter(
      (item) => detailStateById.value[item.id]?.status === 'error'
    )
    await loadDetailBatch(items, activeSource.value)
  }

  async function retryTemplateDetail(id: string) {
    if (!activeSource.value || currentDetailsPromise) {
      return
    }

    const item = indexById.value[id]
    if (!item) {
      return
    }

    await loadDetailBatch([item], activeSource.value)
  }

  async function importTemplate(id: string) {
    const item = indexById.value[id]
    const state = detailStateById.value[id]

    if (!item || !isReadyState(state) || isImporting(id)) {
      return
    }

    markImporting(id, true)

    try {
      const existingTemplates = configStore.getTemplates()
      if (existingTemplates[item.name]) {
        const messageBox = await getMessageBox()
        await messageBox.confirm(
          t('templates.online.messages.exists_confirm', { name: item.name }),
          t('templates.online.messages.exists_title'),
          {
            confirmButtonText: t('templates.online.messages.overwrite'),
            cancelButtonText: t('common.cancel'),
            type: 'warning',
          }
        )
      }

      configStore.setTemplate(item.name, state.detail.template)
      await configStore.saveConfig()
      toast(t('templates.online.messages.import_success', { name: item.name }))
    } catch (error) {
      if (error !== 'cancel') {
        toast(t('templates.online.errors.import_failed'))
      }
    } finally {
      markImporting(id, false)
    }
  }

  function clearLoadError() {
    loadError.value = null
  }

  return {
    libraryOpen,
    keyword,
    selectedCategory,
    selectedBrand,
    indexStatus,
    detailsStatus,
    isRefreshing,
    loadError,
    activeSource,
    session,
    progress,
    showingCachedData,
    preferredSource,
    isFallbackSource,
    filteredRecords,
    availableBrands,
    hasAnyData,
    failedDetailCount,
    pendingDetailCount,
    allDetailsResolved,
    importingIds,
    openLibrary,
    closeLibrary,
    setKeyword,
    setSelectedCategory,
    setSelectedBrand,
    ensureCatalogLoaded,
    reloadCatalog,
    retryFailedDetails,
    retryTemplateDetail,
    importTemplate,
    isImporting,
    clearLoadError,
    templateCategories: TEMPLATE_CATEGORIES,
  }
})
