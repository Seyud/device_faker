<template>
  <el-dialog
    v-model="visible"
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
        <el-input v-model="templateNameInput" :placeholder="t('templates.placeholders.name')" />
      </el-form-item>

      <DeviceFakerFormFields>
        <template #packages>
          <el-form-item :label="t('templates.fields.packages')">
            <div class="package-manager">
              <div :class="['package-input-wrapper', { 'stacked-layout': locale === 'en' }]">
                <el-autocomplete
                  v-model="packageInput"
                  :fetch-suggestions="searchPackages"
                  :placeholder="t('templates.placeholders.packages')"
                  clearable
                  class="package-autocomplete"
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
                <el-button type="primary" :disabled="!packageInput" @click="addPackage">
                  {{ t('templates.actions.add') }}
                </el-button>
              </div>

              <div v-if="formData.packages.length > 0" class="package-list">
                <div v-for="(pkg, index) in formData.packages" :key="pkg" class="package-item">
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
        </template>
      </DeviceFakerFormFields>
    </el-form>

    <template #footer>
      <el-button @click="visible = false">{{ t('common.cancel') }}</el-button>
      <el-button type="primary" @click="saveTemplate">{{ t('common.save') }}</el-button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed, ref, watch } from 'vue'
import { useConfigStore } from '../../stores/config'
import { useAppsStore } from '../../stores/apps'
import { useI18n } from '../../utils/i18n'
import { useLazyMessageBox } from '../../utils/elementPlus'
import { toast } from 'kernelsu-alt'
import type { Template } from '../../types'
import { provideDeviceFakerForm } from '../../composables/useDeviceFakerForm'
import DeviceFakerFormFields from '../shared/DeviceFakerFormFields.vue'

const props = defineProps<{
  modelValue: boolean
  isEditing: boolean
  locale: string
  templateName?: string | null
  templateData?: Template | null
}>()

const emit = defineEmits<{ 'update:modelValue': [boolean]; saved: [string] }>()

const { t } = useI18n()
const configStore = useConfigStore()
const appsStore = useAppsStore()
const getMessageBox = useLazyMessageBox()

const installedApps = computed(() => appsStore.installedApps)
const visible = computed({
  get: () => props.modelValue,
  set: (val: boolean) => emit('update:modelValue', val),
})

const packageInput = ref('')
const templateNameInput = ref('')

const { formData, resetForm, fillFromTemplate, toTemplate } = provideDeviceFakerForm()

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

async function addPackage() {
  const pkgName = packageInput.value.trim()
  if (!pkgName) return

  if (formData.value.packages.includes(pkgName)) {
    toast(t('templates.messages.pkg_exists'))
    return
  }

  formData.value.packages.push(pkgName)
  packageInput.value = ''
}

function removePackage(index: number) {
  formData.value.packages.splice(index, 1)
}

function getAppName(packageName: string): string {
  const app = installedApps.value.find((a) => a.packageName === packageName)
  return app ? app.appName : ''
}

async function confirmRenameOverwrite(oldName: string, newName: string) {
  const messageBox = await getMessageBox()
  await messageBox.confirm(
    t('templates.dialog.rename_overwrite_confirm', { oldName, newName }),
    t('templates.dialog.rename_overwrite_title'),
    {
      confirmButtonText: t('common.confirm'),
      cancelButtonText: t('common.cancel'),
      type: 'warning',
      appendTo: 'body',
    }
  )
}

async function saveTemplate() {
  const name = templateNameInput.value.trim()

  if (!name) {
    toast(t('templates.messages.name_required'))
    return
  }

  templateNameInput.value = name

  const originalTemplateName = props.templateName?.trim() || ''
  const isRenaming =
    props.isEditing && originalTemplateName.length > 0 && originalTemplateName !== name
  let overwriteRenameTarget = false

  const template = toTemplate(props.templateData || undefined)

  try {
    if (isRenaming && Object.prototype.hasOwnProperty.call(configStore.getTemplates(), name)) {
      await confirmRenameOverwrite(originalTemplateName, name)
      overwriteRenameTarget = true
    }

    if (isRenaming) {
      configStore.renameTemplate(originalTemplateName, name, template, {
        overwrite: overwriteRenameTarget,
      })
    } else {
      configStore.setTemplate(name, template, { replace: true })
    }

    await configStore.saveConfig()
    toast(t('templates.messages.saved'))
    emit('saved', name)
    visible.value = false
  } catch (e) {
    if (e === 'cancel') {
      return
    }

    console.error('Save template failed:', e)
    const errorMessage = e instanceof Error ? e.message : String(e)
    toast(`${t('common.failed')}: ${errorMessage}`)
  }
}

watch(
  () => [props.modelValue, props.isEditing, props.templateName, props.templateData],
  ([dialogVisible, editing]) => {
    if (!dialogVisible) return

    packageInput.value = ''
    if (editing) {
      fillFromTemplate(props.templateData!)
      templateNameInput.value = props.templateName || ''
    } else {
      resetForm()
      templateNameInput.value = ''
    }
  },
  { immediate: true }
)
</script>

<style scoped>
.package-manager {
  display: flex;
  flex-direction: column;
  gap: 1rem;
}

.package-input-wrapper {
  display: flex;
  gap: 0.5rem;
  align-items: center;
}

.package-input-wrapper.stacked-layout {
  flex-direction: column;
}

.package-autocomplete {
  width: 100%;
}

.package-input-wrapper :deep(.el-input__wrapper) {
  height: 32px;
}

.package-input-wrapper .el-button {
  height: 32px;
  padding: 0 16px;
  display: flex;
  align-items: center;
  justify-content: center;
}

.package-input-wrapper.stacked-layout .el-button {
  width: 100%;
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
</style>

<style scoped>
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
  box-sizing: border-box;
  width: 90%;
  transform: translateZ(0);
  contain: layout style paint;
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
  padding-bottom: 2rem;
  max-height: calc(100vh - 200px);
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
  backdrop-filter: blur(8px) saturate(110%) !important;
  -webkit-backdrop-filter: blur(8px) saturate(110%) !important;
  background-color: rgba(0, 0, 0, 0.25) !important;
  contain: layout style paint;
  overflow: hidden;
}

@media (prefers-color-scheme: dark) {
  .template-dialog :deep(.el-overlay) {
    backdrop-filter: blur(8px) saturate(110%) !important;
    -webkit-backdrop-filter: blur(8px) saturate(110%) !important;
    background-color: rgba(0, 0, 0, 0.4) !important;
  }
}

.template-dialog :deep(.el-dialog__body) {
  scrollbar-width: none;
  -ms-overflow-style: none;
  contain: layout style paint;
  overflow-y: auto;
  overflow-x: hidden;
}

.template-dialog :deep(.el-dialog__body::-webkit-scrollbar) {
  display: none;
}
</style>

<style>
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
</style>
