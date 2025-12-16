<template>
  <el-dialog
    v-model="visible"
    :title="t('templates.online.title')"
    width="90%"
    :close-on-click-modal="false"
    :append-to-body="true"
    :destroy-on-close="true"
    :z-index="2001"
    class="online-template-dialog"
    modal-class="online-template-modal"
  >
    <div class="online-template-content">
      <!-- 分类标签 -->
      <div class="category-tabs">
        <button
          v-for="(_, key) in TEMPLATE_CATEGORIES"
          :key="key"
          :class="['category-tab', { active: selectedCategory === key }]"
          @click="selectedCategory = key as TemplateCategory"
        >
          {{ t(`templates.categories.${key}`) }}
        </button>
      </div>

      <!-- 加载状态 -->
      <div v-if="loading" class="loading-state">
        <el-icon class="is-loading">
          <Loading />
        </el-icon>
        <p>{{ t('templates.online.loading') }}</p>
      </div>

      <!-- 错误状态 -->
      <div v-else-if="error" class="error-state">
        <el-icon>
          <CircleClose />
        </el-icon>
        <p>{{ error }}</p>
        <el-button @click="loadTemplates">{{ t('templates.online.retry') }}</el-button>
      </div>

      <!-- 模板列表 -->
      <div v-else class="template-grid">
        <div
          v-for="template in filteredTemplates"
          :key="template.path"
          class="online-template-card"
        >
          <div class="template-header">
            <h4 class="template-title">{{ template.displayName }}</h4>
            <span class="template-category">{{
              t('templates.categories.' + template.category)
            }}</span>
          </div>

          <div v-if="template.template" class="template-info">
            <p class="info-line">
              <span class="label">{{ t('templates.online.brand') }}:</span>
              <span class="value">{{ template.template.brand }}</span>
            </p>
            <p class="info-line">
              <span class="label">{{ t('templates.online.model') }}:</span>
              <span class="value">{{ template.template.model }}</span>
            </p>
            <p v-if="template.template.marketname" class="info-line">
              <span class="label">{{ t('templates.online.market_name') }}:</span>
              <span class="value">{{ template.template.marketname }}</span>
            </p>
          </div>

          <!-- 可选元数据字段 -->
          <div
            v-if="
              template.template &&
              (template.template.version ||
                template.template.version_code ||
                template.template.author ||
                template.template.description)
            "
            class="template-meta"
          >
            <p v-if="template.template.version || template.template.version_code" class="meta-line">
              <span class="label">{{ t('templates.labels.version') }}:</span>
              <span class="value">
                {{ template.template.version || '' }}
                <span v-if="template.template.version_code" class="version-code"
                  >({{ template.template.version_code }})</span
                >
              </span>
            </p>
            <p v-if="template.template.author" class="meta-line">
              <span class="label">{{ t('templates.labels.author') }}:</span>
              <span class="value">{{ template.template.author }}</span>
            </p>
            <p v-if="template.template.description" class="meta-line meta-description">
              <span class="label">{{ t('templates.labels.description') }}:</span>
              <span class="value">{{ template.template.description }}</span>
            </p>
          </div>

          <div class="template-actions">
            <el-button
              type="primary"
              size="small"
              :loading="importingTemplates.has(template.path)"
              @click="importTemplate(template)"
            >
              {{
                importingTemplates.has(template.path)
                  ? t('templates.online.importing')
                  : t('templates.online.import')
              }}
            </el-button>
          </div>
        </div>

        <div v-if="filteredTemplates.length === 0" class="empty-state">
          <p>{{ t('templates.online.empty_category') }}</p>
        </div>
      </div>
    </div>

    <template #footer>
      <el-button @click="visible = false">{{ t('templates.online.close') }}</el-button>
      <el-button type="primary" @click="loadTemplates">{{
        t('templates.online.refresh')
      }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { ref, computed, watch } from 'vue'
import { Loading, CircleClose } from '@element-plus/icons-vue'
import { toast } from 'kernelsu-alt'
import {
  fetchOnlineTemplates,
  downloadTemplate,
  TEMPLATE_CATEGORIES,
  type OnlineTemplate,
  type TemplateCategory,
} from '../utils/onlineTemplates'
import { useConfigStore } from '../stores/config'
import { useI18n } from '../utils/i18n'
import { useLazyMessageBox } from '../utils/elementPlus'

const props = defineProps<{
  modelValue: boolean
}>()

const emit = defineEmits<{
  (e: 'update:modelValue', value: boolean): void
}>()

const configStore = useConfigStore()
const { t } = useI18n()
const getMessageBox = useLazyMessageBox()

const visible = computed({
  get: () => props.modelValue,
  set: (value) => emit('update:modelValue', value),
})

const loading = ref(false)
const error = ref<string | null>(null)
const templates = ref<OnlineTemplate[]>([])
const selectedCategory = ref<TemplateCategory>('common')
const importingTemplates = ref(new Set<string>())

// 过滤当前分类的模板
const filteredTemplates = computed(() => {
  return templates.value.filter((t) => t.category === selectedCategory.value)
})

// 加载在线模板列表
async function loadTemplates() {
  loading.value = true
  error.value = null

  toast(t('templates.online.toasts.start_loading'))

  try {
    toast(t('templates.online.toasts.fetching_list'))
    const onlineTemplates = await fetchOnlineTemplates()

    toast(t('templates.online.toasts.got_templates', { count: onlineTemplates.length }))

    if (onlineTemplates.length === 0) {
      error.value = t('templates.online.errors.no_templates')
      toast(t('templates.online.toasts.no_templates_toast'))
      return
    }

    // 先显示模板列表（不等待下载）
    templates.value = onlineTemplates
    toast(t('templates.online.toasts.list_loaded'))

    // 后台异步下载模板内容
    let successCount = 0
    let failCount = 0

    Promise.all(
      onlineTemplates.map(async (t, index) => {
        try {
          const template = await downloadTemplate(t)
          if (template) {
            templates.value[index] = { ...t, template }
            successCount++
          } else {
            failCount++
          }
        } catch {
          failCount++
        }
      })
    ).then(() => {
      if (successCount > 0) {
        toast(t('templates.online.toasts.content_loaded', { count: successCount }))
      }
      if (failCount > 0) {
        toast(t('templates.online.toasts.content_failed', { count: failCount }))
      }
    })
  } catch (e) {
    error.value = e instanceof Error ? e.message : t('templates.online.errors.load_failed')
    toast(t('templates.online.toasts.load_failed', { error: error.value }))
  } finally {
    loading.value = false
  }
}

// 导入模板
async function importTemplate(onlineTemplate: OnlineTemplate) {
  if (!onlineTemplate.template) {
    toast(t('templates.online.errors.empty_content'))
    return
  }

  importingTemplates.value.add(onlineTemplate.path)

  try {
    // 使用文件名作为模板名称
    const templateName = onlineTemplate.name

    // 检查是否已存在
    const existingTemplates = configStore.getTemplates()
    if (existingTemplates[templateName]) {
      const messageBox = await getMessageBox()
      await messageBox.confirm(
        t('templates.online.messages.exists_confirm', { name: templateName }),
        t('templates.online.messages.exists_title'),
        {
          confirmButtonText: t('templates.online.messages.overwrite'),
          cancelButtonText: t('common.cancel'),
          type: 'warning',
        }
      )
    }

    // 保存模板
    configStore.setTemplate(templateName, onlineTemplate.template)
    await configStore.saveConfig()

    toast(t('templates.online.messages.import_success', { name: templateName }))
  } catch (e) {
    if (e !== 'cancel') {
      toast(t('templates.online.errors.import_failed'))
    }
  } finally {
    importingTemplates.value.delete(onlineTemplate.path)
  }
}

// 监听对话框打开，自动加载模板
watch(
  visible,
  (newValue) => {
    if (newValue && templates.value.length === 0) {
      loadTemplates().catch((err) => {
        console.error('loadTemplates error:', err)
        toast(`加载模板时发生错误: ${err.message || err}`)
      })
    }
  },
  { immediate: true }
)
</script>

<style scoped>
.online-template-content {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  min-height: 400px;
}

.category-tabs {
  display: flex;
  gap: 0.5rem;
  padding-bottom: 1rem;
  border-bottom: 1px solid var(--border);
  flex-wrap: nowrap;
}

.category-tab {
  padding: 0.5rem 1rem;
  background: var(--background);
  border: 1px solid var(--border);
  border-radius: 0.5rem;
  color: var(--text);
  font-size: 0.875rem;
  transition: all 0.2s ease;
  cursor: pointer;
  white-space: nowrap;
}

.category-tab:hover {
  background: var(--hover);
}

.category-tab.active {
  background: var(--primary);
  color: white;
  border-color: var(--primary);
}

.loading-state,
.error-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  gap: 1rem;
  padding: 3rem 1rem;
  color: var(--text-secondary);
}

