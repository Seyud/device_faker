<template>
  <el-dialog
    v-model="visible"
    :title="t('templates.transfer.title')"
    width="90%"
    :close-on-click-modal="false"
    :append-to-body="true"
    :destroy-on-close="true"
    :z-index="2001"
    class="template-transfer-dialog"
    modal-class="template-transfer-dialog-modal"
  >
    <div class="transfer-tabs">
      <button
        v-for="tab in tabs"
        :key="tab.id"
        type="button"
        :class="['transfer-tab', { active: activeTab === tab.id }]"
        @click="activeTab = tab.id"
      >
        {{ tab.label }}
      </button>
    </div>

    <div v-if="activeTab === 'import'" class="transfer-panel">
      <div class="panel-section">
        <div class="section-header">
          <h3>{{ t('templates.transfer.import.input_label') }}</h3>
          <p>{{ t('templates.transfer.import.input_hint') }}</p>
        </div>
        <el-input
          v-model="importInput"
          type="textarea"
          :rows="12"
          resize="none"
          class="code-font"
          :placeholder="t('templates.transfer.import.input_placeholder')"
        />
      </div>

      <div class="panel-section preview-card">
        <div class="section-header">
          <h3>{{ t('templates.transfer.import.preview_label') }}</h3>
          <p v-if="importPreview.names.length > 0">
            {{ t('templates.transfer.import.detected', { count: importPreview.names.length }) }}
          </p>
        </div>

        <div v-if="!importInput.trim()" class="empty-state">
          {{ t('templates.transfer.import.empty') }}
        </div>
        <div v-else-if="importPreview.error" class="error-state">
          {{ importPreview.error }}
        </div>
        <div v-else class="name-list">
          <span v-for="name in importPreview.names" :key="name" class="name-chip">{{ name }}</span>
        </div>
      </div>
    </div>

    <div v-else-if="activeTab === 'export'" class="transfer-panel">
      <div class="panel-section">
        <div class="section-header">
          <h3>{{ t('templates.transfer.export.selection_label') }}</h3>
          <p>{{ t('templates.transfer.export.selection_hint') }}</p>
        </div>

        <div class="selection-toolbar">
          <button type="button" class="toolbar-btn" @click="selectAllExportTemplates">
            {{ t('templates.transfer.export.select_all') }}
          </button>
          <button type="button" class="toolbar-btn" @click="clearExportSelection">
            {{ t('templates.transfer.export.clear_all') }}
          </button>
        </div>

        <div v-if="templateEntries.length === 0" class="empty-state">
          {{ t('templates.transfer.export.empty') }}
        </div>
        <el-checkbox-group v-else v-model="selectedExportNames" class="checkbox-list">
          <div v-for="[name, template] in templateEntries" :key="name" class="checkbox-card">
            <el-checkbox :label="name">
              <div class="checkbox-copy">
                <strong>{{ name }}</strong>
                <span>{{ template.brand || template.manufacturer || template.model || '-' }}</span>
              </div>
            </el-checkbox>
          </div>
        </el-checkbox-group>
      </div>

      <div class="panel-section preview-card">
        <div class="section-header">
          <h3>{{ t('templates.transfer.export.preview_label') }}</h3>
          <p v-if="selectedExportNames.length > 0">
            {{ t('templates.transfer.export.selected', { count: selectedExportNames.length }) }}
          </p>
        </div>
        <el-input
          :model-value="exportContent"
          type="textarea"
          :rows="12"
          resize="none"
          readonly
          class="code-font"
          :placeholder="t('templates.transfer.export.empty_preview')"
        />
      </div>
    </div>

    <div v-else class="transfer-panel">
      <div class="panel-section">
        <div class="section-header">
          <h3>{{ t('templates.transfer.device.read_label') }}</h3>
          <p>{{ t('templates.transfer.device.read_hint') }}</p>
        </div>
        <button
          class="primary-action"
          type="button"
          :disabled="deviceLoading"
          @click="loadCurrentDevice"
        >
          {{
            deviceLoading
              ? t('templates.transfer.device.reading')
              : t('templates.transfer.device.read')
          }}
        </button>
      </div>

      <div v-if="deviceTemplate" class="panel-section">
        <div class="section-header">
          <h3>{{ t('templates.transfer.device.template_name_label') }}</h3>
        </div>
        <el-input
          v-model="deviceTemplateName"
          :placeholder="t('templates.transfer.device.template_name_placeholder')"
        />
      </div>

      <div class="panel-section preview-card">
        <div class="section-header">
          <h3>{{ t('templates.transfer.device.preview_label') }}</h3>
        </div>
        <el-input
          :model-value="deviceContent"
          type="textarea"
          :rows="12"
          resize="none"
          readonly
          class="code-font"
          :placeholder="t('templates.transfer.device.empty_preview')"
        />
      </div>
    </div>

    <template #footer>
      <div
        :class="[
          'dialog-footer',
          {
            'dialog-footer--device': activeTab === 'device',
            'dialog-footer--device-en': activeTab === 'device' && locale === 'en',
          },
        ]"
      >
        <el-button @click="visible = false">{{ t('common.cancel') }}</el-button>

        <template v-if="activeTab === 'import'">
          <el-button type="primary" :loading="importing" @click="handleImport">
            {{ t('templates.transfer.import.submit') }}
          </el-button>
        </template>

        <template v-else-if="activeTab === 'export'">
          <el-button type="primary" :disabled="!exportContent" @click="copyExportContent">
            {{ t('templates.transfer.export.copy') }}
          </el-button>
        </template>

        <template v-else>
          <el-button :disabled="!deviceContent" @click="copyDeviceContent">
            {{ t('templates.transfer.device.copy') }}
          </el-button>
          <el-button type="primary" :disabled="!deviceContent" @click="saveCurrentDeviceTemplate">
            {{ t('templates.transfer.device.save') }}
          </el-button>
        </template>
      </div>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { toast } from 'kernelsu-alt'
