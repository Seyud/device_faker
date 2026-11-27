<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  Copyright (c) 2025-2026 Seyud
  Device Faker: https://github.com/Seyud/device_faker
-->

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

    <nav
      ref="bottomNavRef"
      :class="['bottom-nav glass-effect', { 'bottom-nav--dragging': navSlider.isDragging.value }]"
      @pointercancel="navSlider.handlePointerCancel"
      @pointerdown="navSlider.handlePointerDown"
      @pointermove="navSlider.handlePointerMove"
      @pointerup="navSlider.handlePointerEnd"
    >
      <!-- 悬浮滑块：底栏的「当前页」指示器，同时是拖动切页的抓手 -->
      <span class="nav-thumb" :style="navSlider.thumbStyle.value" aria-hidden="true"></span>
      <button
        v-for="page in nav.pages.value"
        :key="page.id"
        :class="['nav-item', { active: nav.activePage.value === page.id }]"
        @pointerdown="primePage(page.id)"
        @click.stop="handleNavClick(page.id)"
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
import { useNavSlider } from './composables/useNavSlider'
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
const bottomNavRef = ref<HTMLElement | null>(null)
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

/*
 * 悬浮底栏滑块：按住横向拖动切页。拖动中把页面轨道按进度实时推过去
 * （swipe.previewTo），松手交给 nav.handlePageChange 做吸附 + 历史入栈；
 * 落点即当前页时只让它回弹。
 */
const navSlider = useNavSlider({
  navRef: bottomNavRef,
  activeIndex: nav.activePageIndex,
  itemCount: computed(() => nav.pages.value.length),
  // 滑块往右滑 = 选中靠后的页 = 页面内容向左移动，所以进度取负
  onPreview: (index, progress) => swipe.previewTo(index, -progress * nav.pageStageWidth.value),
  onHoverIndex: (index) => {
    const pageId = nav.pages.value[index]?.id
    if (pageId) nav.primeNeighborPages(index)
  },
  onCommit: (index) => {
    const pageId = nav.pages.value[index]?.id
    if (!pageId) return
    if (pageId === nav.activePage.value) {
      swipe.settleTo(index)
      return
    }
    nav.handlePageChange(pageId)
  },
})

function handleNavClick(pageId: PageId) {
  // 拖动尾巴的 click 丢掉，否则会把刚吸附到的页又拉回抬手处的 tab
  if (navSlider.shouldSuppressClick()) return
  nav.handlePageChange(pageId)
}

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
  overflow-anchor: none;
  touch-action: pan-y;
}

.page-scroll-content {
  min-height: 100%;
  /* 宽屏收口：内容列居中，避免卡片/表单被拉成满屏长条（手机竖屏够不到上限） */
  max-width: var(--content-max-width, 60rem);
  margin-inline: auto;
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
  /* 纵向内边距抽成变量：滑块（.nav-thumb）的上下 inset 要和它对齐 */
  --nav-pad-y: 0.75rem;
  display: flex;
  justify-content: space-around;
  align-items: center;
  padding: var(--nav-pad-y) 0;
  /* 横向拖动留给指针手势（滑块切页），纵向仍允许系统滚动/手势 */
  touch-action: pan-y;
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

/*
 * 悬浮滑块：既是「当前页」指示器，也是拖动切页的抓手。
 * 位置/宽度由 useNavSlider 按真实 tab 的 offset 算好写进行内样式，
 * 这里只负责外观与过渡；静止时带过渡，拖动时去掉过渡跟手。
 */
.nav-thumb {
  position: absolute;
  top: var(--nav-pad-y);
  bottom: var(--nav-pad-y);
  left: 0;
  z-index: 0;
  border-radius: 0.5rem;
  background: linear-gradient(135deg, rgba(14, 165, 233, 0.18) 0%, rgba(168, 85, 247, 0.18) 100%);
  box-shadow: inset 0 0 0 1px rgba(14, 165, 233, 0.16);
  pointer-events: none;
  will-change: transform;
  transition:
    transform 0.26s cubic-bezier(0.25, 1, 0.5, 1),
    width 0.2s ease;
}

.bottom-nav--dragging .nav-thumb {
  transition: none;
}

/* 拖动中不要保留起手 tab 的按压反馈（高亮由滑块负责） */
.bottom-nav--dragging .nav-item:active {
  background: transparent;
  transform: none;
}

.nav-item {
  position: relative;
  z-index: 1;
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
  /* 高亮底由 .nav-thumb 负责，这里只改配色，避免出现两个指示器 */
  background: transparent;
  color: var(--primary);
}

.nav-item.active svg {
  filter: drop-shadow(0 0 8px rgba(14, 165, 233, 0.5));
}

.nav-label {
  font-size: 0.75rem;
  font-weight: 500;
}

/*
 * 宽屏底部导航：满宽 + space-around 会把 4 个 tab 摊到 1280dp 的两端，
 * 这里收成居中悬浮胶囊（各 tab 等分），并给页面底部多留一点滚动余量。
 * 注意只改 .bottom-nav 自身定位，导航项点击/高亮逻辑不变。
 */
@media (min-width: 700px) {
  .page-scroll {
    padding-bottom: 7rem;
  }

  .bottom-nav {
    --nav-pad-y: 0.5rem;
    left: 50%;
    right: auto;
    transform: translateX(-50%);
    width: min(32rem, calc(100vw - 2rem));
    bottom: calc(0.75rem + var(--safe-area-inset-bottom, 0px));
    padding: var(--nav-pad-y);
    gap: 0.25rem;
    border-radius: 1.25rem;
    border: 1px solid rgba(255, 255, 255, 0.4);
    box-shadow: 0 8px 28px var(--shadow);
  }

  .dark .bottom-nav {
    border-color: rgba(51, 65, 85, 0.5);
  }

  .bottom-nav .nav-item {
    flex: 1 1 0;
    min-width: 0;
  }
}
</style>
