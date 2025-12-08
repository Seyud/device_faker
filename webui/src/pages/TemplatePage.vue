<template>
  <div class="template-page">
    <div class="page-header">
      <h2 class="page-title">{{ t('templates.title') }}</h2>
      <div class="header-actions" :class="{ 'vertical-layout': locale === 'en' }">
        <button class="add-btn secondary" @click="showOnlineDialog">
          <Download :size="20" />
          {{ t('templates.actions.online') }}
        </button>
        <button class="add-btn" @click="showAddDialog">
          <Plus :size="20" />
          {{ t('templates.actions.new') }}
        </button>
      </div>
    </div>

    <div class="template-list">
      <div v-for="(template, name) in templates" :key="name" class="template-card glass-effect">
        <div class="template-header">
          <div class="template-info">
            <h3 class="template-name">{{ name }}</h3>
            <p class="template-device">{{ template.brand }} {{ template.model }}</p>
          </div>
          <div class="template-actions">
            <button class="icon-btn" @click="editTemplate(name, template)">
              <Edit2 :size="18" />
            </button>
            <button class="icon-btn danger" @click="deleteTemplateConfirm(name)">
              <Trash2 :size="18" />
            </button>
          </div>
        </div>

        <div class="template-details">
          <div class="detail-item">
            <span class="detail-label">{{ t('templates.fields.manufacturer') }}:</span>
            <span class="detail-value">{{ template.manufacturer }}</span>
          </div>
          <div class="detail-item">
            <span class="detail-label">{{ t('templates.fields.device') }}:</span>
            <span class="detail-value">{{ template.device }}</span>
          </div>
          <div class="detail-item">
            <span class="detail-label">{{ t('templates.fields.fingerprint') }}:</span>
            <span class="detail-value fingerprint">{{ template.fingerprint }}</span>
          </div>
          <div v-if="template.mode" class="detail-item">
            <span class="detail-label">{{ t('templates.labels.mode') }}:</span>
            <span class="detail-value">{{
              template.mode === 'lite' ? t('templates.values.lite') : t('templates.values.full')
            }}</span>
          </div>
          <div v-if="template.packages && template.packages.length > 0" class="detail-item">
            <span class="detail-label">{{ t('templates.labels.packages') }}:</span>
            <span class="detail-value"
              >{{ template.packages.length }} {{ t('templates.labels.count_suffix') }}</span
            >
          </div>
        </div>
      </div>

      <div v-if="Object.keys(templates).length === 0" class="empty-state">
        <FileText :size="64" class="empty-icon" />
        <p class="empty-text">{{ t('templates.empty.title') }}</p>
        <p class="empty-hint">{{ t('templates.empty.hint') }}</p>
      </div>
    </div>

    <!-- 编辑对话框 -->
    <el-dialog
      v-model="dialogVisible"
      :title="isEditing ? t('templates.dialog.edit_title') : t('templates.dialog.new_title')"
      width="90%"
      :close-on-click-modal="false"
      :append-to-body="true"
      :destroy-on-close="true"
      :z-index="2001"
      class="template-dialog"
      modal-class="template-dialog-modal"
    >
      <el-form :model="formData" label-width="120px" label-position="top">
        <el-form-item :label="t('templates.fields.name')">
          <el-input
            v-model="formData.name"
            :disabled="isEditing"
            :placeholder="t('templates.placeholders.name')"
          />
        </el-form-item>

        <el-form-item :label="t('templates.fields.manufacturer')">
          <el-input v-model="formData.manufacturer" placeholder="例如：ZTE" />
        </el-form-item>

        <el-form-item :label="t('templates.fields.brand')">
          <el-input v-model="formData.brand" placeholder="例如：nubia" />
        </el-form-item>

        <el-form-item :label="t('templates.fields.model')">
          <el-input v-model="formData.model" placeholder="例如：25010PN30C，NX769J" />
        </el-form-item>

        <el-form-item :label="t('templates.fields.device')">
          <el-input v-model="formData.device" placeholder="例如：xuanyuan，NX769J" />
        </el-form-item>

        <el-form-item :label="t('templates.fields.product')">
          <el-input v-model="formData.product" placeholder="例如：xuanyuan，NX769J" />
        </el-form-item>

        <el-form-item :label="t('templates.fields.name_field')">
          <el-input v-model="formData.name_field" placeholder="例如：xuanyuan" />
        </el-form-item>

        <el-form-item :label="t('templates.fields.market_name')">
          <el-input v-model="formData.marketname" placeholder="例如：REDMAGIC 9 Pro" />
        </el-form-item>

        <el-form-item :label="t('templates.fields.fingerprint')">
          <el-input
            v-model="formData.fingerprint"
            type="textarea"
            :rows="3"
            placeholder="例如：nubia/NX769J/NX769J:14/UKQ1.230917.001/20240813.173312:user/release-keys"
          />
        </el-form-item>

        <el-form-item :label="t('templates.fields.mode')">
          <el-select
            v-model="formData.mode"
            :placeholder="t('templates.placeholders.mode')"
            clearable
          >
            <el-option :label="t('templates.options.mode_lite')" value="lite" />
            <el-option :label="t('templates.options.mode_full')" value="full" />
          </el-select>
        </el-form-item>

        <el-form-item :label="t('templates.fields.packages')">
          <div class="package-manager">
            <div class="package-input-wrapper">
              <el-autocomplete
                v-model="packageInput"
                :fetch-suggestions="searchPackages"
                :placeholder="t('templates.placeholders.packages')"
                style="width: 100%"
                clearable
                @keyup.enter="addPackage"
              >
                <template #default="{ item }">
                  <div class="package-suggestion">
                    <div class="app-info">
                      <div class="app-name">{{ item.appName }}</div>
                      <div class="package-name">{{ item.packageName }}</div>
                    </div>
                  </div>
                </template>
              </el-autocomplete>
              <el-button type="primary" :disabled="!packageInput" @click="addPackage">{{
                t('templates.actions.add')
              }}</el-button>
              >
            </div>

            <div v-if="formData.packages.length > 0" class="package-list">
              <div v-for="(pkg, index) in formData.packages" :key="index" class="package-item">
                <div class="package-info">
                  <span class="package-name-text">{{ pkg }}</span>
                  <span v-if="getAppName(pkg)" class="app-name-text">{{ getAppName(pkg) }}</span>
                </div>
                <el-button type="danger" size="small" circle @click="removePackage(index)">
                  ×
                </el-button>
              </div>
            </div>
            <div v-else class="package-list-empty">{{ t('templates.empty.packages') }}</div>
          </div>
        </el-form-item>
      </el-form>

      <template #footer>
        <el-button @click="dialogVisible = false">{{ t('common.cancel') }}</el-button>
        <el-button type="primary" @click="saveTemplate">{{ t('common.save') }}</el-button>
      </template>
    </el-dialog>

    <!-- 在线模板对话框 -->
    <OnlineTemplateDialog v-model="onlineDialogVisible" />
  </div>