import { useConfigStore } from '../../stores/config'
import { useI18n } from '../../utils/i18n'
import { useLazyMessageBox } from '../../utils/elementPlus'
import { execCommand } from '../../utils/ksu'
import {
  copyTextToClipboard,
  parseFirstTemplateFromToml,
  parseTemplatesFromToml,
  stringifyTemplatesToToml,
} from '../../utils/templateTransfer'
import { convertPropertyTextToTemplate } from '../../utils/propsToTemplate'
import type { Template } from '../../types'

type TransferTab = 'import' | 'export' | 'device'

const props = defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{ 'update:modelValue': [boolean] }>()

const configStore = useConfigStore()
const { t, locale } = useI18n()
const getMessageBox = useLazyMessageBox()

const visible = computed({
  get: () => props.modelValue,
  set: (value: boolean) => emit('update:modelValue', value),
})

const tabs = computed(() => [
  { id: 'import' as TransferTab, label: t('templates.transfer.tabs.import') },
  { id: 'export' as TransferTab, label: t('templates.transfer.tabs.export') },
  { id: 'device' as TransferTab, label: t('templates.transfer.tabs.device') },
])

const activeTab = ref<TransferTab>('import')
const importInput = ref('')
const importing = ref(false)
const selectedExportNames = ref<string[]>([])
const deviceLoading = ref(false)
const deviceTemplate = ref<Template | null>(null)
const deviceTemplateName = ref('')

const templatesMap = computed(() => configStore.getTemplates())
const templateEntries = computed(() => Object.entries(templatesMap.value))

const importPreview = computed(() => {
  const rawInput = importInput.value.trim()
  if (!rawInput) {
    return {
      names: [],
      error: '',
    }
  }

  try {
    const templates = parseTemplatesFromToml(rawInput)
    return {
      names: Object.keys(templates),
      error: '',
    }
  } catch (error) {
    return {
      names: [],
      error: formatTransferError(error),
    }
  }
})

