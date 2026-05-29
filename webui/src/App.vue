<template>
  <div :class="['app-container', { dark: isDark }]">
    <main class="main-content">
      <div
        ref="pageStageRef"
        :class="['page-stage', { 'page-stage--dragging': isSwipeDragging }]"
        @click.capture="handleSwipeClickCapture"
        @pointercancel="handleSwipePointerCancel"
        @pointerdown="handleSwipePointerDown"
        @pointermove="handleSwipePointerMove"
        @pointerup="handleSwipePointerEnd"
      >
        <div class="page-track" :style="pageTrackStyle">
          <section
            v-for="page in pages"
            :key="page.id"
            :aria-hidden="activePage !== page.id"
            class="page-panel"
          >
            <div
              class="page-scroll"
              @touchstart="handlePageScrollTouchStart"
              @touchmove="handlePageScrollTouchMove"
              @touchend="handlePageScrollTouchEnd"
              @touchcancel="handlePageScrollTouchEnd"
            >
              <div class="page-scroll-content">
                <header class="app-header">
                  <h1 class="header-title">
                    Device Faker
                    <span class="version">{{ versionDisplay }}</span>
                  </h1>
                </header>
                <component
                  :is="page.component"
                  v-if="shouldRenderPage(page.id)"
                  class="page-view"
                />
                <AsyncPagePlaceholder v-else />
              </div>
            </div>
          </section>
        </div>
      </div>
    </main>

    <nav class="bottom-nav glass-effect">
      <button
        v-for="page in pages"
        :key="page.id"
        :class="['nav-item', { active: activePage === page.id }]"
        @pointerdown="primePage(page.id)"
        @click.stop="handlePageChange(page.id)"
      >
        <component :is="page.icon" :size="24" />
        <span class="nav-label">{{ page.label }}</span>
      </button>
    </nav>
  </div>
</template>

<script setup lang="ts">
import {
  computed,
  defineAsyncComponent,
  defineComponent,
  h,
  onMounted,
  onUnmounted,
  ref,
  watch,
} from 'vue'
import { Home, FileText, Smartphone, Settings } from 'lucide-vue-next'
import AppsPageSkeleton from './components/apps/AppsPageSkeleton.vue'
import { useAppsStore } from './stores/apps'
import { useConfigStore } from './stores/config'
import { useNavigationStore } from './stores/navigation'
import type { PageId } from './stores/navigation'
import { useSettingsStore } from './stores/settings'
import { applyThemeToDocument } from './utils/theme'
import { useI18n } from './utils/i18n'
import StatusPage from './pages/StatusPage.vue'

type AppsPageComponent = (typeof import('./pages/AppsPage.vue'))['default']
type TemplatePageComponent = (typeof import('./pages/TemplatePage.vue'))['default']
type SettingsPageComponent = (typeof import('./pages/SettingsPage.vue'))['default']
type SwipeIntent = 'horizontal' | 'vertical' | null
type ResizeObserverInstance = InstanceType<typeof window.ResizeObserver>
const PAGE_ORDER: PageId[] = ['home', 'templates', 'apps', 'settings']
const PAGE_INDEX_BY_ID: Record<PageId, number> = {
  home: 0,
  templates: 1,
  apps: 2,
  settings: 3,
}
const SWIPE_LOCK_DISTANCE_PX = 14
const SWIPE_DISTANCE_RATIO = 0.18
const SWIPE_VELOCITY_THRESHOLD = 0.65
const EDGE_RESISTANCE = 0.35
const CLICK_SUPPRESS_WINDOW_MS = 320
const OVERSCROLL_BOUNCE_MAX_PX = 88
const OVERSCROLL_BOUNCE_RESISTANCE = 0.5
const OVERSCROLL_STRETCH_RATIO = 0.001
const SWIPE_IGNORE_SELECTOR = [
  '[data-page-swipe-ignore]',
  'a',
  'button',
  'input',
  'textarea',
  'select',
  'label',
  '[role="button"]',
  '.el-button',
  '.el-input',
  '.el-input__wrapper',
  '.el-input__inner',
  '.el-textarea',
  '.el-select',
  '.el-switch',
  '.el-radio',
  '.el-checkbox',
  '.el-slider',
  '.el-dialog',
  '.el-overlay',
  '.el-popper',
  '.el-picker-panel',
  '.el-message-box',
].join(', ')

