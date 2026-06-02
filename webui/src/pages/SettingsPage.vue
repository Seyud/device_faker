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
        <el-select v-model="currentTheme" class="setting-control" @change="onThemeChange">
          <el-option :label="t('settings.display.theme.system')" value="system" />
          <el-option :label="t('settings.display.theme.light')" value="light" />
          <el-option :label="t('settings.display.theme.dark')" value="dark" />
        </el-select>
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
        <el-select v-model="currentLanguage" class="setting-control" @change="onLanguageChange">
          <el-option :label="t('settings.display.language.system')" value="system" />
          <el-option :label="t('settings.display.language.zh')" value="zh" />
          <el-option :label="t('settings.display.language.en')" value="en" />
          <el-option :label="t('settings.display.language.tr')" value="tr" />
        </el-select>
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

      <div class="setting-item">
        <div class="setting-info">
          <div class="setting-icon">
            <Settings :size="24" />
          </div>
          <div class="setting-text">
            <h3 class="setting-name">{{ t('settings.module.default_mode.label') }}</h3>
            <p class="setting-desc">{{ t('settings.module.default_mode.desc') }}</p>
          </div>
        </div>
        <el-select v-model="defaultMode" class="setting-control" @change="onModeChange">
          <el-option :label="t('settings.module.default_mode.lite')" value="lite" />
          <el-option :label="t('settings.module.default_mode.full')" value="full" />
          <el-option :label="t('settings.module.default_mode.resetprop')" value="resetprop" />
        </el-select>
      </div>

      <div class="setting-item">
        <div class="setting-info">
          <div class="setting-icon">
            <Shield :size="24" />
          </div>
          <div class="setting-text">
            <div
              style="
                display: flex;
                align-items: center;
                justify-content: space-between;
                margin-bottom: 0.25rem;
              "
            >
              <h3 class="setting-name" style="margin-bottom: 0; white-space: normal">
                {{ t('settings.module.force_denylist_unmount.label') }}
              </h3>
              <el-switch
                v-model="defaultForceDenylistUnmount"
                class="setting-control-switch"
                @change="onForceDenylistUnmountChange"
              />
            </div>
            <p class="setting-desc">{{ t('settings.module.force_denylist_unmount.desc') }}</p>
          </div>
        </div>
      </div>

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
        <el-form-item :label="t('settings.dialog.result.preview_label')">
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
import { ref, watch, onActivated } from 'vue'
import { Moon, Globe, Settings, Bug, FileUp, Shield } from 'lucide-vue-next'
import { useConfigStore } from '../stores/config'
import { useSettingsStore } from '../stores/settings'
import { execCommand, readFile } from '../utils/ksu'
import { useI18n } from '../utils/i18n'
import { toast } from 'kernelsu-alt'
import type { Template } from '../types'
import {
  convertZipOnDevice,
  createDeviceTempPath,
  parseFirstTemplateFromToml,
  shellQuote,
  uploadFileToDevice,
} from '../utils/templateTransfer'

const configStore = useConfigStore()
const settingsStore = useSettingsStore()
const { t } = useI18n()

const currentTheme = ref(settingsStore.theme)
const currentLanguage = ref(settingsStore.language)
const defaultMode = ref(configStore.config.default_mode || 'lite')
const defaultForceDenylistUnmount = ref(configStore.config.default_force_denylist_unmount || false)
const debugMode = ref(configStore.config.debug || false)

const convertDialogVisible = ref(false)
const converting = ref(false)
const convertedTemplate = ref<Template | null>(null)
const convertedTemplateName = ref('')
const convertedContent = ref('')

function onThemeChange(value: string) {
  settingsStore.setTheme(value as 'system' | 'light' | 'dark')
}

function onLanguageChange(value: string) {
  settingsStore.setLanguage(value as 'system' | 'zh' | 'en' | 'tr')
}

async function onModeChange(value: string) {
  configStore.config.default_mode = value as 'lite' | 'full' | 'resetprop'
  try {
    await configStore.saveConfig()
    toast(t('settings.messages.default_mode_updated'))
  } catch {
    toast(t('settings.messages.save_failed'))
  }
}

