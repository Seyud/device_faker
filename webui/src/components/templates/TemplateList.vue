<template>
  <div class="template-list">
    <TemplateCard
      v-for="entry in entries"
      :key="entry.name"
      :name="entry.name"
      :template="entry.template"
      @export="emit('export', entry.name, entry.template)"
      @edit="emit('edit', entry.name, entry.template)"
      @delete="emit('delete', entry.name)"
    />

    <div v-if="entries.length === 0" class="empty-state">
      <FileText :size="64" class="empty-icon" />
      <p class="empty-text">{{ emptyText }}</p>
      <p v-if="isSearching" class="empty-hint">{{ t('templates.search.no_results') }}</p>
      <p v-else class="empty-hint">{{ t('templates.empty.hint') }}</p>
    </div>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { FileText } from 'lucide-vue-next'
import TemplateCard from './TemplateCard.vue'
import { useI18n } from '../../utils/i18n'
import type { Template } from '../../types'

interface TemplateListEntry {
  name: string
  template: Template
}

const props = defineProps<{ entries: TemplateListEntry[]; isSearching?: boolean }>()
const emit = defineEmits<{
  export: [string, Template]
  edit: [string, Template]
  delete: [string]
}>()

const { t } = useI18n()

const entries = computed(() => props.entries)

const emptyText = computed(() => {
  if (props.isSearching && props.entries.length === 0) {
    return t('templates.search.no_results')
  }
  return t('templates.empty.title')
})
</script>

<style scoped>
.template-list {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  box-sizing: border-box;
}

.empty-state {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: center;
  padding: 3rem 1rem;
  text-align: center;
  border-radius: 1rem;
  background: var(--card);
  border: 1px solid var(--border);
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
</style>