const AsyncPagePlaceholder = defineComponent({
  name: 'AsyncPagePlaceholder',
  setup() {
    return () =>
      h('div', { class: 'page-placeholder glass-effect' }, [
        h('div', { class: 'page-placeholder__line page-placeholder__line--title' }),
        h('div', { class: 'page-placeholder__line' }),
        h('div', { class: 'page-placeholder__line page-placeholder__line--short' }),
      ])
  },
})

let appsPageLoader: Promise<AppsPageComponent> | null = null
let templatePageLoader: Promise<TemplatePageComponent> | null = null
let settingsPageLoader: Promise<SettingsPageComponent> | null = null
let idleWarmupTimer: number | null = null
let idleWarmupId: number | null = null
let appDataWarmupTimer: number | null = null
let appDataWarmupId: number | null = null
let pageStageResizeObserver: ResizeObserverInstance | null = null
let mediaQuery: ReturnType<typeof window.matchMedia> | null = null
let mediaQueryListener: ((event: { matches: boolean }) => void) | null = null
let popstateHandler: ((event: Event) => void) | null = null
let overscrollReleaseTimer: number | null = null
let overscrollScrollElement: HTMLElement | null = null
let overscrollTouchId: number | null = null
let overscrollStartX = 0
let overscrollStartY = 0
let overscrollOffset = 0
let pointerStartX = 0
let pointerStartY = 0
let pointerStartTime = 0
let pointerIntent: SwipeIntent = null
let suppressClickUntil = 0

const pageStageRef = ref<HTMLElement | null>(null)
const activePage = ref<PageId>('home')
const renderedPageIds = ref<PageId[]>(['home'])
const pageStageWidth = ref(window.innerWidth)
const activePointerId = ref<number | null>(null)
const dragOffsetPx = ref(0)
const isSwipeDragging = ref(false)
const systemPrefersDark = ref(window.matchMedia('(prefers-color-scheme: dark)').matches)

function preloadAppsPage() {
  if (!appsPageLoader) {
    appsPageLoader = import('./pages/AppsPage.vue')
      .then((module) => module.default)
      .catch((error) => {
        appsPageLoader = null
        throw error
      })
  }

  return appsPageLoader
}

function preloadTemplatePage() {
  if (!templatePageLoader) {
    templatePageLoader = import('./pages/TemplatePage.vue')
      .then((module) => module.default)
      .catch((error) => {
        templatePageLoader = null
        throw error
      })
  }

  return templatePageLoader
}

function preloadSettingsPage() {
  if (!settingsPageLoader) {
    settingsPageLoader = import('./pages/SettingsPage.vue')
      .then((module) => module.default)
      .catch((error) => {
        settingsPageLoader = null
        throw error
      })
  }

  return settingsPageLoader
}

const AppsPage = defineAsyncComponent({
  loader: preloadAppsPage,
  suspensible: false,
  loadingComponent: AppsPageSkeleton,
  delay: 0,
})
const TemplatePage = defineAsyncComponent<TemplatePageComponent>({
  loader: preloadTemplatePage,
  suspensible: false,
  loadingComponent: AsyncPagePlaceholder,
  delay: 0,
})
const SettingsPage = defineAsyncComponent<SettingsPageComponent>({
  loader: preloadSettingsPage,
  suspensible: false,
  loadingComponent: AsyncPagePlaceholder,
  delay: 0,
})

const configStore = useConfigStore()
const appsStore = useAppsStore()
const settingsStore = useSettingsStore()
const navigationStore = useNavigationStore()
const { t } = useI18n()