const exportContent = computed(() => {
  if (selectedExportNames.value.length === 0) {
    return ''
  }

  const selectedTemplates = selectedExportNames.value.reduce<Record<string, Template>>(
    (result, name) => {
      const template = templatesMap.value[name]
      if (template) {
        result[name] = template
      }
      return result
    },
    {}
  )

  if (Object.keys(selectedTemplates).length === 0) {
    return ''
  }

  try {
    return stringifyTemplatesToToml(selectedTemplates)
  } catch {
    return ''
  }
})

const deviceContent = computed(() => {
  const templateName = deviceTemplateName.value.trim()
  if (!templateName || !deviceTemplate.value) {
    return ''
  }

  try {
    return stringifyTemplatesToToml({
      [templateName]: deviceTemplate.value,
    })
  } catch {
    return ''
  }
})

function resetDialogState() {
  activeTab.value = 'import'
  importInput.value = ''
  importing.value = false
  selectedExportNames.value = []
  deviceLoading.value = false
  deviceTemplate.value = null
  deviceTemplateName.value = ''
}

function formatTransferError(error: unknown) {
  const message = error instanceof Error ? error.message : String(error)

  switch (message) {
    case 'Invalid TOML content':
      return t('templates.transfer.import.invalid_toml')
    case 'No valid templates found in TOML content':
    case 'No valid template data found':
      return t('templates.transfer.import.no_valid_templates')
    default:
      return message
  }
}

async function confirmOverwrite(message: string, title: string) {
  const messageBox = await getMessageBox()
  await messageBox.confirm(message, title, {
    confirmButtonText: t('common.confirm'),
    cancelButtonText: t('common.cancel'),
    type: 'warning',
    appendTo: 'body',
  })
}

async function handleImport() {
  if (importing.value) {
    return
  }

  let parsedTemplates: Record<string, Template>
  try {
    parsedTemplates = parseTemplatesFromToml(importInput.value)
  } catch (error) {
    toast(formatTransferError(error))
    return
  }

  importing.value = true

  try {
    const duplicates = Object.keys(parsedTemplates).filter((name) => name in templatesMap.value)

    if (duplicates.length > 0) {
      await confirmOverwrite(
        t('templates.transfer.import.overwrite_confirm', {
          count: duplicates.length,
          names: duplicates.join(', '),
        }),
        t('templates.transfer.import.overwrite_title')
      )
    }

    for (const [name, template] of Object.entries(parsedTemplates)) {
      configStore.setTemplate(name, template, { replace: true })
    }

    await configStore.saveConfig()
    toast(t('templates.transfer.import.success', { count: Object.keys(parsedTemplates).length }))
    visible.value = false
  } catch (error) {
    if (error === 'cancel') {
      return
    }

    const message = error instanceof Error ? error.message : String(error)
    toast(`${t('common.failed')}: ${message}`)
  } finally {
    importing.value = false
  }
}

function selectAllExportTemplates() {
  selectedExportNames.value = templateEntries.value.map(([name]) => name)
}

function clearExportSelection() {
  selectedExportNames.value = []
}

async function copyExportContent() {
  if (!exportContent.value) {
    toast(t('templates.transfer.export.none_selected'))
    return
  }

  const copied = await copyTextToClipboard(exportContent.value)
  toast(
    copied
      ? t('templates.transfer.export.copy_success')
      : t('templates.transfer.export.copy_failed')
  )
}

