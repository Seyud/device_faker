<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  Copyright (c) 2025-2026 Seyud
  Device Faker: https://github.com/Seyud/device_faker
-->

<template>
  <div class="settings-page">
    <div class="settings-section glass-effect">
      <h2 class="section-title">{{ t('settings.display.title') }}</h2>

      <div class="setting-item setting-item-split">
        <div class="setting-info">
          <div class="setting-icon">
            <Moon :size="24" />
          </div>
          <div class="setting-text">
            <h3 class="setting-name">{{ t('settings.display.theme.label') }}</h3>
            <p class="setting-desc">{{ t('settings.display.theme.desc') }}</p>
          </div>
        </div>
        <BaseSelect
          v-model="currentTheme"
          class="setting-control"
          :options="themeOptions"
          :aria-label="t('settings.display.theme.label')"
          @change="onThemeChange"
        />
      </div>

      <div class="setting-item setting-item-split">
        <div class="setting-info">
          <div class="setting-icon">
            <Globe :size="24" />
          </div>
          <div class="setting-text">
            <h3 class="setting-name">{{ t('settings.display.language.label') }}</h3>
            <p class="setting-desc">{{ t('settings.display.language.desc') }}</p>
          </div>
        </div>
        <BaseSelect
          v-model="currentLanguage"
          class="setting-control"
          :options="languageOptions"
          :aria-label="t('settings.display.language.label')"
          @change="onLanguageChange"
        />
      </div>
    </div>

    <div class="settings-section glass-effect">
      <h2 class="section-title">{{ t('settings.tools.title') }}</h2>

      <div class="setting-item">
        <div class="setting-info">
          <div class="setting-icon">
            <FileUp :size="24" />
          </div>
          <div class="setting-text">
            <h3 class="setting-name">{{ t('settings.tools.convert.label') }}</h3>
            <p class="setting-desc">{{ t('settings.tools.convert.desc') }}</p>
          </div>
        </div>
        <el-button type="primary" :loading="converting" @click="startConversion">
          {{ t('settings.tools.convert.btn') }}
        </el-button>
      </div>
    </div>

    <div class="settings-section glass-effect">
      <h2 class="section-title">{{ t('settings.module.title') }}</h2>

      <div class="setting-item setting-item-horizontal">
        <div class="setting-info">
          <div class="setting-icon">
            <Bug :size="24" />
          </div>
          <div class="setting-text">
            <h3 class="setting-name">{{ t('settings.module.debug.label') }}</h3>
            <p class="setting-desc">{{ t('settings.module.debug.desc') }}</p>
          </div>
        </div>
        <el-switch v-model="debugMode" class="setting-control-switch" @change="onDebugChange" />
      </div>
    </div>

    <!-- 转换结果对话框 -->
    <el-dialog
      v-model="convertDialogVisible"
      :title="t('settings.dialog.result.title')"
      width="90%"
      :close-on-click-modal="false"
      :append-to-body="true"
      class="template-dialog"
      modal-class="template-dialog-modal"
    >
      <el-form label-width="100px" label-position="top">
        <el-form-item :label="t('settings.dialog.result.template_name_label')">
          <el-input
            v-model="convertedTemplateName"
            :placeholder="t('settings.dialog.result.template_name_placeholder')"
          />
        </el-form-item>
        <el-form-item class="form-item-wide" :label="t('settings.dialog.result.preview_label')">
          <el-input
            v-model="convertedContent"
            type="textarea"
            :rows="10"
            readonly
            class="code-font"
          />
        </el-form-item>
      </el-form>
      <template #footer>
        <el-button @click="convertDialogVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" @click="saveConvertedTemplate">{{
          t('settings.dialog.result.btn_save')
        }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { ref, watch, computed, onActivated } from 'vue'
import { Moon, Globe, Bug, FileUp } from '@lucide/vue'
import { useConfigStore } from '../stores/config'
import { useSettingsStore } from '../stores/settings'
import { useI18n } from '../utils/i18n'
import { toast } from 'kernelsu-alt'
import type { Template } from '../types'
import { parseFirstTemplateFromToml } from '../utils/templateTransfer'
import { readZipEntries } from '../utils/zipReader'
import { convertZipEntriesToTemplate } from '../utils/propsToTemplate'

const configStore = useConfigStore()
const settingsStore = useSettingsStore()
const { t } = useI18n()

const currentTheme = ref(settingsStore.theme)
const currentLanguage = ref(settingsStore.language)
const debugMode = ref(configStore.config.debug || false)

const convertDialogVisible = ref(false)
const converting = ref(false)
const convertedTemplate = ref<Template | null>(null)
const convertedTemplateName = ref('')
const convertedContent = ref('')

function onThemeChange(value: string | number) {
  settingsStore.setTheme(value as 'system' | 'light' | 'dark')
}

function onLanguageChange(value: string | number) {
  settingsStore.setLanguage(value as 'system' | 'zh' | 'en' | 'tr')
}

const themeOptions = computed(() => [
  { value: 'system', label: t('settings.display.theme.system') },
  { value: 'light', label: t('settings.display.theme.light') },
  { value: 'dark', label: t('settings.display.theme.dark') },
])

const languageOptions = computed(() => [
  { value: 'system', label: t('settings.display.language.system') },
  { value: 'zh', label: t('settings.display.language.zh') },
  { value: 'en', label: t('settings.display.language.en') },
  { value: 'tr', label: t('settings.display.language.tr') },
])

async function onDebugChange(value: boolean) {
  configStore.config.debug = value
  try {
    await configStore.saveConfig()
    toast(value ? t('settings.messages.debug_enabled') : t('settings.messages.debug_disabled'))
  } catch {
    toast(t('settings.messages.save_failed'))
  }
}

function pickZipFile(): Promise<File | null> {
  return new Promise((resolve) => {
    const input = document.createElement('input')
    input.type = 'file'
    input.accept = '.zip'
    input.onchange = (event) => {
      const target = event.target as HTMLInputElement
      const file = target.files?.[0] || null
      resolve(file)
    }
    input.click()
  })
}

async function startConversion() {
  if (converting.value) return

  if (import.meta.env?.DEV) {
    const { mockConfig } = await import('../utils/mockData')
    const { templateData, defaultName } = parseFirstTemplateFromToml(mockConfig)
    convertedTemplate.value = templateData
    convertedTemplateName.value = defaultName
    convertedContent.value = mockConfig
    convertDialogVisible.value = true
    return
  }

  const file = await pickZipFile()
  if (!file) {
    return
  }

  converting.value = true

  try {
    // ZIP 在 WebView 内直接解析：不落盘、不 fork CLI、不依赖 manager 版本
    const entries = await readZipEntries(await file.arrayBuffer())
    const { name, template, toml } = convertZipEntriesToTemplate(entries)
    convertedTemplate.value = template
    convertedTemplateName.value = name
    convertedContent.value = toml
    convertDialogVisible.value = true
  } catch (err) {
    toast(
      `${t('settings.messages.convert_failed')}: ${err instanceof Error ? err.message : String(err)}`
    )
    console.error(err)
  } finally {
    converting.value = false
  }
}

async function saveConvertedTemplate() {
  if (!convertedTemplateName.value) {
    toast(t('settings.dialog.result.template_name_placeholder'))
    return
  }
  if (!convertedTemplate.value) return

  try {
    configStore.setTemplate(convertedTemplateName.value, convertedTemplate.value)
    await configStore.saveConfig()
    toast(t('settings.messages.template_saved'))
    convertDialogVisible.value = false
  } catch (err) {
    toast(
      `${t('settings.messages.save_failed')}: ${err instanceof Error ? err.message : String(err)}`
    )
  }
}

// 监听配置变化（只创建一次监听器）
watch(
  () => configStore.config.debug,
  (newDebug: boolean | undefined) => {
    const newValue = newDebug || false
    if (debugMode.value !== newValue) {
      debugMode.value = newValue
    }
  }
)

// KeepAlive 激活时同步最新配置
onActivated(() => {
  currentTheme.value = settingsStore.theme
  currentLanguage.value = settingsStore.language
  debugMode.value = configStore.config.debug || false
})
</script>

<style scoped>
.settings-page {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  width: 100%;
  /* 宽屏收窄：设置项是「左标题 + 右控件」，960px 满宽时两端相距过远（手机无影响） */
  max-width: var(--dialog-max-width, 46rem);
  margin-inline: auto;
  box-sizing: border-box;
  /* 确保宽度稳定，不受滚动条影响 */
  overflow: hidden;
}

.settings-section {
  padding: 1.5rem;
  border-radius: 1rem;
  box-shadow: 0 2px 8px var(--shadow);
}

.section-title {
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--text);
  margin-bottom: 1rem;
}

