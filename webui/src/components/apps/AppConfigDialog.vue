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

    <!-- 生效范围：明示命中的配置键作用到谁（仅主应用 / 主应用+分身回退 / 仅该分身） -->
    <p v-if="scopeText" class="scope-hint">{{ scopeText }}</p>

    <!-- 继承提示：分身当前只是回退命中主应用的裸包名配置，本实例尚无自有条目 -->
    <p v-if="inheritedHintVisible" class="inherited-hint">
      {{ t('apps.dialog.inherited_hint', { base: inheritedBase }) }}
    </p>

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
import {
  configKeyScope,
  configWriteKey,
  filterOutOwnConfigKeys,
  findOwnConfigKey,
  ownConfigKeys,
  parsePackageUser,
} from '../../utils/package'

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
/** 运行时命中的配置键（含可能继承主应用的裸 base 回退键），写回时会换算成本实例自有键 */
const resolvedConfigKey = ref<string | null>(null)
/** 当前配置是否只是「回退命中主应用的裸包名条目」（分身未单独配置） */
const customInherited = ref(false)
const templateInherited = ref(false)

const inheritedBase = computed(() => parsePackageUser(props.app?.packageName || '').base)
const inheritedHintVisible = computed(() => customInherited.value || templateInherited.value)

/**
 * 生效范围提示：把命中的配置键作用域写清楚。
 * 继承态不显示（此时由 inheritedHint 说明「继承自主应用」，避免两句话重复）。
 */
const scopeText = computed(() => {
  const key = resolvedConfigKey.value
  if (!key || inheritedHintVisible.value) {
    return ''
  }

  const scope = configKeyScope(key)
  if (scope === 'user0') {
    return t('apps.dialog.scope_user0')
  }
  if (scope === 'fallback') {
    return t('apps.dialog.scope_fallback')
  }

  return t('apps.dialog.scope_clone', { uid: String(parsePackageUser(key).userId) })
})

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

/**
 * 只移除本实例自有键。分身的裸 base 键是「主应用的键 + 分身回退键」，绝不能由分身编辑删除
 * （历史 bug：分身改模板把主应用的 packages 一并清空，主应用变「未配置」）。
 */
function removeOwnAppEntries(packageName: string): boolean {
  let removed = false
  for (const key of ownConfigKeys(packageName)) {
    if (!configStore.getApps().some((app) => app.package === key)) continue
    configStore.deleteApp(key)
    removed = true
  }
  return removed
}

function removeOwnTemplateEntries(packageName: string): boolean {
  let removed = false
  for (const { name, template } of configStore.templateEntries) {
    if (!template.packages?.length) continue
    const packages = filterOutOwnConfigKeys(template.packages, packageName)
    if (packages.length === template.packages.length) continue
    configStore.setTemplate(name, { ...template, packages })
    removed = true
  }
  return removed
}

function hasOwnAppEntry(packageName: string): boolean {
  const own = new Set(ownConfigKeys(packageName))
  return configStore.getApps().some((app) => own.has(app.package))
}

function hasOwnTemplateEntry(packageName: string): boolean {
  return configStore.templateEntries.some(
    ({ template }) => findOwnConfigKey(template.packages, packageName) !== null
  )
}

/** 配置只继承自主应用的裸包名条目：无法在本实例内移除，提示用户去主应用处理。 */
function notifyInherited() {
  toast(t('apps.messages.inherited_config', { base: inheritedBase.value }))
}

/**
 * 表单当前值与原配置是否等价（忽略空串/空对象/空数组，避免 fillFromAppConfig→toAppConfig
 * 往返把 undefined 写成 '' 造成假差异）。用于「没改就不落盘」：分身仅继承主应用配置时，
 * 打开对话框直接点确定不应静默把继承配置固化成本实例条目（否则此后不再跟随主应用改动）。
 */
function isFormEquivalentToAppConfig(original: AppConfig, packageName: string): boolean {
  const canonical = (config: AppConfig): string =>
    JSON.stringify(
      Object.entries(config)
        .filter(([key, value]) => {
          if (key === 'package') return false
          if (value === undefined || value === null || value === '') return false
          if (Array.isArray(value)) return value.length > 0
          if (typeof value === 'object') return Object.keys(value as object).length > 0
          return true
        })
        .sort(([a], [b]) => (a < b ? -1 : a > b ? 1 : 0))
    )

  return canonical(toAppConfig(packageName)) === canonical(original)
}

function syncFromExistingConfig() {
  if (!props.app) return

  templateSearch.value = ''
  const packageName = props.app.packageName

  // 与 Rust 运行时严格同源（src/lib.rs + src/config.rs get_merged_config）：
  // 键优先（pkg@uid → 裸 pkg），命中第一个键即停；同一键内 [[apps]] 先于模板。
  // 不能用「先按模板书写顺序遍历」或「两类配置各自跨键扫描」：
  //  - 前者让先书写模板里的裸回退键遮蔽后书写模板的 @uid 精确键（分身显示主应用模板）；
  //  - 后者会把更靠后键上的另一类配置当成生效项（例：分身在 @uid 键被模板命中时，
  //    裸 base 上的 [[apps]] 条目对分身并不生效，不能显示成「继承的自定义配置」）。
  const matches = configStore.resolvePackageMatchesFor(packageName)
  const appMatch = matches?.appMatch ?? null
  const templateMatch = matches?.templateMatch ?? null

  resolvedConfigKey.value = matches?.key ?? null

  if (appMatch) {
    hasCustomConfig.value = true
    customInherited.value = appMatch.inherited
    originalCustomConfig.value = { ...appMatch.app }
    fillFromAppConfig(appMatch.app)
  } else {
    hasCustomConfig.value = false
    customInherited.value = false
    originalCustomConfig.value = null
    resetForm()
  }

  if (templateMatch) {
    hasTemplateConfig.value = true
    templateInherited.value = templateMatch.inherited
    originalTemplateName.value = templateMatch.templateName
    selectedTemplate.value = templateMatch.templateName
  } else {
    hasTemplateConfig.value = false
    templateInherited.value = false
    originalTemplateName.value = null
    selectedTemplate.value = ''
  }

  // 默认展示运行时真正生效的那一种：同一个键内 [[apps]] 先于模板
  activeTab.value = appMatch ? 'custom' : templateMatch ? 'template' : 'custom'
}