async function loadCurrentDevice() {
  if (deviceLoading.value) {
    return
  }

  deviceLoading.value = true

  try {
    let templateData: Template
    let defaultName: string

    if (import.meta.env?.DEV) {
      const { mockConfig } = await import('../../utils/mockData')
      ;({ templateData, defaultName } = parseFirstTemplateFromToml(mockConfig))
    } else {
      // 直接读 getprop 输出在前端转换：不再需要临时文件与 device_faker_cli
      const output = await execCommand('getprop')
      if (!output.trim()) {
        throw new Error(t('settings.messages.read_failed'))
      }
      ;({ template: templateData, name: defaultName } = convertPropertyTextToTemplate(output))
    }

    deviceTemplate.value = templateData
    deviceTemplateName.value = defaultName
    toast(t('templates.transfer.device.read_success'))
  } catch (error) {
    const message = error instanceof Error ? error.message : String(error)
    toast(`${t('templates.transfer.device.read_failed')}: ${message}`)
  } finally {
    deviceLoading.value = false
  }
}

async function copyDeviceContent() {
  if (!deviceContent.value) {
    toast(t('templates.transfer.device.empty'))
    return
  }

  const copied = await copyTextToClipboard(deviceContent.value)
  toast(
    copied
      ? t('templates.transfer.device.copy_success')
      : t('templates.transfer.device.copy_failed')
  )
}

async function saveCurrentDeviceTemplate() {
  const templateName = deviceTemplateName.value.trim()
  if (!templateName) {
    toast(t('templates.transfer.device.template_name_required'))
    return
  }

  if (!deviceTemplate.value) {
    toast(t('templates.transfer.device.empty'))
    return
  }

  try {
    if (templateName in templatesMap.value) {
      await confirmOverwrite(
        t('templates.transfer.device.overwrite_confirm', { name: templateName }),
        t('templates.transfer.device.overwrite_title')
      )
    }

    configStore.setTemplate(templateName, deviceTemplate.value, { replace: true })
    await configStore.saveConfig()
    toast(t('templates.transfer.device.save_success'))
    visible.value = false
  } catch (error) {
    if (error === 'cancel') {
      return
    }

    const message = error instanceof Error ? error.message : String(error)
    toast(`${t('common.failed')}: ${message}`)
  }
}

watch(
  () => props.modelValue,
  (dialogVisible) => {
    if (dialogVisible) {
      resetDialogState()
    }
  },
  { immediate: true }
)

watch(templateEntries, () => {
  selectedExportNames.value = selectedExportNames.value.filter((name) => name in templatesMap.value)
})
</script>