async function onForceDenylistUnmountChange(value: boolean) {
  configStore.config.default_force_denylist_unmount = value
  try {
    await configStore.saveConfig()
    toast(t('common.saved'))
  } catch {
    toast(t('settings.messages.save_failed'))
  }
}

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
  const tempZipPath = createDeviceTempPath('device_faker_convert', '.zip')
  const tempOutputPath = createDeviceTempPath('device_faker_convert', '.toml')

  try {
    await uploadFileToDevice(file, tempZipPath)
    await convertZipOnDevice(tempZipPath, tempOutputPath)

    const outputContent = await readFile(tempOutputPath)
    if (!outputContent) {
      toast(t('settings.messages.read_failed'))
      return
    }

    const { templateData, defaultName } = parseFirstTemplateFromToml(outputContent)
    convertedTemplate.value = templateData
    convertedTemplateName.value = defaultName
    convertedContent.value = outputContent
    convertDialogVisible.value = true
  } catch (err) {
    toast(
      `${t('settings.messages.convert_failed')}: ${err instanceof Error ? err.message : String(err)}`
    )
    console.error(err)
  } finally {
    converting.value = false
    await execCommand(`rm -f ${shellQuote(tempZipPath)} ${shellQuote(tempOutputPath)}`).catch(
      () => {}
    )
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
  () => configStore.config.default_mode,
  (newMode: 'lite' | 'full' | 'resetprop' | undefined) => {
    if (newMode && defaultMode.value !== newMode) {
      defaultMode.value = newMode
    }
  }
)

watch(
  () => configStore.config.default_force_denylist_unmount,
  (newValue: boolean | undefined) => {
    const val = newValue || false
    if (defaultForceDenylistUnmount.value !== val) {
      defaultForceDenylistUnmount.value = val
    }
  }
)

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
  defaultMode.value = configStore.config.default_mode || 'lite'
  defaultForceDenylistUnmount.value = configStore.config.default_force_denylist_unmount || false
  debugMode.value = configStore.config.debug || false
})
</script>

<style scoped>
.settings-page {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  width: 100%;
  max-width: 100%;
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

/* Dialog styles (copied from TemplatePage for consistency) */
.template-dialog :deep(.el-dialog) {
  margin-top: 5vh !important;
  margin-bottom: 80px !important;
  max-height: calc(100vh - 80px - 10vh) !important;
  display: flex;
  flex-direction: column;
  background: rgba(255, 255, 255, 0.15) !important;
  backdrop-filter: blur(40px) saturate(150%) brightness(1.1);
  -webkit-backdrop-filter: blur(40px) saturate(150%) brightness(1.1);
  border: 1px solid rgba(255, 255, 255, 0.4);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.15);
}

@media (prefers-color-scheme: dark) {
  .template-dialog :deep(.el-dialog) {
    background: rgba(20, 20, 20, 0.6) !important;
    backdrop-filter: blur(40px) saturate(150%) brightness(0.9);
    -webkit-backdrop-filter: blur(40px) saturate(150%) brightness(0.9);
    border: 1px solid rgba(255, 255, 255, 0.15);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }
}

.template-dialog :deep(.el-dialog__body) {
  flex: 1;
  overflow-y: auto;
  padding: 1.5rem;
  background: transparent;
  /* 隐藏滚动条 */
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.template-dialog :deep(.el-dialog__body::-webkit-scrollbar) {
  display: none;
}

.template-dialog :deep(.el-dialog__header) {
  background: rgba(255, 255, 255, 0.1);
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
  border-bottom: 1px solid rgba(255, 255, 255, 0.2);
}

@media (prefers-color-scheme: dark) {
  .template-dialog :deep(.el-dialog__header) {
    background: rgba(0, 0, 0, 0.2);
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }
}

.template-dialog :deep(.el-dialog__footer) {
  padding: 1rem 1.5rem;
  border-top: 1px solid rgba(255, 255, 255, 0.2);
  background: rgba(255, 255, 255, 0.15);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  flex-shrink: 0;
}

@media (prefers-color-scheme: dark) {
  .template-dialog :deep(.el-dialog__footer) {
    border-top: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(0, 0, 0, 0.3);
  }
}

.template-dialog :deep(.el-overlay) {
  z-index: 2000 !important;
  backdrop-filter: blur(12px) saturate(120%) !important;
  -webkit-backdrop-filter: blur(12px) saturate(120%) !important;
  background-color: rgba(0, 0, 0, 0.25) !important;
}

@media (prefers-color-scheme: dark) {
  .template-dialog :deep(.el-overlay) {
    backdrop-filter: blur(12px) saturate(120%) !important;
    -webkit-backdrop-filter: blur(12px) saturate(120%) !important;
    background-color: rgba(0, 0, 0, 0.4) !important;
  }
}

.form-tip {
  font-size: 0.75rem;
  color: var(--text-secondary);
  margin-top: 0.25rem;
}
</style>
