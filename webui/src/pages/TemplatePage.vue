<template>
  <div class="template-page">
    <TemplateHeader
      :locale="locale"
      @open-online="showOnlineDialog"
      @open-create="showCreateDialog"
      @search="handleSearch"
    />

    <TemplateList
      :templates="filteredTemplates"
      :is-searching="searchQuery.length > 0"
      @edit="handleEdit"
      @delete="deleteTemplateConfirm"
    />

    <TemplateDialog
      v-model="dialogVisible"
      :is-editing="isEditing"
      :locale="locale"
      :template-name="editingTemplateName"
      :template-data="editingTemplate"
      @saved="handleTemplateSaved"
    />

    <OnlineTemplateDialog v-model="onlineDialogVisible" />
  </div>
</template>

<script setup lang="ts">
import { computed, onActivated, onMounted, ref } from 'vue'
import TemplateDialog from '../components/templates/TemplateDialog.vue'
import TemplateHeader from '../components/templates/TemplateHeader.vue'
import TemplateList from '../components/templates/TemplateList.vue'
import OnlineTemplateDialog from '../components/OnlineTemplateDialog.vue'
import { useAppsStore } from '../stores/apps'
import { useConfigStore } from '../stores/config'
import { useI18n } from '../utils/i18n'
import { useLazyMessageBox } from '../utils/elementPlus'
import { toast } from 'kernelsu-alt'
import type { Template } from '../types'

const configStore = useConfigStore()
const appsStore = useAppsStore()
const { t, locale } = useI18n()
const getMessageBox = useLazyMessageBox()

const searchQuery = ref('')

const allTemplates = computed(() => configStore.getTemplates())

const filteredTemplates = computed(() => {
  if (!searchQuery.value.trim()) {
    return allTemplates.value
  }

  const query = searchQuery.value.toLowerCase().trim()

  return Object.entries(allTemplates.value).reduce<Record<string, Template>>(
    (acc, [name, template]) => {
      const searchFields = [
        name,
        template.brand || '',
        template.model || '',
        template.device || '',
        template.manufacturer || '',
        template.product || '',
      ]

      const matches = searchFields.some((field) => field.toLowerCase().includes(query))

      if (matches) {
        acc[name] = template
      }

      return acc
    },
    {}
  )
})

function handleSearch(query: string) {
  searchQuery.value = query
}

const dialogVisible = ref(false)
const onlineDialogVisible = ref(false)
const isEditing = ref(false)
const editingTemplateName = ref<string | null>(null)
const editingTemplate = ref<Template | null>(null)

function showOnlineDialog() {
  onlineDialogVisible.value = true
}

function showCreateDialog() {
  isEditing.value = false
  editingTemplateName.value = null
  editingTemplate.value = null
  dialogVisible.value = true
}

function handleEdit(name: string, template: Template) {
  isEditing.value = true
  editingTemplateName.value = name
  editingTemplate.value = template
  dialogVisible.value = true
}

async function deleteTemplateConfirm(name: string) {
  try {
    const messageBox = await getMessageBox()
    await messageBox.confirm(
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
    toast(t('templates.messages.deleted'))
  } catch {
    // 用户取消
  }
}

function handleTemplateSaved() {
  // 保存后无需额外动作，保留扩展点
}

onMounted(() => {
  if (appsStore.installedApps.length === 0) {
    appsStore.loadInstalledApps()
  }
})

onActivated(() => {
  // KeepAlive 激活时触发一次尺寸计算，确保列表布局正确
  window.dispatchEvent(new Event('resize'))
  window.dispatchEvent(new Event('scroll'))
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
  overflow: hidden;
}
</style>

<style>
.delete-confirm-modal {
  backdrop-filter: blur(12px) saturate(120%) !important;
  -webkit-backdrop-filter: blur(12px) saturate(120%) !important;
  background-color: rgba(0, 0, 0, 0.15) !important;
}

.dark .delete-confirm-modal {
  backdrop-filter: blur(12px) saturate(120%) !important;
  -webkit-backdrop-filter: blur(12px) saturate(120%) !important;
  background-color: rgba(0, 0, 0, 0.4) !important;
}

.delete-confirm-box {
  background: rgba(255, 255, 255, 0.95) !important;
  backdrop-filter: blur(40px) saturate(150%) brightness(1.1) !important;
  -webkit-backdrop-filter: blur(40px) saturate(150%) brightness(1.1) !important;
  border: 1px solid rgba(0, 0, 0, 0.1) !important;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.1) !important;
}

.dark .delete-confirm-box {
  background: rgba(20, 20, 20, 0.6) !important;
  backdrop-filter: blur(40px) saturate(150%) brightness(0.9) !important;
  -webkit-backdrop-filter: blur(40px) saturate(150%) brightness(0.9) !important;
  border: 1px solid rgba(255, 255, 255, 0.15) !important;
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5) !important;
}
</style>