</template>

<script setup lang="ts">
import { ref, computed, onMounted, onActivated, nextTick } from 'vue'
import { Plus, Edit2, Trash2, FileText, Download } from 'lucide-vue-next'
import { ElMessage, ElMessageBox } from 'element-plus'
import { useConfigStore } from '../stores/config'
import { useAppsStore } from '../stores/apps'
import OnlineTemplateDialog from '../components/OnlineTemplateDialog.vue'
import { useI18n } from '../utils/i18n'
import type { Template } from '../types'

const configStore = useConfigStore()
const appsStore = useAppsStore()
const { t, locale } = useI18n()

const templates = computed(() => configStore.getTemplates())
const installedApps = computed(() => appsStore.installedApps)

const dialogVisible = ref(false)
const onlineDialogVisible = ref(false)
const isEditing = ref(false)
const packageInput = ref('')
const formData = ref({
  name: '',
  manufacturer: '',
  brand: '',
  model: '',
  device: '',
  product: '',
  name_field: '',
  marketname: '',
  fingerprint: '',
  mode: '',
  packages: [] as string[],
})

function showOnlineDialog() {
  onlineDialogVisible.value = true
}

function showAddDialog() {
  isEditing.value = false
  packageInput.value = ''
  formData.value = {
    name: '',
    manufacturer: '',
    brand: '',
    model: '',
    device: '',
    product: '',
    name_field: '',
    marketname: '',
    fingerprint: '',
    mode: '',
    packages: [],
  }
  dialogVisible.value = true
}