const versionDisplay = computed(() =>
  configStore.moduleMetaReady ? configStore.moduleVersion : '--'
)
const isDark = computed(() => {
  if (settingsStore.theme === 'system') {
    return systemPrefersDark.value
  }

  return settingsStore.theme === 'dark'
})
const pages = computed(() => [
  { id: 'home' as const, label: t('nav.home'), icon: Home, component: StatusPage },
  {
    id: 'templates' as const,
    label: t('nav.templates'),
    icon: FileText,
    component: TemplatePage,
  },
  { id: 'apps' as const, label: t('nav.apps'), icon: Smartphone, component: AppsPage },
  {
    id: 'settings' as const,
    label: t('nav.settings'),
    icon: Settings,
    component: SettingsPage,
  },
])
const activePageIndex = computed(() => PAGE_INDEX_BY_ID[activePage.value])
const pageTrackStyle = computed(() => {
  const translateX = -(activePageIndex.value * pageStageWidth.value) + dragOffsetPx.value

  return {
    transform: `translate3d(${translateX}px, 0, 0)`,
    transition: isSwipeDragging.value ? 'none' : 'transform 260ms cubic-bezier(0.22, 1, 0.36, 1)',
  }
})

function markPageAsRendered(pageId: PageId) {
  if (renderedPageIds.value.includes(pageId)) {
    return
  }

  renderedPageIds.value = [...renderedPageIds.value, pageId]
}

function shouldRenderPage(pageId: PageId) {
  return renderedPageIds.value.includes(pageId) || activePage.value === pageId
}

function syncPageStageWidth() {
  const measuredWidth = pageStageRef.value?.clientWidth ?? window.innerWidth
  if (measuredWidth > 0) {
    pageStageWidth.value = measuredWidth
  }
}

function warmPage(pageId: PageId, options: { includeAppData?: boolean } = {}) {
  if (pageId === 'apps') {
    void preloadAppsPage().catch(() => {})
    if (options.includeAppData) {
      void appsStore.ensureUserAppsLoaded()
    }
    return
  }

  if (pageId === 'templates') {
    void preloadTemplatePage().catch(() => {})
    return
  }

  if (pageId === 'settings') {
    void preloadSettingsPage().catch(() => {})
  }
}

function primePage(pageId: PageId) {
  if (pageId === 'home') {
    return
  }

  markPageAsRendered(pageId)
  warmPage(pageId, { includeAppData: pageId === 'apps' })
}

function primeNeighborPages(index: number) {
  const previousPage = PAGE_ORDER[index - 1]
  const nextPage = PAGE_ORDER[index + 1]

  if (previousPage) {
    markPageAsRendered(previousPage)
    primePage(previousPage)
  }

  if (nextPage) {
    markPageAsRendered(nextPage)
    primePage(nextPage)
  }
}

function setActivePage(pageId: PageId, options: { skipHistory?: boolean } = {}) {
  if (activePage.value === pageId) {
    return
  }

  markPageAsRendered(pageId)
  activePage.value = pageId
  primePage(pageId)
  navigationStore.setCurrentPage(pageId)
  if (!options.skipHistory) {
    navigationStore.pushPageToStack(pageId)
  }
}

function handlePageChange(pageId: PageId) {
  setActivePage(pageId)
}

function normalizeTargetElement(target: globalThis.EventTarget | null) {
  if (target instanceof HTMLElement) {
    return target
  }

  if (target instanceof window.Node) {
    return target.parentElement
  }

  return null
}

function hasHorizontalScrollableAncestor(target: HTMLElement | null) {
  let current = target

  while (current && current !== pageStageRef.value) {
    const style = window.getComputedStyle(current)
    const overflowX = style.overflowX
    const isHorizontalScroller =
      (overflowX === 'auto' || overflowX === 'scroll' || overflowX === 'overlay') &&
      current.scrollWidth > current.clientWidth + 4

    if (isHorizontalScroller) {
      return true
    }

    current = current.parentElement
  }

  return false
}

