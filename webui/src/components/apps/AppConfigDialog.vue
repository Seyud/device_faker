<template>
  <el-dialog
    v-model="visible"
    :title="dialogTitle"
    width="90%"
    :close-on-click-modal="false"
    :append-to-body="true"
    :destroy-on-close="true"
    :z-index="2001"
    class="app-config-dialog"
    modal-class="app-config-modal"
  >
    <div class="config-options">
      <el-radio-group v-model="configMode">
        <el-radio label="template">{{ t('apps.dialog.mode_template') }}</el-radio>
        <el-radio label="custom">{{ t('apps.dialog.mode_custom') }}</el-radio>
        <el-radio label="remove">{{ t('apps.dialog.mode_remove') }}</el-radio>
      </el-radio-group>
    </div>

    <div v-if="configMode === 'template'" class="template-selector">
      <el-input
        v-model="templateSearch"
        :placeholder="t('apps.dialog.search_template_placeholder')"
        prefix-icon="Search"
        clearable
        class="template-search"
      />
      <el-select
        v-model="selectedTemplate"
        :placeholder="t('apps.dialog.select_template_placeholder')"
        :no-data-text="templateNoDataText"
        :no-match-text="templateNoMatchText"
        filterable
        style="width: 100%"
      >
        <el-option
          v-for="option in filteredTemplateOptions"
          :key="option.name"
          :label="option.label"
          :value="option.name"
        />
      </el-select>
    </div>

    <div v-if="configMode === 'custom'" class="custom-config">
      <el-form :model="customFormData" label-width="120px" label-position="top">
        <el-form-item :label="t('templates.fields.manufacturer')">
          <el-input
            v-model="customFormData.manufacturer"
            :placeholder="t('templates.placeholders.manufacturer')"
          />
        </el-form-item>
        <el-form-item :label="t('templates.fields.brand')">
          <el-input
            v-model="customFormData.brand"
            :placeholder="t('templates.placeholders.brand')"
          />
        </el-form-item>
        <el-form-item :label="t('templates.fields.model')">
          <el-input
            v-model="customFormData.model"
            :placeholder="t('templates.placeholders.model')"
          />
        </el-form-item>
        <el-form-item :label="t('templates.fields.device')">
          <el-input
            v-model="customFormData.device"
            :placeholder="t('templates.placeholders.device')"
          />
        </el-form-item>
        <el-form-item :label="t('templates.fields.product')">
          <el-input
            v-model="customFormData.product"
            :placeholder="t('templates.placeholders.product')"
          />
        </el-form-item>
        <el-form-item :label="t('templates.fields.name_field')">
          <el-input
            v-model="customFormData.name"
            :placeholder="t('templates.placeholders.name_field')"
          />
        </el-form-item>
        <el-form-item :label="t('templates.fields.market_name')">
          <el-input
            v-model="customFormData.marketname"
            :placeholder="t('templates.placeholders.market_name')"
          />
        </el-form-item>
        <el-form-item :label="t('templates.fields.fingerprint')">
          <el-input
            v-model="customFormData.fingerprint"
            type="textarea"
            :rows="3"
            :placeholder="t('templates.placeholders.fingerprint')"
          />
        </el-form-item>
        <el-form-item :label="t('templates.fields.mode')">
          <el-select
            v-model="customFormData.mode"
            :placeholder="t('templates.placeholders.mode')"
            clearable
            popper-class="mode-select-popper"
            style="width: 100%"
          >
            <el-option :label="t('templates.options.mode_lite')" value="lite" />
            <el-option :label="t('templates.options.mode_full')" value="full" />
            <el-option :label="t('templates.options.mode_resetprop')" value="resetprop" />
          </el-select>
        </el-form-item>

        <el-form-item
          v-if="
            customFormData.mode === 'resetprop' ||
            (!customFormData.mode && configStore.config.default_mode === 'resetprop')
          "
          :label="t('templates.fields.characteristics')"
        >
          <el-input
            v-model="customFormData.characteristics"
            :placeholder="t('templates.placeholders.characteristics')"
          />
        </el-form-item>

        <el-form-item :label="t('templates.fields.force_denylist_unmount')">
          <el-select
            v-model="customFormData.force_denylist_unmount"
            :placeholder="t('common.default')"
            style="width: 100%"
          >
            <el-option :label="t('common.default')" :value="undefined" />
            <el-option :label="t('common.enabled')" :value="true" />
            <el-option :label="t('common.disabled')" :value="false" />
          </el-select>
        </el-form-item>
      </el-form>

      <!-- 元数据只读显示区域 -->
      <div v-if="hasMetaData" class="config-meta">
        <div v-if="customMetaData.version || customMetaData.version_code" class="meta-item">
          <span class="meta-label">{{ t('templates.labels.version') }}:</span>
          <span class="meta-value">
            {{ customMetaData.version || '' }}
            <span v-if="customMetaData.version_code" class="version-code"
              >({{ customMetaData.version_code }})</span
            >
          </span>
        </div>
        <div v-if="customMetaData.author" class="meta-item">
          <span class="meta-label">{{ t('templates.labels.author') }}:</span>
          <span class="meta-value">{{ customMetaData.author }}</span>
        </div>
        <div v-if="customMetaData.description" class="meta-item meta-description">
          <span class="meta-label">{{ t('templates.labels.description') }}:</span>
          <span class="meta-value">{{ customMetaData.description }}</span>
        </div>
      </div>
    </div>

    <div v-if="configMode === 'remove'" class="remove-hint">
      <p>{{ t('apps.dialog.remove_hint') }}</p>
    </div>

    <template #footer>
      <el-button @click="visible = false">{{ t('common.cancel') }}</el-button>
      <el-button type="primary" @click="saveAppConfig">{{ t('common.confirm') }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useConfigStore } from '../../stores/config'
