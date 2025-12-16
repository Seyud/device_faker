<template>
  <div class="template-card glass-effect">
    <div class="template-header">
      <div class="template-info">
        <h3 class="template-name">{{ name }}</h3>
        <p class="template-device">{{ template.brand }} {{ template.model }}</p>
      </div>
      <div class="template-actions">
        <button class="icon-btn" @click="emit('edit', name, template)">
          <Edit2 :size="18" />
        </button>
        <button class="icon-btn danger" @click="emit('delete', name)">
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
        <span class="detail-value">
          {{
            template.mode === 'lite'
              ? t('templates.values.lite')
              : template.mode === 'full'
                ? t('templates.values.full')
                : t('templates.values.resetprop')
          }}
        </span>
      </div>
      <div v-if="template.characteristics" class="detail-item">
        <span class="detail-label">{{ t('templates.fields.characteristics') }}:</span>
        <span class="detail-value">{{ template.characteristics }}</span>
      </div>
      <div v-if="template.force_denylist_unmount !== undefined" class="detail-item">
        <span class="detail-label">{{ t('templates.fields.force_denylist_unmount') }}:</span>
        <span class="detail-value">
          {{ template.force_denylist_unmount ? t('common.enabled') : t('common.disabled') }}
        </span>
      </div>
      <div v-if="template.packages && template.packages.length > 0" class="detail-item">
        <span class="detail-label">{{ t('templates.labels.packages') }}:</span>
        <span class="detail-value"
          >{{ template.packages.length }} {{ t('templates.labels.count_suffix') }}</span
        >
      </div>
    </div>

    <!-- 可选元数据字段 -->
    <div
      v-if="template.version || template.version_code || template.author || template.description"
      class="template-meta"
    >
      <div v-if="template.version || template.version_code" class="meta-item">
        <span class="meta-label">{{ t('templates.labels.version') }}:</span>
        <span class="meta-value">
          {{ template.version || '' }}
          <span v-if="template.version_code" class="version-code"
            >({{ template.version_code }})</span
          >
        </span>
      </div>
      <div v-if="template.author" class="meta-item">
        <span class="meta-label">{{ t('templates.labels.author') }}:</span>
        <span class="meta-value">{{ template.author }}</span>
      </div>
      <div v-if="template.description" class="meta-item meta-description">
        <span class="meta-label">{{ t('templates.labels.description') }}:</span>
        <span class="meta-value">{{ template.description }}</span>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { Edit2, Trash2 } from 'lucide-vue-next'
import { toRefs } from 'vue'
import { useI18n } from '../../utils/i18n'
import type { Template } from '../../types'

const props = defineProps<{ name: string; template: Template }>()
const { name, template } = toRefs(props)
const emit = defineEmits<{ edit: [string, Template]; delete: [string] }>()

const { t } = useI18n()
</script>

<style scoped>
.template-card {
  padding: 1.25rem;
  border-radius: 1rem;
  box-shadow: 0 2px 8px var(--shadow);
  width: 100%;
  min-width: 0;
  box-sizing: border-box;
  position: relative;
  transform: translateZ(0);
  contain: layout style paint;
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
  width: 100%;
  max-width: calc(100% - 80px);
}

.template-device {
  font-size: 0.875rem;
  color: var(--text-secondary);
  white-space: nowrap;
  overflow: hidden;
  text-overflow: ellipsis;
  width: 100%;
  max-width: calc(100% - 80px);
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

.template-meta {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  margin-top: 0.75rem;
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