function hasVerticalScrollableAncestor(target: HTMLElement | null, boundary: HTMLElement) {
  let current = target

  while (current && current !== boundary) {
    const style = window.getComputedStyle(current)
    const overflowY = style.overflowY
    const isVerticalScroller =
      (overflowY === 'auto' || overflowY === 'scroll' || overflowY === 'overlay') &&
      current.scrollHeight > current.clientHeight + 4

    if (isVerticalScroller) {
      return true
    }

    current = current.parentElement
  }

  return false
}

function shouldIgnoreSwipeTarget(target: globalThis.EventTarget | null) {
  const element = normalizeTargetElement(target)
  if (!element) {
    return false
  }

  if (element.closest(SWIPE_IGNORE_SELECTOR)) {
    return true
  }

  return hasHorizontalScrollableAncestor(element)
}

function clampSwipeOffset(offsetPx: number) {
  const activeIndex = activePageIndex.value
  const maxOffset = pageStageWidth.value

  let nextOffset = Math.max(Math.min(offsetPx, maxOffset), -maxOffset)

  if (activeIndex === 0 && nextOffset > 0) {
    nextOffset *= EDGE_RESISTANCE
  }

  if (activeIndex === PAGE_ORDER.length - 1 && nextOffset < 0) {
    nextOffset *= EDGE_RESISTANCE
  }

  return nextOffset
}

function releaseSwipeCapture(pointerId: number | null) {
  if (
    pointerId !== null &&
    pageStageRef.value?.hasPointerCapture &&
    pageStageRef.value.hasPointerCapture(pointerId)
  ) {
    pageStageRef.value.releasePointerCapture(pointerId)
  }
}

function resetSwipeTracking(pointerId: number | null = activePointerId.value) {
  releaseSwipeCapture(pointerId)
  activePointerId.value = null
  dragOffsetPx.value = 0
  isSwipeDragging.value = false
  pointerIntent = null
}

function handleSwipePointerDown(event: globalThis.PointerEvent) {
  if (event.button !== 0 || activePointerId.value !== null) {
    return
  }

  if (shouldIgnoreSwipeTarget(event.target)) {
    return
  }

  activePointerId.value = event.pointerId
  pointerStartX = event.clientX
  pointerStartY = event.clientY
  pointerStartTime = event.timeStamp
  pointerIntent = null
  dragOffsetPx.value = 0
  isSwipeDragging.value = false
  primeNeighborPages(activePageIndex.value)
}

function handleSwipePointerMove(event: globalThis.PointerEvent) {
  if (activePointerId.value !== event.pointerId) {
    return
  }

  const deltaX = event.clientX - pointerStartX
  const deltaY = event.clientY - pointerStartY

  if (pointerIntent === null) {
    const absX = Math.abs(deltaX)
    const absY = Math.abs(deltaY)

    if (absX < SWIPE_LOCK_DISTANCE_PX && absY < SWIPE_LOCK_DISTANCE_PX) {
      return
    }

    if (absX > absY * 1.1) {
      pointerIntent = 'horizontal'
      isSwipeDragging.value = true
      pageStageRef.value?.setPointerCapture?.(event.pointerId)
    } else {
      pointerIntent = 'vertical'
      resetSwipeTracking(event.pointerId)
      return
    }
  }

  if (pointerIntent !== 'horizontal') {
    return
  }

  dragOffsetPx.value = clampSwipeOffset(deltaX)
  event.preventDefault()
}

function handleSwipePointerEnd(event: globalThis.PointerEvent) {
  if (activePointerId.value !== event.pointerId) {
    return
  }

  const totalDeltaX = event.clientX - pointerStartX
  const elapsedMs = Math.max(event.timeStamp - pointerStartTime, 1)
  const velocityX = totalDeltaX / elapsedMs
  const swipeThresholdPx = pageStageWidth.value * SWIPE_DISTANCE_RATIO
  const currentIndex = activePageIndex.value
  let targetIndex = currentIndex

  if (pointerIntent === 'horizontal') {
    if (
      (totalDeltaX <= -swipeThresholdPx || velocityX <= -SWIPE_VELOCITY_THRESHOLD) &&
      currentIndex < PAGE_ORDER.length - 1
    ) {
      targetIndex = currentIndex + 1
    } else if (
      (totalDeltaX >= swipeThresholdPx || velocityX >= SWIPE_VELOCITY_THRESHOLD) &&
      currentIndex > 0
    ) {
      targetIndex = currentIndex - 1
    }

    if (Math.abs(totalDeltaX) > 8) {
      suppressClickUntil = window.performance.now() + CLICK_SUPPRESS_WINDOW_MS
    }
  }

  resetSwipeTracking(event.pointerId)

  const targetPage = PAGE_ORDER[targetIndex]
  if (targetPage && targetPage !== activePage.value) {
    setActivePage(targetPage)
  }
}