async function removeCustomConfig() {
  if (!props.app) return

  const packageName = props.app.packageName
  // 继承来的条目没有本实例自有键：删不掉，也不能删（会连主应用一起废掉）
  if (!hasOwnAppEntry(packageName)) {
    notifyInherited()
    return
  }

  removeOwnAppEntries(packageName)
  hasCustomConfig.value = false
  customInherited.value = false
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

  const packageName = props.app.packageName
  if (!hasOwnTemplateEntry(packageName)) {
    notifyInherited()
    return
  }

  removeOwnTemplateEntries(packageName)
  hasTemplateConfig.value = false
  templateInherited.value = false
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

  const packageName = props.app.packageName
  const writeKey = configWriteKey(packageName, resolvedConfigKey.value)

  // 只是继承主应用的条目时，「取消勾选/移除」在本实例里没有可删的东西：不阻断保存其他改动，
  // 只在结束后提示原因（裸 base 键绝不能由分身删改）。
  const keepsInheritedTemplate =
    !hasTemplateConfig.value && templateInherited.value && !hasOwnTemplateEntry(packageName)
  const keepsInheritedCustom =
    !hasCustomConfig.value && customInherited.value && !hasOwnAppEntry(packageName)

  if (hasTemplateConfig.value) {
    if (!selectedTemplate.value) {
      toast(t('apps.messages.select_template'))
      return
    }

    const targetName = selectedTemplate.value
    if (!templates.value[targetName]) {
      toast(t('apps.messages.select_template'))
      return
    }

    // 只有「换了模板」或「新启用模板配置」才写盘：原模板未变时不重复写，
    // 尤其不能把继承自主应用的裸 base 键误固化成该分身的自有条目。
    const templateSelectionChanged =
      !originalTemplateName.value || originalTemplateName.value !== targetName

    if (originalTemplateName.value && templateSelectionChanged) {
      // 换模板：只把本实例自有键从旧模板剔除；主应用的裸 base 键与别的分身键原样保留
      const oldTemplate = templates.value[originalTemplateName.value]
      if (oldTemplate?.packages?.length) {
        const packages = filterOutOwnConfigKeys(oldTemplate.packages, packageName)
        if (packages.length !== oldTemplate.packages.length) {
          configStore.setTemplate(originalTemplateName.value, { ...oldTemplate, packages })
        }
      }
    }

    if (templateSelectionChanged) {
      const targetTemplate = templates.value[targetName]
      if (targetTemplate) {
        const packages = targetTemplate.packages ? [...targetTemplate.packages] : []
        // 复用本实例已有键；继承来的裸 base 键不算命中，分身必须写 pkg@uid
        const existingOwnKey = findOwnConfigKey(packages, packageName)
        const keyToWrite = existingOwnKey ?? writeKey
        if (!packages.includes(keyToWrite)) {
          packages.push(keyToWrite)
          configStore.setTemplate(targetName, { ...targetTemplate, packages })
        }
      }
    }
  } else {
    removeOwnTemplateEntries(packageName)
  }

  if (hasCustomConfig.value) {
    // 继承自主应用的条目：只有表单实际改动才落成该分身的自有条目（没改就不写盘）
    const inheritedAndUnchanged =
      customInherited.value &&
      originalCustomConfig.value !== null &&
      isFormEquivalentToAppConfig(originalCustomConfig.value, packageName)
    if (!inheritedAndUnchanged) {
      configStore.setApp(toAppConfig(writeKey))
    }
  } else {
    removeOwnAppEntries(packageName)
  }

  try {
    await configStore.saveConfig()
    if (keepsInheritedTemplate || keepsInheritedCustom) {
      // 其他改动已保存；被继承的那一份无法在本实例内移除，单独提示
      notifyInherited()
    } else {
      toast(t('apps.messages.saved'))
    }
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

watch(selectedTemplate, (val) => {
  if (val) {
    hasTemplateConfig.value = true
  }
})
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

.inherited-hint {
  margin: 0 0 1rem;
  padding: 0.5rem 0.75rem;
  border-radius: 0.375rem;
  font-size: 0.8125rem;
  line-height: 1.45;
  color: var(--text-secondary);
  background: color-mix(in srgb, var(--primary-color, #409eff) 10%, transparent);
  border: 1px solid color-mix(in srgb, var(--primary-color, #409eff) 25%, transparent);
}

.scope-hint {
  margin: 0 0 0.75rem;
  font-size: 0.8125rem;
  color: var(--text-secondary);
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
  border: 1px solid rgba(255, 255, 255, 0.4);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.15);
}

@media (prefers-color-scheme: dark) {
  .app-config-dialog :deep(.el-dialog) {
    background: rgba(20, 20, 20, 0.6) !important;
    backdrop-filter: blur(40px) saturate(150%) brightness(0.9);
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
  background-color: rgba(0, 0, 0, 0.25) !important;
}

@media (prefers-color-scheme: dark) {
  .app-config-modal {
    backdrop-filter: blur(12px) saturate(120%) !important;
    background-color: rgba(0, 0, 0, 0.4) !important;
  }
}
</style>