import { useI18n } from '../../utils/i18n'
import { toast } from 'kernelsu-alt'
import type { InstalledApp, AppConfig } from '../../types'

interface TemplateOption {
  name: string
  label: string
  searchable: string
}

const props = defineProps<{
  modelValue: boolean
  app: InstalledApp | null
}>()

const emit = defineEmits<{ 'update:modelValue': [boolean]; saved: [] }>()

const configStore = useConfigStore()
const { t } = useI18n()

const templates = computed(() => configStore.getTemplates())
const visible = computed({
  get: () => props.modelValue,
  set: (val: boolean) => emit('update:modelValue', val),
})

const configMode = ref<'template' | 'custom' | 'remove'>('template')
const selectedTemplate = ref('')
const templateSearch = ref('')
const customFormData = ref({
  manufacturer: '',
  brand: '',
  model: '',
  device: '',
  product: '',
  name: '',
  marketname: '',
  fingerprint: '',
  characteristics: '',
  force_denylist_unmount: undefined as boolean | undefined,
  mode: undefined as 'lite' | 'full' | 'resetprop' | undefined,
})

// 元数据字段（只读显示）
const customMetaData = ref({
  version: '' as string | undefined,
  version_code: undefined as number | undefined,
  author: '' as string | undefined,
  description: '' as string | undefined,
})

// 判断是否有元数据需要显示
const hasMetaData = computed(() => {
  return (
    customMetaData.value.version ||
    customMetaData.value.version_code ||
    customMetaData.value.author ||
    customMetaData.value.description
  )
})

const dialogTitle = computed(() =>
  t('apps.dialog.config_title', { name: props.app?.appName || '' })
)

const templateOptions = computed<TemplateOption[]>(() => {
  const allTemplates = templates.value || {}

  return Object.entries(allTemplates).map(([name, template]) => {
    const label = `${name} - ${template.brand || ''} ${template.model || ''}`
      .replace(/\s+/g, ' ')
      .trim()

    const searchable = [
      name,
      template.brand,
      template.model,
      template.marketname,
      template.device,
      template.product,
    ]
      .filter(Boolean)
      .map((part) => String(part).toLowerCase())
      .join(' ')

    return {
      name,
      label: label || name,
      searchable,
    }
  })
})

const filteredTemplateOptions = computed(() => {
  const keyword = templateSearch.value.trim().toLowerCase()
  if (!keyword) return templateOptions.value

  return templateOptions.value.filter((option) => option.searchable.includes(keyword))
})

