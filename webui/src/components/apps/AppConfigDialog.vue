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
    <!-- 配置类型标签页 -->
    <div class="config-tabs">
      <div
        class="config-tab"
        :class="{ active: activeTab === 'custom', configured: hasCustomConfig }"
        @click="activeTab = 'custom'"
      >
        <el-checkbox
          :model-value="hasCustomConfig"
          @click.stop
          @change="(val: boolean) => handleConfigToggle('custom', val)"
        />
        <span class="tab-label">{{ t('apps.dialog.mode_custom') }}</span>
      </div>
      <div
        class="config-tab"
        :class="{
          active: activeTab === 'template',
          configured: hasTemplateConfig && !hasCustomConfig,
        }"
        @click="activeTab = 'template'"
      >
        <el-checkbox
          :model-value="hasTemplateConfig"
          @click.stop
          @change="(val: boolean) => handleConfigToggle('template', val)"
        />
        <span class="tab-label">{{ t('apps.dialog.mode_template') }}</span>
      </div>
    </div>

    <!-- 模板配置界面 -->
    <div v-if="activeTab === 'template'" class="template-selector">
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

      <!-- 模板配置操作按钮 -->
      <div class="config-actions">
        <el-button v-if="hasTemplateConfig" type="danger" @click="removeTemplateConfig">
          {{ t('apps.dialog.remove_template_config') }}
        </el-button>
      </div>
    </div>

    <!-- 自定义配置界面 -->
    <div v-if="activeTab === 'custom'" class="custom-config">
      <el-form :model="formData" label-width="120px" label-position="top">
        <DeviceFakerFormFields />
      </el-form>

      <!-- 自定义配置操作按钮 -->
      <div class="config-actions">
        <el-button v-if="hasCustomConfig" type="danger" @click="removeCustomConfig">
          {{ t('apps.dialog.remove_custom_config') }}
        </el-button>
      </div>
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
import { provideDeviceFakerForm } from '../../composables/useDeviceFakerForm'
import DeviceFakerFormFields from '../shared/DeviceFakerFormFields.vue'

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

const activeTab = ref<'template' | 'custom'>('custom')

const hasCustomConfig = ref(false)
const hasTemplateConfig = ref(false)

const originalCustomConfig = ref<AppConfig | null>(null)
const originalTemplateName = ref<string | null>(null)

const selectedTemplate = ref('')
const templateSearch = ref('')

const { formData, resetForm, fillFromAppConfig, toAppConfig } = provideDeviceFakerForm()

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
      template.build_id,
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

function handleConfigToggle(type: 'custom' | 'template', enabled: boolean) {
  if (type === 'custom') {
    hasCustomConfig.value = enabled
    if (enabled) {
      activeTab.value = 'custom'
    }
  } else {
    hasTemplateConfig.value = enabled
    if (enabled) {
      activeTab.value = 'template'
      if (!selectedTemplate.value && templateOptions.value.length > 0) {
        selectedTemplate.value = templateOptions.value[0].name
      }
    } else {
      selectedTemplate.value = ''
    }
  }
}

function syncFromExistingConfig() {
  if (!props.app) return

  templateSearch.value = ''

  const appConfig = configStore.getApps().find((a) => a.package === props.app!.packageName)
  if (appConfig) {
    hasCustomConfig.value = true
    originalCustomConfig.value = { ...appConfig }
    fillFromAppConfig(appConfig)
  } else {
    hasCustomConfig.value = false
    originalCustomConfig.value = null
    resetForm()
  }

  let foundTemplateName: string | null = null
  for (const [name, template] of Object.entries(templates.value)) {
    if (template.packages?.includes(props.app.packageName)) {
      foundTemplateName = name
      break
    }
  }

  if (foundTemplateName) {
    hasTemplateConfig.value = true
    originalTemplateName.value = foundTemplateName
    selectedTemplate.value = foundTemplateName
  } else {
    hasTemplateConfig.value = false
    originalTemplateName.value = null
    selectedTemplate.value = ''
  }

  if (hasCustomConfig.value) {
    activeTab.value = 'custom'
  } else if (hasTemplateConfig.value) {
    activeTab.value = 'template'
  } else {
    activeTab.value = 'custom'
  }
}

