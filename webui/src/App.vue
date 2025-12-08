<template>
  <div :class="['app-container', { dark: isDark }]">
    <!-- 顶部标题栏 -->
    <header class="app-header glass-effect">
      <h1 class="header-title">
        Device Faker
        <span class="version">{{ version }}</span>
      </h1>
    </header>

    <!-- 主内容区域 -->
    <main class="main-content">
      <KeepAlive>
        <component :is="currentPageComponent" :key="currentPage" />
      </KeepAlive>
    </main>

    <!-- 底部导航栏 -->
    <nav class="bottom-nav glass-effect">
      <button
        v-for="page in pages"
        :key="page.id"
        :class="['nav-item', { active: currentPage === page.id }]"
        @click.stop="handlePageChange(page.id)"
      >
        <component :is="page.icon" :size="24" />
        <span class="nav-label">{{ page.label }}</span>
      </button>
    </nav>
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onUnmounted, watchEffect, nextTick } from 'vue'
import { Home, FileText, Smartphone, Settings } from 'lucide-vue-next'
import { useConfigStore } from './stores/config'
import { useSettingsStore } from './stores/settings'
import { useI18n } from './utils/i18n'
import StatusPage from './pages/StatusPage.vue'
import TemplatePage from './pages/TemplatePage.vue'
import AppsPage from './pages/AppsPage.vue'
import SettingsPage from './pages/SettingsPage.vue'

const configStore = useConfigStore()
const settingsStore = useSettingsStore()

const currentPage = ref('status')
let lastClickTime = 0
let isChangingPage = false

// 滚动条宽度补偿机制
const scrollbarWidth = ref(0)

// 处理页面切换，防止重复点击和事件冲突
function handlePageChange(pageId: string) {
  const now = Date.now()

  // 如果正在切换页面，忽略
  if (isChangingPage) {
    return
  }

  // 防抖：50ms 内只响应一次点击
  if (now - lastClickTime < 50) {
    return
  }

  lastClickTime = now

  if (currentPage.value !== pageId) {
    isChangingPage = true

    // 使用 requestAnimationFrame 确保在下一帧切换，避免阻塞
    requestAnimationFrame(() => {
      currentPage.value = pageId

      nextTick(() => {
        // 触发布局更新，确保元素适应新宽度
        const mainContent = document.querySelector('.main-content') as HTMLElement
        if (mainContent) {
          // 触发重排
          void mainContent.offsetHeight
        }

        // 触发窗口 resize 事件，确保全局布局更新
        window.dispatchEvent(new Event('resize'))

        // 重置标志
        setTimeout(() => {
          isChangingPage = false
        }, 50)
      })
    })
  }
}
const version = computed(() => configStore.moduleVersion)
const isDark = computed(() => {
  if (settingsStore.theme === 'system') {
    return window.matchMedia('(prefers-color-scheme: dark)').matches
  }
  return settingsStore.theme === 'dark'
})

const { t } = useI18n()

const pages = computed(() => [
  { id: 'status', label: t('nav.status'), icon: Home, component: StatusPage },
  { id: 'templates', label: t('nav.templates'), icon: FileText, component: TemplatePage },
  { id: 'apps', label: t('nav.apps'), icon: Smartphone, component: AppsPage },
  { id: 'settings', label: t('nav.settings'), icon: Settings, component: SettingsPage },
])

const currentPageComponent = computed(() => {
  return pages.value.find((p) => p.id === currentPage.value)?.component || StatusPage
})

// 应用深色模式到 html 元素（Element Plus 需要）
watchEffect(() => {
  if (isDark.value) {
    document.documentElement.classList.add('dark')
    // 深色模式：匹配深色毛玻璃效果
    document.getElementById('theme-color')?.setAttribute('content', '#1a2538')
  } else {
    document.documentElement.classList.remove('dark')
    // 浅色模式：匹配浅色毛玻璃效果
    document.getElementById('theme-color')?.setAttribute('content', '#f2f9ff')
  }
})

