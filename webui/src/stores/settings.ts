import { defineStore } from 'pinia'
import { ref, watch } from 'vue'
import type { OnlineTemplateSource, Settings } from '../types'

const SETTINGS_KEY = 'device_faker_settings'

export const useSettingsStore = defineStore('settings', () => {
  const settings = ref<Settings>({
    theme: 'system',
    language: 'system',
    showSystemApps: false,
    onlineTemplateSource: 'gitee',
  })

  // 加载设置
  function loadSettings() {
    const stored = localStorage.getItem(SETTINGS_KEY)
    if (stored) {
      try {
        const parsed = JSON.parse(stored) as Partial<Settings>
        settings.value = {
          theme: 'system',
          language: 'system',
          showSystemApps: false,
          onlineTemplateSource: 'gitee',
          ...parsed,
        }
      } catch {
        // 使用默认设置
      }
    }
  }

  // 保存设置
  function saveSettings() {
    localStorage.setItem(SETTINGS_KEY, JSON.stringify(settings.value))
  }

  // 主题设置
  const theme = ref<'system' | 'light' | 'dark'>('system')

  function setTheme(newTheme: 'system' | 'light' | 'dark') {
    settings.value.theme = newTheme
    theme.value = newTheme
    saveSettings()
  }

  // 语言设置
  const language = ref<'system' | 'zh' | 'en'>('system')

  function setLanguage(newLanguage: 'system' | 'zh' | 'en') {
    settings.value.language = newLanguage
    language.value = newLanguage
    saveSettings()
  }

  const showSystemApps = ref(false)

  function setShowSystemApps(enabled: boolean) {
    settings.value.showSystemApps = enabled
    showSystemApps.value = enabled
    saveSettings()
  }

  const onlineTemplateSource = ref<OnlineTemplateSource>('gitee')

  function setOnlineTemplateSource(source: OnlineTemplateSource) {
    settings.value.onlineTemplateSource = source
    onlineTemplateSource.value = source
    saveSettings()
  }

  // 监听设置变化
  watch(settings, saveSettings, { deep: true })

  // 初始化
  loadSettings()
  theme.value = settings.value.theme
  language.value = settings.value.language
  showSystemApps.value = settings.value.showSystemApps
  onlineTemplateSource.value = settings.value.onlineTemplateSource

  return {
    settings,
    theme,
    language,
    showSystemApps,
    onlineTemplateSource,
    setTheme,
    setLanguage,
    setShowSystemApps,
    setOnlineTemplateSource,
  }
})