function handleSwipePointerCancel(event: globalThis.PointerEvent) {
  if (activePointerId.value !== event.pointerId) {
    return
  }

  resetSwipeTracking(event.pointerId)
}

function handleSwipeClickCapture(event: globalThis.MouseEvent) {
  if (window.performance.now() >= suppressClickUntil) {
    return
  }

  event.preventDefault()
  event.stopPropagation()
}

function findTouchById(
  touchList: globalThis.TouchList,
  touchId: number | null
): globalThis.Touch | null {
  if (touchId === null) {
    return null
  }

  for (let index = 0; index < touchList.length; index += 1) {
    const touch = touchList.item(index)
    if (touch?.identifier === touchId) {
      return touch
    }
  }

  return null
}

function setOverscrollOffset(scrollElement: HTMLElement, offsetPx: number) {
  overscrollOffset = offsetPx
  const stretchScale = offsetPx > 0 ? 1 + Math.abs(offsetPx) * OVERSCROLL_STRETCH_RATIO : 1
  const translateY = offsetPx < 0 ? offsetPx : 0

  scrollElement.style.setProperty('--overscroll-scale-y', `${stretchScale}`)
  scrollElement.style.setProperty('--overscroll-translate-y', `${translateY}px`)
  scrollElement.style.setProperty('--overscroll-origin-y', 'top')
}

function clearOverscrollState() {
  overscrollScrollElement = null
  overscrollTouchId = null
  overscrollStartX = 0
  overscrollStartY = 0
  overscrollOffset = 0
}

function releaseOverscroll(scrollElement: HTMLElement, animated = true) {
  if (overscrollReleaseTimer !== null) {
    window.clearTimeout(overscrollReleaseTimer)
    overscrollReleaseTimer = null
  }

  if (animated) {
    scrollElement.classList.add('page-scroll--releasing')
  } else {
    scrollElement.classList.remove('page-scroll--releasing')
  }

  setOverscrollOffset(scrollElement, 0)

  if (!animated) {
    scrollElement.classList.remove('page-scroll--overscrolling')
    return
  }

  overscrollReleaseTimer = window.setTimeout(() => {
    scrollElement.classList.remove('page-scroll--releasing')
    scrollElement.classList.remove('page-scroll--overscrolling')
    overscrollReleaseTimer = null
  }, 190)
}

function handlePageScrollTouchStart(event: globalThis.TouchEvent) {
  if (event.touches.length !== 1) {
    return
  }

  const scrollElement = event.currentTarget
  if (!(scrollElement instanceof HTMLElement)) {
    return
  }

  if (hasVerticalScrollableAncestor(normalizeTargetElement(event.target), scrollElement)) {
    clearOverscrollState()
    return
  }

  const touch = event.touches.item(0)
  if (!touch) {
    return
  }

  if (overscrollReleaseTimer !== null) {
    window.clearTimeout(overscrollReleaseTimer)
    overscrollReleaseTimer = null
  }

  scrollElement.classList.remove('page-scroll--releasing')
  overscrollScrollElement = scrollElement
  overscrollTouchId = touch.identifier
  overscrollStartX = touch.clientX
  overscrollStartY = touch.clientY
  overscrollOffset = 0
}