async function removeCustomConfig() {
  if (!props.app) return

  configStore.deleteApp(props.app.packageName)
  hasCustomConfig.value = false
  originalCustomConfig.value = null
  resetForm()

  try {
    await configStore.saveConfig()
    toast(t('apps.messages.custom_config_removed'))
  } catch {
    toast(t('common.failed'))
  }
}

async function removeTemplateConfig() {
  if (!props.app) return

  const allTemplates = configStore.getTemplates()
  for (const [name, template] of Object.entries(allTemplates)) {
    if (template.packages?.includes(props.app.packageName)) {
      template.packages = template.packages.filter((p: string) => p !== props.app!.packageName)
      configStore.setTemplate(name, template)
    }
  }

  hasTemplateConfig.value = false
  originalTemplateName.value = null
  selectedTemplate.value = ''

  try {
    await configStore.saveConfig()
    toast(t('apps.messages.template_config_removed'))
  } catch {
    toast(t('common.failed'))
  }
}

async function saveAppConfig() {
  if (!props.app) return

  if (hasTemplateConfig.value) {
    if (!selectedTemplate.value) {
      toast(t('apps.messages.select_template'))
      return
    }

    if (originalTemplateName.value && originalTemplateName.value !== selectedTemplate.value) {
      const oldTemplate = templates.value[originalTemplateName.value]
      if (oldTemplate && oldTemplate.packages) {
        oldTemplate.packages = oldTemplate.packages.filter(
          (p: string) => p !== props.app!.packageName
        )
        configStore.setTemplate(originalTemplateName.value, oldTemplate)
      }
    }

    const template = templates.value[selectedTemplate.value]
    if (template) {
      if (!template.packages) {
        template.packages = []
      }
      if (!template.packages.includes(props.app.packageName)) {
        template.packages.push(props.app.packageName)
        configStore.setTemplate(selectedTemplate.value, template)
      }
    }
  } else {
    const allTemplates = configStore.getTemplates()
    for (const [name, template] of Object.entries(allTemplates)) {
      if (template.packages?.includes(props.app.packageName)) {
        template.packages = template.packages.filter((p: string) => p !== props.app!.packageName)
        configStore.setTemplate(name, template)
      }
    }
  }

  if (hasCustomConfig.value) {
    const appConfig = toAppConfig(props.app.packageName)
    configStore.setApp(appConfig)
  } else {
    configStore.deleteApp(props.app.packageName)
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
.config-tabs {
  display: flex;
  gap: 0.5rem;
  margin-bottom: 1.5rem;
  border-bottom: 1px solid var(--border);
  padding-bottom: 0.5rem;
}

.config-tab {
  position: relative;
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 1rem;
  border-radius: 0.375rem;
  cursor: pointer;
  transition: all 0.2s ease;
}

.config-tab:hover {
  background: var(--hover);
}

.config-tab.active {
  background: color-mix(in srgb, var(--primary-color, #409eff) 12%, transparent);
}

.config-tab.configured::after {
  content: '';
  position: absolute;
  top: 0.3rem;
  right: 0.3rem;
  width: 0.4rem;
  height: 0.4rem;
  border-radius: 999px;
  background: var(--success-color, #67c23a);
}

.tab-label {
  font-size: 0.875rem;
  color: var(--text);
}

.config-tab.active .tab-label {
  color: var(--primary-color, #409eff);
  font-weight: 600;
}

.template-selector,
.custom-config {
  margin-top: 1rem;
}

.template-search {
  margin-bottom: 0.75rem;
}

.config-actions {
  margin-top: 1.5rem;
  padding-top: 1rem;
  border-top: 1px solid var(--border);
  display: flex;
  justify-content: flex-start;
}

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