function editTemplate(name: string, template: Template) {
  isEditing.value = true
  packageInput.value = ''
  formData.value = {
    name,
    manufacturer: template.manufacturer || '',
    brand: template.brand || '',
    model: template.model || '',
    device: template.device || '',
    product: template.product || '',
    name_field: template.name || '',
    marketname: template.marketname || '',
    fingerprint: template.fingerprint || '',
    mode: template.mode || '',
    packages: template.packages || [],
  }
  dialogVisible.value = true
}

// 搜索包名建议
function searchPackages(
  queryString: string,
  cb: (suggestions: Array<{ value: string; appName: string; packageName: string }>) => void
) {
  const suggestions = installedApps.value
    .filter((app) => {
      const query = queryString.toLowerCase()
      return (
        app.packageName.toLowerCase().includes(query) || app.appName.toLowerCase().includes(query)
      )
    })
    .map((app) => ({
      value: app.packageName,
      appName: app.appName,
      packageName: app.packageName,
    }))
  cb(suggestions)
}

// 添加包名
function addPackage() {
  const pkgName = packageInput.value.trim()
  if (!pkgName) {
    return
  }

  if (formData.value.packages.includes(pkgName)) {
    ElMessage.warning(t('templates.messages.pkg_exists'))
    return
  }

  formData.value.packages.push(pkgName)
  packageInput.value = ''
}

// 删除包名
function removePackage(index: number) {
  formData.value.packages.splice(index, 1)
}

// 获取应用名称
function getAppName(packageName: string): string {
  const app = installedApps.value.find((a) => a.packageName === packageName)
  return app ? app.appName : ''
}

async function saveTemplate() {
  if (!formData.value.name) {
    ElMessage.error(t('templates.messages.name_required'))
    return
  }

  const template: Template = {
    manufacturer: formData.value.manufacturer,
    brand: formData.value.brand,
    model: formData.value.model,
    device: formData.value.device,
    product: formData.value.product,
    fingerprint: formData.value.fingerprint,
  }

  if (formData.value.name_field) {
    template.name = formData.value.name_field
  }

  if (formData.value.marketname) {
    template.marketname = formData.value.marketname
  }

  if (formData.value.mode) {
    template.mode = formData.value.mode as 'lite' | 'full'
  }

  if (formData.value.packages.length > 0) {
    template.packages = formData.value.packages
  }

  configStore.setTemplate(formData.value.name, template)

  try {
    await configStore.saveConfig()
    ElMessage.success(t('templates.messages.saved'))
    dialogVisible.value = false
  } catch {
    ElMessage.error(t('common.failed'))
  }
}

async function deleteTemplateConfirm(name: string) {
  try {
    await ElMessageBox.confirm(
      t('templates.dialog.delete_confirm', { name }),
      t('templates.dialog.delete_title'),
      {
        confirmButtonText: t('common.delete'),
        cancelButtonText: t('common.cancel'),
        type: 'warning',
        appendTo: 'body',
        customClass: 'delete-confirm-box',
        modalClass: 'delete-confirm-modal',
      }
    )

    configStore.deleteTemplate(name)
    await configStore.saveConfig()
    ElMessage.success(t('templates.messages.deleted'))
  } catch {
    // 用户取消
  }
}

onMounted(() => {
  // 初始加载
  if (appsStore.installedApps.length === 0) {
    appsStore.loadInstalledApps()
  }
})