.loading-state .el-icon {
  font-size: 2rem;
}

.error-state .el-icon {
  font-size: 3rem;
  color: #ef4444;
}

.template-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(280px, 1fr));
  gap: 1rem;
  max-height: 500px;
  overflow-y: auto;
  padding: 0.5rem;
  /* 移除滚动条，防止影响主页面布局 */
  scrollbar-width: none; /* Firefox */
  -ms-overflow-style: none; /* IE/Edge */
  contain: layout style paint;
}

/* 隐藏滚动条 */
.template-grid::-webkit-scrollbar {
  display: none; /* Chrome/Safari/Opera */
}

.online-template-card {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  padding: 1rem;
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: 0.75rem;
  transition: all 0.2s ease;
}

.online-template-card:hover {
  border-color: var(--primary);
  box-shadow: 0 4px 12px var(--shadow);
}

.template-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  gap: 0.5rem;
}

.template-title {
  font-size: 1rem;
  font-weight: 600;
  color: var(--text);
  margin: 0;
  flex: 1;
}

.template-category {
  padding: 0.25rem 0.5rem;
  background: var(--primary);
  color: white;
  font-size: 0.75rem;
  border-radius: 0.25rem;
  white-space: nowrap;
}

.template-info {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.info-line {
  display: flex;
  gap: 0.5rem;
  font-size: 0.875rem;
  margin: 0;
}

.info-line .label {
  color: var(--text-secondary);
  min-width: 60px;
}

.info-line .value {
  color: var(--text);
  flex: 1;
  word-break: break-all;
}

.template-meta {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  padding-top: 0.5rem;
  border-top: 1px dashed var(--border);
  margin-top: 0.25rem;
}

.meta-line {
  display: flex;
  gap: 0.5rem;
  font-size: 0.8125rem;
  margin: 0;
}

.meta-line .label {
  color: var(--text-secondary);
  min-width: 50px;
  flex-shrink: 0;
}

.meta-line .value {
  color: var(--text);
  flex: 1;
  word-break: break-all;
}

.meta-line .version-code {
  color: var(--text-secondary);
  font-size: 0.75rem;
}

.meta-description .value {
  font-size: 0.75rem;
  color: var(--text-secondary);
  line-height: 1.4;
}

.meta-description .label {
  font-size: 0.75rem;
  line-height: 1.4;
}

.template-actions {
  display: flex;
  justify-content: flex-end;
  padding-top: 0.5rem;
  border-top: 1px solid var(--border);
}

.empty-state {
  grid-column: 1 / -1;
  text-align: center;
  padding: 3rem 1rem;
  color: var(--text-secondary);
}

/* Dialog 样式 */
.online-template-dialog :deep(.el-dialog) {
  background: rgba(255, 255, 255, 0.15) !important;
  backdrop-filter: blur(40px) saturate(150%) brightness(1.1);
  -webkit-backdrop-filter: blur(40px) saturate(150%) brightness(1.1);
  border: 1px solid rgba(255, 255, 255, 0.4);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.15);
}

/* 确保对话框内容区域不显示滚动条 */
.online-template-dialog :deep(.el-dialog__body) {
  scrollbar-width: none;
  -ms-overflow-style: none;
  overflow-y: auto;
}

.online-template-dialog :deep(.el-dialog__body::-webkit-scrollbar) {
  display: none;
}

@media (prefers-color-scheme: dark) {
  .online-template-dialog :deep(.el-dialog) {
    background: rgba(20, 20, 20, 0.6) !important;
    backdrop-filter: blur(40px) saturate(150%) brightness(0.9);
    -webkit-backdrop-filter: blur(40px) saturate(150%) brightness(0.9);
    border: 1px solid rgba(255, 255, 255, 0.15);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }
}
</style>

<style>
.online-template-modal {
  backdrop-filter: blur(12px) saturate(120%) !important;
  -webkit-backdrop-filter: blur(12px) saturate(120%) !important;
  background-color: rgba(0, 0, 0, 0.25) !important;
}

@media (prefers-color-scheme: dark) {
  .online-template-modal {
    backdrop-filter: blur(12px) saturate(120%) !important;
    -webkit-backdrop-filter: blur(12px) saturate(120%) !important;
    background-color: rgba(0, 0, 0, 0.4) !important;
  }
}
</style>