const templateNoDataText = computed(() =>
  templateSearch.value.trim() ? t('apps.dialog.search_no_result') : t('apps.dialog.no_templates')
)
const templateNoMatchText = computed(() => t('apps.dialog.search_no_result'))

function getAppConfig(packageName: string) {
  return configStore.getPackageConfig(packageName)
}

function syncFromExistingConfig() {
  if (!props.app) return

  templateSearch.value = ''
  const existingConfig = getAppConfig(props.app.packageName)

  if (existingConfig) {
    if ('source' in existingConfig) {
      configMode.value = 'template'
      selectedTemplate.value = existingConfig.source
      // 清空元数据（模板模式下不在此处显示）
      customMetaData.value = {
        version: undefined,
        version_code: undefined,
        author: undefined,
        description: undefined,
      }
    } else {
      configMode.value = 'custom'
      customFormData.value = {
        manufacturer: existingConfig.manufacturer || '',
        brand: existingConfig.brand || '',
        model: existingConfig.model || '',
        device: existingConfig.device || '',
        product: existingConfig.product || '',
        name: existingConfig.name || '',
        marketname: existingConfig.marketname || '',
        fingerprint: existingConfig.fingerprint || '',
        characteristics: existingConfig.characteristics || '',
        force_denylist_unmount: existingConfig.force_denylist_unmount,
        mode: existingConfig.mode as 'lite' | 'full' | 'resetprop' | undefined,
      }
      // 同步元数据字段（只读显示）
      customMetaData.value = {
        version: existingConfig.version,
        version_code: existingConfig.version_code,
        author: existingConfig.author,
        description: existingConfig.description,
      }
    }
  } else {
    configMode.value = 'template'
    selectedTemplate.value = ''
    customFormData.value = {
      manufacturer: '',
      brand: '',
      model: '',
      device: '',
      product: '',
      name: '',
      marketname: '',
      fingerprint: '',
      characteristics: '',
      force_denylist_unmount: undefined,
      mode: undefined,
    }
    customMetaData.value = {
      version: undefined,
      version_code: undefined,
      author: undefined,
      description: undefined,
    }
  }
}

async function saveAppConfig() {
  if (!props.app) return

  if (configMode.value === 'remove') {
    configStore.deleteApp(props.app.packageName)
    const templateMap = configStore.getTemplates()
    for (const [name, template] of Object.entries(templateMap)) {
      if (template.packages?.includes(props.app.packageName)) {
        template.packages = template.packages.filter((p: string) => p !== props.app!.packageName)
        configStore.setTemplate(name, template)
      }
    }
  } else if (configMode.value === 'template') {
    if (!selectedTemplate.value) {
      toast(t('apps.messages.select_template'))
      return
    }

    // Remove from other templates to avoid conflicts
    const allTemplates = configStore.getTemplates()
    for (const [name, tmpl] of Object.entries(allTemplates)) {
      if (name !== selectedTemplate.value && tmpl.packages?.includes(props.app.packageName)) {
        tmpl.packages = tmpl.packages.filter((p: string) => p !== props.app!.packageName)
        configStore.setTemplate(name, tmpl)
      }
    }

    const template = templates.value[selectedTemplate.value]
    if (!template.packages) {
      template.packages = []
    }
    if (!template.packages.includes(props.app.packageName)) {
      template.packages.push(props.app.packageName)
      configStore.setTemplate(selectedTemplate.value, template)
    }
  } else if (configMode.value === 'custom') {
    const appConfig: AppConfig = {
      package: props.app.packageName,
      manufacturer: customFormData.value.manufacturer,
      brand: customFormData.value.brand,
      model: customFormData.value.model,
      device: customFormData.value.device,
      product: customFormData.value.product,
      name: customFormData.value.name,
      marketname: customFormData.value.marketname,
      fingerprint: customFormData.value.fingerprint,
      characteristics: customFormData.value.characteristics,
      force_denylist_unmount: customFormData.value.force_denylist_unmount,
      mode: customFormData.value.mode,
    }
    configStore.setApp(appConfig)
  }

  try {
    await configStore.saveConfig()
    toast(t('apps.messages.saved'))
    visible.value = false
    emit('saved')
  } catch {
    toast(t('common.failed'))
  }
}

