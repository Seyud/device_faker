<template>
  <div :class="['app-container', { dark: isDark }]">
    <main class="main-content">
      <div
        ref="pageStageRef"
        :class="['page-stage', { 'page-stage--dragging': swipe.isSwipeDragging.value }]"
        @click.capture="swipe.handleClickCapture"
        @pointercancel="swipe.handlePointerCancel"
        @pointerdown="swipe.handlePointerDown"
        @pointermove="swipe.handlePointerMove"
        @pointerup="swipe.handlePointerEnd"
      >
        <div class="page-track">
          <section
            v-for="page in nav.pages.value"
            :key="page.id"
            :aria-hidden="nav.activePage.value !== page.id"
            class="page-panel"
          >
            <div
              class="page-scroll"
              @touchstart="overscroll.handleTouchStart"
              @touchmove="overscroll.handleTouchMove"
              @touchend="overscroll.handleTouchEnd"
              @touchcancel="overscroll.handleTouchEnd"
            >
              <div class="page-scroll-content">
                <header class="app-header">
                  <h1 class="header-title">
                    Device Faker
                    <span class="version">{{ nav.versionDisplay.value }}</span>
                  </h1>
                </header>
                <component
                  :is="page.component"
                  v-if="nav.shouldRenderPage(page.id)"
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
        v-for="page in nav.pages.value"
        :key="page.id"
        :class="['nav-item', { active: nav.activePage.value === page.id }]"
        @pointerdown="primePage(page.id)"
        @click.stop="nav.handlePageChange(page.id)"
      >
        <component :is="page.icon" :size="24" />
        <span class="nav-label">{{ page.label }}</span>
      </button>
    </nav>
  </div>
</template>

<script setup lang="ts">
import { computed, defineComponent, h, onMounted, onUnmounted, ref, watch } from 'vue'
import { useSettingsStore } from './stores/settings'
import { applyThemeToDocument } from './utils/theme'
import { usePageNavigation, PAGE_INDEX_BY_ID } from './composables/usePageNavigation'
import { usePageSwipe } from './composables/usePageSwipe'
import { useOverscrollBounce } from './composables/useOverscrollBounce'
import { usePageHistory } from './composables/usePageHistory'
import type { PageId } from './stores/navigation'

type ResizeObserverInstance = InstanceType<typeof window.ResizeObserver>

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

let pageStageResizeObserver: ResizeObserverInstance | null = null
let mediaQuery: ReturnType<typeof window.matchMedia> | null = null
let mediaQueryListener: ((event: { matches: boolean }) => void) | null = null

const pageStageRef = ref<HTMLElement | null>(null)
const settingsStore = useSettingsStore()
const systemPrefersDark = ref(window.matchMedia('(prefers-color-scheme: dark)').matches)

const isDark = computed(() => {
  if (settingsStore.theme === 'system') return systemPrefersDark.value
  return settingsStore.theme === 'dark'
})

// Composables — break circular dependency
const history = usePageHistory()
const nav = usePageNavigation({
  isHandlingPopstate: history.isHandlingPopstate,
  animateToPage: (pageId) => swipe.settleTo(PAGE_INDEX_BY_ID[pageId]),
})
history.bindSetActivePage(nav.setActivePage)

const swipe = usePageSwipe({
  activePage: nav.activePage,
  pageStageWidth: nav.pageStageWidth,
  pageStageRef,
  primeNeighborPages: nav.primeNeighborPages,
  setActivePageSilent: nav.setActivePageSilent,
})

const overscroll = useOverscrollBounce()

function primePage(pageId: PageId) {
  if (pageId === 'home') return
  nav.primeNeighborPages(PAGE_INDEX_BY_ID[pageId])
}

// Theme
watch(isDark, (v) => applyThemeToDocument(v), { immediate: true })

// Lifecycle
function syncWidth() {
  nav.syncPageStageWidth(pageStageRef.value)
  swipe.snapToCurrentPage()
}

onMounted(() => {
  history.setupHistory()
  nav.scheduleConfigBootstrap()
  nav.schedulePageWarmup()
  nav.scheduleAppDataWarmup()
  syncWidth()

  window.addEventListener('resize', syncWidth, { passive: true })

  if (typeof window.ResizeObserver === 'function' && pageStageRef.value) {
    pageStageResizeObserver = new window.ResizeObserver(syncWidth)
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
  history.cleanup()
  swipe.cleanup()
  overscroll.cleanup()
  nav.cleanup()

  window.removeEventListener('resize', syncWidth)
  pageStageResizeObserver?.disconnect()
  pageStageResizeObserver = null

  if (mediaQuery && mediaQueryListener) {
    mediaQuery.removeEventListener('change', mediaQueryListener)
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
  backface-visibility: hidden;
  -webkit-backface-visibility: hidden;
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
  height: 100%;
  min-height: 0;
  overflow-y: auto;
  overflow-x: hidden;
  padding-bottom: 5.5rem;
  -webkit-overflow-scrolling: touch;
  overscroll-behavior-y: none;
  scroll-behavior: smooth;
  touch-action: pan-y;
}

.page-scroll-content {
  min-height: 100%;
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
  border-top: 1px solid rgba(255, 255, 255, 0.4);
}

.dark .bottom-nav {
  background: rgba(30, 41, 59, 0.85);
  backdrop-filter: blur(20px) saturate(180%);
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