onActivated(() => {
  // KeepAlive 激活时，确保布局正确
  nextTick(() => {
    // 更彻底的布局重置
    const templatePage = document.querySelector('.template-page') as HTMLElement | null
    const templateList = document.querySelector('.template-list') as HTMLElement | null
    const templateCards = document.querySelectorAll('.template-card')
    const glassElements = document.querySelectorAll('.glass-effect')

    // 强制重新计算所有元素样式
    const allElements = document.querySelectorAll('*')
    allElements.forEach((element) => {
      // 强制浏览器重新计算样式
      void window.getComputedStyle(element).width
      void window.getComputedStyle(element).height
    })

    // 触发全局重排
    if (templatePage) {
      void templatePage.offsetHeight
    }

    // 触发模板列表重排
    if (templateList) {
      void templateList.offsetHeight
    }

    // 触发每个模板卡片重排
    templateCards.forEach((card) => {
      const htmlCard = card as HTMLElement
      // 重新应用样式
      htmlCard.style.display = 'none'
      void htmlCard.offsetHeight
      htmlCard.style.display = ''
      void htmlCard.offsetHeight
    })

    // 确保玻璃效果重新渲染
    glassElements.forEach((element) => {
      const htmlElement = element as HTMLElement
      // 重新应用背景和模糊效果
      const currentBackground = window.getComputedStyle(htmlElement).background
      htmlElement.style.background = 'transparent'
      void htmlElement.offsetHeight
      htmlElement.style.background = currentBackground
      void htmlElement.offsetHeight
    })

    // 触发窗口 resize 事件，确保全局布局更新
    window.dispatchEvent(new Event('resize'))

    // 触发滚动事件，确保滚动位置正确
    window.dispatchEvent(new Event('scroll'))

    // 触发滚动条宽度补偿更新
    const mainContent = document.querySelector('.main-content') as HTMLElement
    if (mainContent) {
      // 触发滚动条宽度计算和补偿
      const event = new Event('resize')
      window.dispatchEvent(event)
    }
  })
})
</script>

<style scoped>
.template-page {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  width: 100%;
  max-width: 100%;
  box-sizing: border-box;
  /* 确保宽度稳定，不受滚动条影响 */
  overflow: hidden;
}

.page-header {
  display: flex;
  justify-content: space-between;
  align-items: center;
  margin-bottom: 0.5rem;
  flex-wrap: wrap;
  gap: 0.5rem;
}

.page-title {
  font-size: 1.5rem;
  font-weight: 600;
  color: var(--text);
  line-height: 1.3;
}

.header-actions {
  display: flex;
  gap: 0.5rem;
  flex-shrink: 0;
}

.header-actions.vertical-layout {
  flex-direction: column;
  align-items: flex-start;
}

.add-btn {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.75rem 1rem;
  background: var(--primary);
  color: white;
  border: none;
  border-radius: 0.5rem;
  font-size: 0.875rem;
  font-weight: 500;
  transition: all 0.2s ease;
  -webkit-tap-highlight-color: transparent;
  user-select: none;
  -webkit-user-select: none;
  white-space: nowrap;
}

.add-btn.secondary {
  background: var(--card);
  color: var(--text);
  border: 1px solid var(--border);
}

.add-btn:active {
  opacity: 0.8;
  transform: scale(0.98);
}

.template-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  width: 100%;
  max-width: 100%;
}

.template-card {
  padding: 1.25rem;
  border-radius: 1rem;
  box-shadow: 0 2px 8px var(--shadow);
  width: 100%;
  min-width: 0;
  box-sizing: border-box;
  /* 增强布局稳定性 */
  position: relative;
  /* 触发硬件加速，增强渲染稳定性 */
  transform: translateZ(0);
  /* 防止布局抖动 */
  contain: layout style paint;
  /* 优化阴影实现 */
  box-shadow:
    0 2px 8px var(--shadow),
    0 0 0 0 transparent;
}

.template-header {
  display: flex;
  justify-content: space-between;
  align-items: flex-start;
  margin-bottom: 1rem;
  width: 100%;
  box-sizing: border-box;
}

.template-info {
  flex: 1;
  min-width: 0;
  overflow: hidden;
}

.template-name {
  font-size: 1.125rem;
  font-weight: 600;
  color: var(--text);
  margin-bottom: 0.25rem;
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  /* 增强宽度约束 */
  width: 100%;
  max-width: calc(100% - 80px); /* 减去操作按钮宽度 */
}