function handlePageScrollTouchMove(event: globalThis.TouchEvent) {
  const scrollElement = overscrollScrollElement
  if (!scrollElement || event.currentTarget !== scrollElement) {
    return
  }

  const touch = findTouchById(event.touches, overscrollTouchId)
  if (!touch) {
    return
  }

  const deltaX = touch.clientX - overscrollStartX
  const deltaY = touch.clientY - overscrollStartY

  if (Math.abs(deltaY) <= Math.abs(deltaX)) {
    if (overscrollOffset !== 0) {
      releaseOverscroll(scrollElement)
    }
    return
  }

  const maxScrollTop = Math.max(scrollElement.scrollHeight - scrollElement.clientHeight, 0)
  const isAtTop = scrollElement.scrollTop <= 0
  const isAtBottom = scrollElement.scrollTop >= maxScrollTop - 1
  const pullingPastTop = isAtTop && deltaY > 0
  const pullingPastBottom = isAtBottom && deltaY < 0

  if (!pullingPastTop && !pullingPastBottom) {
    if (overscrollOffset !== 0) {
      releaseOverscroll(scrollElement)
    }
    return
  }

  const resistedOffset =
    Math.sign(deltaY) *
    Math.min(OVERSCROLL_BOUNCE_MAX_PX, Math.abs(deltaY) * OVERSCROLL_BOUNCE_RESISTANCE)

  scrollElement.classList.add('page-scroll--overscrolling')
  scrollElement.classList.remove('page-scroll--releasing')
  setOverscrollOffset(scrollElement, resistedOffset)
  event.preventDefault()
}

function handlePageScrollTouchEnd(event: globalThis.TouchEvent) {
  const scrollElement = overscrollScrollElement
  if (!scrollElement || event.currentTarget !== scrollElement) {
    return
  }

  if (overscrollOffset !== 0) {
    releaseOverscroll(scrollElement)
  } else {
    scrollElement.classList.remove('page-scroll--overscrolling')
    scrollElement.classList.remove('page-scroll--releasing')
  }

  clearOverscrollState()
}

watch(
  isDark,
  (isDarkMode) => {
    applyThemeToDocument(isDarkMode)
  },
  { immediate: true }
)

function scheduleConfigBootstrap() {
  requestAnimationFrame(() => {
    window.setTimeout(() => {
      void configStore.bootstrap()
    }, 0)
  })
}

function schedulePageWarmup() {
  const runWarmup = () => {
    idleWarmupId = null
    idleWarmupTimer = null
    warmPage('templates')
    warmPage('settings')
    warmPage('apps')
  }

  if (typeof window.requestIdleCallback === 'function') {
    idleWarmupId = window.requestIdleCallback(runWarmup, { timeout: 1500 })
    return
  }

  idleWarmupTimer = window.setTimeout(runWarmup, 800)
}

function scheduleAppDataWarmup() {
  const runWarmup = () => {
    appDataWarmupId = null
    appDataWarmupTimer = null
    void appsStore.ensureUserAppsLoaded()
  }

  if (typeof window.requestIdleCallback === 'function') {
    appDataWarmupId = window.requestIdleCallback(runWarmup, { timeout: 2500 })
    return
  }

  appDataWarmupTimer = window.setTimeout(runWarmup, 1800)
}

function setupHistory() {
  // Give the initial entry a state so popstate can read dfPage when swiping back to it
  navigationStore.replacePageHistory('home')

  popstateHandler = (_event: Event) => {
    if (navigationStore.shouldIgnorePopstate()) return

    // Sub-page (modal) → close it visually.
    // The browser's history.back() already brought us to the previous entry,
    // so we must NOT call popModalHistory() — that would be an extra back step.
    // wasProgrammaticClose prevents the composable's watch from calling it either.
    if (navigationStore.hasOpenModals()) {
      navigationStore.dismissTopModal()
      return
    }

    // Top-level page → jump directly to home (skip intermediate top-level pages)
    if (navigationStore.hasPageHistory()) {
      navigationStore.jumpToHome()
      // jumpToHome calls history.go(-n), which fires another popstate.
      // That popstate restores the home entry's state; handler reads it below.
      return
    }

    // At home (no previous page) → read state; null means WebView exits via system back
    const state = window.history?.state as { dfPage?: PageId; modal?: boolean } | null
    const targetPage = state?.dfPage
    if (targetPage && PAGE_ORDER.includes(targetPage)) {
      setActivePage(targetPage, { skipHistory: true })
    }
  }

  window.addEventListener('popstate', popstateHandler)
}

