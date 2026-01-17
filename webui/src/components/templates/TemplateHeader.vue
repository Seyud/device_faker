<template>
  <div class="page-header">
    <h2 class="page-title">{{ t('templates.title') }}</h2>
    <div class="header-toolbar">
      <div class="search-wrapper">
        <Search :size="18" class="search-icon" />
        <input
          v-model="searchQuery"
          type="text"
          class="search-input"
          :placeholder="t('templates.search.placeholder')"
          @input="emit('search', searchQuery)"
        />
        <button v-if="searchQuery" class="clear-btn" @click="clearSearch">
          <X :size="16" />
        </button>
      </div>
      <div class="header-actions" :class="{ 'vertical-layout': locale === 'en' }">
        <button class="add-btn secondary" @click="emit('open-online')">
          <Download :size="20" />
          {{ t('templates.actions.online') }}
        </button>
        <button class="add-btn" @click="emit('open-create')">
          <Plus :size="20" />
          {{ t('templates.actions.new') }}
        </button>
      </div>
    </div>
  </div>
</template>

<script setup lang="ts">
import { Plus, Download, Search, X } from 'lucide-vue-next'
import { toRefs, ref, watch } from 'vue'
import { useI18n } from '../../utils/i18n'

const props = defineProps<{ locale: string }>()
const { locale } = toRefs(props)
const emit = defineEmits<{ 'open-online': []; 'open-create': []; search: [string] }>()

const { t } = useI18n()
const searchQuery = ref('')

function clearSearch() {
  searchQuery.value = ''
  emit('search', '')
}

watch(searchQuery, (value) => {
  emit('search', value)
})
</script>

<style scoped>
.page-header {
  display: flex;
  flex-direction: column;
  gap: 0.75rem;
  margin-bottom: 0.5rem;
}

.page-title {
  font-size: 1.5rem;
  font-weight: 600;
  color: var(--text);
  line-height: 1.3;
}

.header-toolbar {
  display: flex;
  align-items: center;
  gap: 0.75rem;
  flex-wrap: wrap;
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

.search-wrapper {
  display: flex;
  align-items: center;
  gap: 0.5rem;
  padding: 0.5rem 0.75rem;
  background: var(--card);
  border: 1px solid var(--border);
  border-radius: 0.5rem;
  flex: 1;
  max-width: 400px;
  min-width: 200px;
}

.search-icon {
  color: var(--text-secondary);
  flex-shrink: 0;
}

.search-input {
  flex: 1;
  border: none;
  background: transparent;
  font-size: 0.875rem;
  color: var(--text);
  outline: none;
  min-width: 0;
}

.search-input::placeholder {
  color: var(--text-secondary);
}

.clear-btn {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 20px;
  height: 20px;
  background: var(--background);
  border: none;
  border-radius: 50%;
  color: var(--text-secondary);
  cursor: pointer;
  transition: all 0.2s ease;
  flex-shrink: 0;
}

.clear-btn:hover {
  background: var(--border);
  color: var(--text);
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
</style>