.setting-item {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  padding: 1rem 0;
  border-bottom: 1px solid var(--border);
}

.setting-item:last-child {
  border-bottom: none;
}

.setting-info {
  display: flex;
  align-items: center;
  gap: 1rem;
  flex: 1;
}

.setting-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  background: var(--background);
  border-radius: 0.5rem;
  color: var(--primary);
  flex-shrink: 0;
}

.setting-text {
  flex: 1;
  overflow: hidden;
}

.setting-name {
  font-size: 1rem;
  font-weight: 500;
  color: var(--text);
  margin: 0 0 0.25rem 0;
  line-height: 1.5;
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.setting-desc {
  font-size: 0.875rem;
  color: var(--text-secondary);
  margin: 0;
  line-height: 1.5;
}

.setting-control {
  width: 100%;
}

.setting-item-horizontal {
  flex-direction: row;
  align-items: center;
  justify-content: space-between;
}

.setting-item-split {
  flex-direction: row;
  align-items: flex-start;
  justify-content: space-between;
  gap: 1rem;
}

.setting-item-split .setting-info {
  min-width: 0;
}

.setting-item-split .setting-name {
  white-space: normal;
  text-overflow: clip;
}

.setting-item-split .setting-desc {
  overflow-wrap: anywhere;
}

.setting-item-split .setting-control {
  width: clamp(7rem, 30vw, 11rem);
  min-width: 7rem;
  flex-shrink: 1;
}

.setting-control-switch {
  flex-shrink: 0;
  margin-left: 1rem;
}

/* Code font for preview */
.code-font {
  font-family: monospace;
}

.form-tip {
  font-size: 0.75rem;
  color: var(--text-secondary);
  margin-top: 0.25rem;
}
</style>