onMounted(() => {
  setupHistory()
  scheduleConfigBootstrap()
  schedulePageWarmup()
  scheduleAppDataWarmup()
  syncPageStageWidth()

  window.addEventListener('resize', syncPageStageWidth, { passive: true })

  if (typeof window.ResizeObserver === 'function' && pageStageRef.value) {
    pageStageResizeObserver = new window.ResizeObserver(() => {
      syncPageStageWidth()
    })
    pageStageResizeObserver.observe(pageStageRef.value)
  }

  mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
  systemPrefersDark.value = mediaQuery.matches
  mediaQueryListener = (event) => {
    systemPrefersDark.value = event.matches
  }
  mediaQuery.addEventListener('change', mediaQueryListener)
})

onUnmounted(() => {
  if (popstateHandler) {
    window.removeEventListener('popstate', popstateHandler)
    popstateHandler = null
  }

  resetSwipeTracking(activePointerId.value)
  window.removeEventListener('resize', syncPageStageWidth)
  pageStageResizeObserver?.disconnect()
  pageStageResizeObserver = null

  if (mediaQuery && mediaQueryListener) {
    mediaQuery.removeEventListener('change', mediaQueryListener)
  }

  if (idleWarmupTimer !== null) {
    window.clearTimeout(idleWarmupTimer)
  }

  if (idleWarmupId !== null && typeof window.cancelIdleCallback === 'function') {
    window.cancelIdleCallback(idleWarmupId)
  }

  if (appDataWarmupTimer !== null) {
    window.clearTimeout(appDataWarmupTimer)
  }

  if (appDataWarmupId !== null && typeof window.cancelIdleCallback === 'function') {
    window.cancelIdleCallback(appDataWarmupId)
  }

  if (overscrollReleaseTimer !== null) {
    window.clearTimeout(overscrollReleaseTimer)
  }
})
</script>

<style scoped>
.app-container {
  display: flex;
  height: 100vh;
  height: 100dvh;
  min-height: 0;
  background: var(--background);
  padding: 0 var(--safe-area-inset-right) var(--safe-area-inset-bottom) var(--safe-area-inset-left);
}

.app-header {
  padding-top: calc(var(--safe-area-inset-top) + 1rem);
  padding-left: 1rem;
  padding-right: 1rem;
  padding-bottom: 1rem;
  border-radius: 0 0 1rem 1rem;
  margin-bottom: 0.5rem;
  box-shadow: 0 4px 12px var(--shadow);
  position: relative;
  overflow: hidden;
  background: var(--card-bg);
  border-bottom: 1px solid var(--border);
}

.app-header::before {
  content: '';
  position: absolute;
  inset: 0;
  background: linear-gradient(135deg, var(--gradient-start) 0%, var(--gradient-end) 100%);
  opacity: 0.08;
  z-index: 0;
}

.header-title {
  font-size: 1.5rem;
  font-weight: 600;
  background: linear-gradient(135deg, var(--gradient-start) 0%, var(--gradient-end) 100%);
  -webkit-background-clip: text;
  -webkit-text-fill-color: transparent;
  background-clip: text;
  display: flex;
  align-items: flex-end;
  gap: 0.5rem;
  line-height: 1;
  position: relative;
  z-index: 1;
}

.version {
  font-size: 1rem;
  font-weight: 400;
  color: var(--text-secondary);
  line-height: 1;
  padding-bottom: 0.1rem;
}

.main-content {
  flex: 1 1 auto;
  min-height: 0;
  padding: 0;
  overflow: hidden;
  display: flex;
}

.page-stage {
  position: relative;
  flex: 1;
  min-height: 0;
  overflow: hidden;
  touch-action: pan-y;
}