// 计算滚动条宽度
function calculateScrollbarWidth(): number {
  // 创建一个测试元素来计算滚动条宽度
  const testDiv = document.createElement('div')
  testDiv.style.cssText = `
    position: absolute;
    top: -9999px;
    width: 100px;
    height: 100px;
    overflow: scroll;
    visibility: hidden;
  `
  document.body.appendChild(testDiv)

  // 计算滚动条宽度
  const width = testDiv.offsetWidth - testDiv.clientWidth
  document.body.removeChild(testDiv)
  return width
}

// 应用滚动条宽度补偿
function applyScrollbarWidth() {
  const width = calculateScrollbarWidth()
  const previousWidth = scrollbarWidth.value
  scrollbarWidth.value = width

  // 为所有页面容器添加滚动条宽度补偿
  const mainContent = document.querySelector('.main-content') as HTMLElement
  if (mainContent) {
    // 检查页面是否有垂直滚动条
    const hasScrollbar = document.body.scrollHeight > window.innerHeight
    const newPadding = hasScrollbar ? `${width}px` : '0px'
    if (mainContent.style.paddingRight !== newPadding) {
      mainContent.style.paddingRight = newPadding
      // 记录调试信息
      console.warn(
        `[DeviceFaker] 滚动条宽度补偿更新: 滚动条宽度=${width}px, hasScrollbar=${hasScrollbar}, paddingRight=${newPadding}`
      )
    }
  }

  // 记录宽度变化
  if (width !== previousWidth) {
    console.warn(`[DeviceFaker] 滚动条宽度变化: 旧=${previousWidth}px, 新=${width}px`)
  }
}

// 监听系统主题变化
onMounted(() => {
  configStore.loadConfig()
  configStore.loadModuleVersion()

  // 初始化滚动条宽度计算
  applyScrollbarWidth()

  // 监听窗口 resize 和滚动事件，动态更新滚动条宽度补偿
  window.addEventListener('resize', applyScrollbarWidth)
  window.addEventListener('scroll', applyScrollbarWidth)

  const mediaQuery = window.matchMedia('(prefers-color-scheme: dark)')
  mediaQuery.addEventListener('change', () => {
    if (settingsStore.theme === 'system') {
      // 触发重新计算
      applyScrollbarWidth()
      window.dispatchEvent(new Event('resize'))
    }
  })
})

// 组件卸载时清理事件监听器
onUnmounted(() => {
  window.removeEventListener('resize', applyScrollbarWidth)
  window.removeEventListener('scroll', applyScrollbarWidth)
})
</script>

<style scoped>
.app-container {
  display: flex;
  flex-direction: column;
  min-height: 100vh; /* 改为最小高度,允许内容超出视口 */
  background: var(--background);
  /* 移除顶部内边距,让顶栏延伸到状态栏 */
  padding: 0 var(--safe-area-inset-right) var(--safe-area-inset-bottom) var(--safe-area-inset-left);
}

.app-header {
  /* 添加顶部内边距以适配状态栏 */
  padding-top: calc(var(--safe-area-inset-top) + 1rem);
  padding-left: 1rem;
  padding-right: 1rem;
  padding-bottom: 1rem;
  border-radius: 0 0 1rem 1rem;
  margin-bottom: 1rem;
  box-shadow: 0 4px 12px var(--shadow);
  position: relative; /* 改为相对定位,不固定在视野 */
  overflow: hidden;
}

.app-header::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
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
  align-items: center;
  gap: 0.5rem;
  position: relative;
  z-index: 1;
}

.version {
  font-size: 0.875rem;
  font-weight: 400;
  color: var(--text-secondary);
}

.main-content {
  flex: 1;
  overflow-y: scroll;
  padding: 0 1rem;
  padding-bottom: 5.5rem; /* 为固定定位的底栏留出空间 */
  /* 优化滚动性能 */
  -webkit-overflow-scrolling: touch;
  scroll-behavior: smooth;
}

.bottom-nav {
  display: flex;
  justify-content: space-around;
  align-items: center;
  padding: 0.75rem 0;
  border-radius: 1rem 1rem 0 0;
  box-shadow: 0 -4px 12px var(--shadow);
  position: fixed; /* 使用固定定位 */
  bottom: 0;
  left: 0;
  right: 0;
  z-index: 100; /* 正常显示时的优先级 */
  pointer-events: auto;
  /* 添加真正的毛玻璃效果 */
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
  transition: all 0.2s ease;
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