.template-device {
  font-size: 0.875rem;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  /* 增强宽度约束 */
  width: 100%;
  max-width: calc(100% - 80px); /* 减去操作按钮宽度 */
}

.template-actions {
  display: flex;
  gap: 0.5rem;
  flex-shrink: 0;
  min-width: fit-content;
}

.icon-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  background: var(--background);
  border: none;
  border-radius: 0.5rem;
  color: var(--text);
  transition: all 0.2s ease;
  -webkit-tap-highlight-color: transparent;
  user-select: none;
  -webkit-user-select: none;
}

.icon-btn:active {
  background: var(--primary);
  color: white;
  transform: scale(0.95);
}

.icon-btn.danger:active {
  background: #ef4444;
}

.template-details {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
}

.detail-item {
  display: flex;
  gap: 0.5rem;
  font-size: 0.875rem;
}

.detail-label {
  color: var(--text-secondary);
  min-width: 100px;
}

.detail-value {
  color: var(--text);
  flex: 1;
  word-break: break-all;
}

.detail-value.fingerprint {
  font-family: monospace;
  font-size: 0.75rem;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 3rem 1rem;
  text-align: center;
}

.empty-icon {
  color: var(--text-secondary);
  opacity: 0.3;
  margin-bottom: 1rem;
}

.empty-text {
  font-size: 1.125rem;
  font-weight: 500;
  color: var(--text);
  margin-bottom: 0.5rem;
}

.empty-hint {
  font-size: 0.875rem;
  color: var(--text-secondary);
}

/* 包名管理器样式 */
.package-manager {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.package-input-wrapper {
  display: flex;
  gap: 0.5rem;
  align-items: flex-start;
}

.package-suggestion {
  padding: 0.25rem 0;
}

.app-info {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
}

.app-name {
  font-size: 0.875rem;
  color: var(--text);
}

.package-name {
  font-size: 0.75rem;
  color: var(--text-secondary);
  font-family: monospace;
}

.package-list {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  padding: 0.75rem;
  background: var(--background);
  border-radius: 0.5rem;
  max-height: 300px;
  overflow-y: auto;
  /* 隐藏滚动条 */
  scrollbar-width: none;
  -ms-overflow-style: none;
}

.package-list::-webkit-scrollbar {
  display: none;
}

.package-list-empty {
  padding: 1.5rem;
  text-align: center;
  color: var(--text-secondary);
  font-size: 0.875rem;
  background: var(--background);
  border-radius: 0.5rem;
}

.package-item {
  display: flex;
  align-items: center;
  justify-content: space-between;
  padding: 0.625rem 0.75rem;
  background: var(--card);
  border-radius: 0.375rem;
  transition: all 0.2s ease;
}

.package-item:hover {
  background: var(--hover);
}

.package-info {
  display: flex;
  flex-direction: column;
  gap: 0.125rem;
  flex: 1;
  min-width: 0;
}

.package-name-text {
  font-size: 0.875rem;
  font-family: monospace;
  color: var(--text);
  word-break: break-all;
}

.app-name-text {
  font-size: 0.75rem;
  color: var(--text-secondary);
}

/* Dialog 样式优化 */
.template-dialog :deep(.el-dialog) {
  margin-top: 5vh !important;
  margin-bottom: 80px !important; /* 为底栏留出足够空间 */
  max-height: calc(100vh - 80px - 10vh) !important; /* 减去底栏高度和顶部边距 */
  display: flex;
  flex-direction: column;
  /* 增强的毛玻璃效果 */
  background: rgba(255, 255, 255, 0.15) !important;
  backdrop-filter: blur(40px) saturate(150%) brightness(1.1);
  -webkit-backdrop-filter: blur(40px) saturate(150%) brightness(1.1);
  border: 1px solid rgba(255, 255, 255, 0.4);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.15);
  /* 确保对话框布局稳定 */
  box-sizing: border-box;
  width: 90%;
  /* 触发硬件加速 */
  transform: translateZ(0);
  /* 防止对话框影响页面布局 */
  contain: layout style paint;
}