<style scoped>
.transfer-tabs {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 1rem;
  overflow-x: auto;
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.transfer-tabs::-webkit-scrollbar {
  display: none;
}

.transfer-tab {
  border: 1px solid var(--border);
  background: var(--card);
  color: var(--text-secondary);
  border-radius: 999px;
  padding: 0.625rem 1rem;
  font-size: 0.875rem;
  font-weight: 600;
  white-space: nowrap;
  transition: all 0.2s ease;
}

.transfer-tab.active {
  background: var(--primary);
  border-color: var(--primary);
  color: white;
}

.transfer-panel {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.panel-section {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.preview-card,
.checkbox-list,
.selection-toolbar,
.primary-action,
.transfer-tab,
.toolbar-btn {
  -webkit-tap-highlight-color: transparent;
}

.section-header h3 {
  margin: 0;
  font-size: 0.95rem;
  color: var(--text);
}

.section-header p {
  margin: 0.25rem 0 0;
  font-size: 0.8rem;
  color: var(--text-secondary);
  line-height: 1.5;
}

.empty-state,
.error-state {
  padding: 1rem;
  border-radius: 0.75rem;
  background: var(--background);
  font-size: 0.875rem;
  line-height: 1.5;
}

.empty-state {
  color: var(--text-secondary);
}

.error-state {
  color: #d14343;
}

.name-list {
  display: flex;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.name-chip {
  display: inline-flex;
  align-items: center;
  border-radius: 999px;
  padding: 0.4rem 0.75rem;
  background: var(--background);
  color: var(--text);
  font-size: 0.8rem;
}

.selection-toolbar {
  display: flex;
  gap: 0.5rem;
  flex-wrap: wrap;
}

.toolbar-btn,
.primary-action {
  border: 1px solid var(--border);
  border-radius: 0.75rem;
  background: var(--card);
  color: var(--text);
  padding: 0.75rem 1rem;
  font-size: 0.875rem;
  font-weight: 600;
}

.primary-action {
  background: var(--primary);
  border-color: var(--primary);
  color: white;
}

.primary-action:disabled {
  opacity: 0.7;
}

.checkbox-list {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
}

.checkbox-card {
  display: block;
  padding: 0.85rem 1rem;
  border-radius: 0.9rem;
  background: var(--background);
  border: 1px solid var(--border);
}

.checkbox-copy {
  display: flex;
  flex-direction: column;
  gap: 0.2rem;
  margin-left: 0.35rem;
}

.checkbox-copy strong {
  color: var(--text);
  font-size: 0.9rem;
}

.checkbox-copy span {
  color: var(--text-secondary);
  font-size: 0.8rem;
}

.dialog-footer {
  display: flex;
  justify-content: flex-end;
  gap: 0.75rem;
  flex-wrap: wrap;
}

.dialog-footer :deep(.el-button + .el-button) {
  margin-left: 0;
}

.dialog-footer--device {
  flex-wrap: nowrap;
}

.dialog-footer--device .el-button {
  flex: 1 1 0;
  min-width: 0;
}

.dialog-footer--device-en {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  width: 100%;
  justify-content: stretch;
}

.dialog-footer--device-en .el-button {
  flex: none;
  width: 100%;
}

.dialog-footer--device-en .el-button:last-child {
  grid-column: 1 / -1;
}

.code-font {
  font-family: monospace;
}
</style>

<style scoped>
.template-transfer-dialog :deep(.el-dialog) {
  margin-top: 5vh !important;
  margin-bottom: 80px !important;
  max-height: calc(100vh - 80px - 10vh) !important;
  display: flex;
  flex-direction: column;
  background: rgba(255, 255, 255, 0.15) !important;
  backdrop-filter: blur(40px) saturate(150%) brightness(1.1);
  border: 1px solid rgba(255, 255, 255, 0.4);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.15);
}

@media (prefers-color-scheme: dark) {
  .template-transfer-dialog :deep(.el-dialog) {
    background: rgba(20, 20, 20, 0.6) !important;
    backdrop-filter: blur(40px) saturate(150%) brightness(0.9);
    border: 1px solid rgba(255, 255, 255, 0.15);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }
}

.template-transfer-dialog :deep(.el-dialog__body) {
  flex: 1;
  overflow-y: auto;
  padding: 1.5rem;
  background: transparent;
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.template-transfer-dialog :deep(.el-dialog__body::-webkit-scrollbar) {
  display: none;
}

.template-transfer-dialog :deep(.el-dialog__header) {
  background: rgba(255, 255, 255, 0.1);
  backdrop-filter: blur(10px);
  border-bottom: 1px solid rgba(255, 255, 255, 0.2);
}

@media (prefers-color-scheme: dark) {
  .template-transfer-dialog :deep(.el-dialog__header) {
    background: rgba(0, 0, 0, 0.2);
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }
}

.template-transfer-dialog :deep(.el-dialog__footer) {
  padding: 1rem 1.5rem;
  border-top: 1px solid rgba(255, 255, 255, 0.2);
  background: rgba(255, 255, 255, 0.15);
  backdrop-filter: blur(20px);
  flex-shrink: 0;
}

@media (prefers-color-scheme: dark) {
  .template-transfer-dialog :deep(.el-dialog__footer) {
    border-top: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(0, 0, 0, 0.3);
  }
}
</style>

<style>
.template-transfer-dialog-modal {
  backdrop-filter: blur(12px) saturate(120%) !important;
  background-color: rgba(0, 0, 0, 0.25) !important;
}

@media (prefers-color-scheme: dark) {
  .template-transfer-dialog-modal {
    backdrop-filter: blur(12px) saturate(120%) !important;
    background-color: rgba(0, 0, 0, 0.4) !important;
  }
}
</style>