watch(
  () => [props.app, visible.value],
  ([, dialogVisible]) => {
    if (dialogVisible) {
      syncFromExistingConfig()
    }
  },
  { immediate: true }
)
</script>

<style scoped>
.config-options {
  margin-bottom: 1.5rem;
}

.template-selector,
.custom-config {
  margin-top: 1rem;
}

.template-search {
  margin-bottom: 0.75rem;
}

.remove-hint {
  padding: 1.5rem;
  text-align: center;
  color: var(--text-secondary);
}

.config-meta {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  margin-top: 1rem;
  padding-top: 0.75rem;
  border-top: 1px solid var(--border);
}

.meta-item {
  display: flex;
  gap: 0.5rem;
  font-size: 0.8125rem;
}

.meta-label {
  color: var(--text-secondary);
  min-width: 50px;
  flex-shrink: 0;
}

.meta-value {
  color: var(--text);
  flex: 1;
  word-break: break-all;
}

.version-code {
  color: var(--text-secondary);
  font-size: 0.75rem;
}

.meta-description .meta-value {
  font-size: 0.75rem;
  color: var(--text-secondary);
  line-height: 1.4;
}

.meta-description .meta-label {
  font-size: 0.75rem;
  line-height: 1.4;
}
</style>

<style scoped>
.app-config-dialog :deep(.el-dialog) {
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
  .app-config-dialog :deep(.el-dialog) {
    background: rgba(20, 20, 20, 0.6) !important;
    backdrop-filter: blur(40px) saturate(150%) brightness(0.9);
    -webkit-backdrop-filter: blur(40px) saturate(150%) brightness(0.9);
    border: 1px solid rgba(255, 255, 255, 0.15);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }
}

.app-config-dialog :deep(.el-dialog__body) {
  flex: 1;
  overflow-y: auto;
  padding: 1.5rem;
  padding-bottom: 2rem;
  max-height: calc(100vh - 200px);
  background: transparent;
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.app-config-dialog :deep(.el-dialog__body::-webkit-scrollbar) {
  display: none;
}

.app-config-dialog :deep(.el-dialog__header) {
  background: rgba(255, 255, 255, 0.1);
  backdrop-filter: blur(10px);
  -webkit-backdrop-filter: blur(10px);
  border-bottom: 1px solid rgba(255, 255, 255, 0.2);
}

@media (prefers-color-scheme: dark) {
  .app-config-dialog :deep(.el-dialog__header) {
    background: rgba(0, 0, 0, 0.2);
    border-bottom: 1px solid rgba(255, 255, 255, 0.1);
  }
}

.app-config-dialog :deep(.el-dialog__footer) {
  padding: 1rem 1.5rem;
  border-top: 1px solid rgba(255, 255, 255, 0.2);
  background: rgba(255, 255, 255, 0.15);
  backdrop-filter: blur(20px);
  -webkit-backdrop-filter: blur(20px);
  flex-shrink: 0;
}

@media (prefers-color-scheme: dark) {
  .app-config-dialog :deep(.el-dialog__footer) {
    border-top: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(0, 0, 0, 0.3);
  }
}

.app-config-dialog :deep(.el-overlay) {
  z-index: 2000 !important;
  backdrop-filter: blur(8px) brightness(0.7) !important;
  -webkit-backdrop-filter: blur(8px) brightness(0.7) !important;
  background-color: rgba(0, 0, 0, 0.6) !important;
}

@media (prefers-color-scheme: dark) {
  .app-config-dialog :deep(.el-overlay) {
    backdrop-filter: blur(8px) brightness(0.5) !important;
    -webkit-backdrop-filter: blur(8px) brightness(0.5) !important;
    background-color: rgba(0, 0, 0, 0.7) !important;
  }
}
</style>

<style>
.app-config-modal {
  backdrop-filter: blur(12px) saturate(120%) !important;
  -webkit-backdrop-filter: blur(12px) saturate(120%) !important;
  background-color: rgba(0, 0, 0, 0.25) !important;
}

@media (prefers-color-scheme: dark) {
  .app-config-modal {
    backdrop-filter: blur(12px) saturate(120%) !important;
    -webkit-backdrop-filter: blur(12px) saturate(120%) !important;
    background-color: rgba(0, 0, 0, 0.4) !important;
  }
}
</style>