/* 深色模式下的毛玻璃效果 */
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
  padding-bottom: 2rem; /* 增加底部内边距 */
  max-height: calc(100vh - 200px); /* 确保有足够的滚动空间 */
  background: transparent;
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
  flex-shrink: 0; /* 防止底部按钮区域被压缩 */
}

@media (prefers-color-scheme: dark) {
  .template-dialog :deep(.el-dialog__footer) {
    border-top: 1px solid rgba(255, 255, 255, 0.1);
    background: rgba(0, 0, 0, 0.3);
  }
}

/* 确保 Dialog 遮罩层在底栏之上 */
.template-dialog :deep(.el-overlay) {
  z-index: 2000 !important;
  /* 降低背景模糊效果，减少对页面布局的影响 */
  backdrop-filter: blur(8px) saturate(110%) !important;
  -webkit-backdrop-filter: blur(8px) saturate(110%) !important;
  background-color: rgba(0, 0, 0, 0.25) !important;
  /* 确保遮罩层不会影响页面布局 */
  contain: layout style paint;
  /* 防止遮罩层滚动条影响主页面 */
  overflow: hidden;
}

@media (prefers-color-scheme: dark) {
  .template-dialog :deep(.el-overlay) {
    backdrop-filter: blur(8px) saturate(110%) !important;
    -webkit-backdrop-filter: blur(8px) saturate(110%) !important;
    background-color: rgba(0, 0, 0, 0.4) !important;
  }
}

/* 对话框内容区域滚动优化 - 确保不显示滚动条 */
.template-dialog :deep(.el-dialog__body) {
  scrollbar-width: none; /* Firefox */
  -ms-overflow-style: none; /* IE/Edge */
  contain: layout style paint;
  overflow-y: auto;
  overflow-x: hidden;
}

.template-dialog :deep(.el-dialog__body::-webkit-scrollbar) {
  display: none; /* Chrome/Safari/Opera */
}
</style>

<style>
/* 全局样式：Dialog 遮罩层毛玻璃效果 */
.template-dialog-modal {
  backdrop-filter: blur(12px) saturate(120%) !important;
  -webkit-backdrop-filter: blur(12px) saturate(120%) !important;
  background-color: rgba(0, 0, 0, 0.25) !important;
}

@media (prefers-color-scheme: dark) {
  .template-dialog-modal {
    backdrop-filter: blur(12px) saturate(120%) !important;
    -webkit-backdrop-filter: blur(12px) saturate(120%) !important;
    background-color: rgba(0, 0, 0, 0.4) !important;
  }
}

/* 删除确认框的毛玻璃效果 */
.delete-confirm-modal {
  backdrop-filter: blur(12px) saturate(120%) !important;
  -webkit-backdrop-filter: blur(12px) saturate(120%) !important;
  background-color: rgba(0, 0, 0, 0.25) !important;
}

@media (prefers-color-scheme: dark) {
  .delete-confirm-modal {
    backdrop-filter: blur(12px) saturate(120%) !important;
    -webkit-backdrop-filter: blur(12px) saturate(120%) !important;
    background-color: rgba(0, 0, 0, 0.4) !important;
  }
}

.delete-confirm-box {
  background: rgba(255, 255, 255, 0.15) !important;
  backdrop-filter: blur(40px) saturate(150%) brightness(1.1) !important;
  -webkit-backdrop-filter: blur(40px) saturate(150%) brightness(1.1) !important;
  border: 1px solid rgba(255, 255, 255, 0.4) !important;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.15) !important;
}

@media (prefers-color-scheme: dark) {
  .delete-confirm-box {
    background: rgba(20, 20, 20, 0.6) !important;
    backdrop-filter: blur(40px) saturate(150%) brightness(0.9) !important;
    -webkit-backdrop-filter: blur(40px) saturate(150%) brightness(0.9) !important;
    border: 1px solid rgba(255, 255, 255, 0.15) !important;
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5) !important;
  }
}
</style>