.page-stage--dragging {
  user-select: none;
  -webkit-user-select: none;
  cursor: grabbing;
}

.page-track {
  display: flex;
  height: 100%;
  will-change: transform;
}

.page-panel {
  flex: 0 0 100%;
  box-sizing: border-box;
  min-width: 0;
  min-height: 0;
  height: 100%;
  padding: 0 1rem;
}

.page-scroll {
  --overscroll-scale-y: 1;
  --overscroll-translate-y: 0px;
  --overscroll-origin-y: top;
  height: 100%;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding-bottom: 5.5rem;
  -webkit-overflow-scrolling: touch;
  overscroll-behavior-y: auto;
  scroll-behavior: smooth;
  touch-action: pan-y;
}

.page-scroll-content {
  min-height: 100%;
  transform: translate3d(0, var(--overscroll-translate-y), 0) scaleY(var(--overscroll-scale-y));
  transform-origin: 50% var(--overscroll-origin-y);
  will-change: transform;
}

.page-scroll--releasing .page-scroll-content {
  transition: transform 180ms cubic-bezier(0.2, 0.9, 0.3, 1);
}

.page-stage--dragging .page-scroll {
  scroll-behavior: auto;
}

.page-view {
  min-height: 100%;
  width: 100%;
}

.page-placeholder {
  display: flex;
  flex-direction: column;
  gap: 0.875rem;
  padding: 1.5rem;
  border-radius: 1rem;
  min-height: 14rem;
}

.page-placeholder__line {
  height: 0.95rem;
  width: 100%;
  border-radius: 999px;
  background: linear-gradient(90deg, var(--border) 25%, var(--card-bg) 50%, var(--border) 75%);
  background-size: 200% 100%;
  animation: page-placeholder-shimmer 1.3s linear infinite;
  opacity: 0.75;
}

.page-placeholder__line--title {
  width: 42%;
  height: 1.2rem;
}

.page-placeholder__line--short {
  width: 65%;
}

@keyframes page-placeholder-shimmer {
  from {
    background-position: -200% 0;
  }

  to {
    background-position: 200% 0;
  }
}

.bottom-nav {
  display: flex;
  justify-content: space-around;
  align-items: center;
  padding: 0.75rem 0;
  border-radius: 1rem 1rem 0 0;
  box-shadow: 0 -4px 12px var(--shadow);
  position: fixed;
  bottom: 0;
  left: 0;
  right: 0;
  z-index: 100;
  pointer-events: auto;
  background: rgba(255, 255, 255, 0.8);
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
  border-top: 1px solid rgba(255, 255, 255, 0.4);
}

.dark .bottom-nav {
  background: rgba(30, 41, 59, 0.85);
  backdrop-filter: blur(20px) saturate(180%);
  -webkit-backdrop-filter: blur(20px) saturate(180%);
  border-top: 1px solid rgba(51, 65, 85, 0.4);
}

.nav-item {
  display: flex;
  flex-direction: column;
  align-items: center;
  gap: 0.25rem;
  padding: 0.5rem 1rem;
  background: transparent;
  border: none;
  color: var(--text-secondary);
  transition:
    color 0.2s ease,
    background-color 0.2s ease,
    transform 0.2s ease;
  border-radius: 0.5rem;
  -webkit-tap-highlight-color: transparent;
  user-select: none;
  -webkit-user-select: none;
  cursor: pointer;
  touch-action: manipulation;
}

.nav-item:active {
  background: linear-gradient(135deg, rgba(14, 165, 233, 0.15) 0%, rgba(168, 85, 247, 0.15) 100%);
  transform: scale(0.95);
}

.nav-item.active {
  background: linear-gradient(135deg, rgba(14, 165, 233, 0.1) 0%, rgba(168, 85, 247, 0.1) 100%);
  color: var(--primary);
}

.nav-item.active svg {
  filter: drop-shadow(0 0 8px rgba(14, 165, 233, 0.5));
}

.nav-label {
  font-size: 0.75rem;
  font-weight: 500;
}
</style>
