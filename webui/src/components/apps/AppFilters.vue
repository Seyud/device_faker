<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  Copyright (c) 2025-2026 Seyud
  Device Faker: https://github.com/Seyud/device_faker
-->

<template>
  <div class="page-header">
    <div class="page-header-top">
      <h2 class="page-title">{{ t('apps.title') }}</h2>
      <el-button class="system-toggle-btn" plain size="small" @click="toggleSystemApps">
        {{ showSystemApps ? t('apps.actions.hide_system') : t('apps.actions.show_system') }}
      </el-button>
    </div>
    <el-input
      v-model="searchModel"
      :placeholder="t('apps.search_placeholder')"
      clearable
      class="search-input"
    >
      <template #prefix>
        <el-icon><Search /></el-icon>
      </template>
    </el-input>
  </div>

  <div class="filter-tabs">
    <button :class="['filter-tab', { active: filterType === 'all' }]" @click="setFilter('all')">
      {{ t('apps.tabs.all') }} <span v-if="!loading">({{ totalCount }})</span>
    </button>
    <button
      :class="['filter-tab', { active: filterType === 'configured' }]"
      @click="setFilter('configured')"
    >
      {{ t('apps.tabs.configured') }} ({{ configuredCount }})
    </button>
  </div>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { Search } from '@element-plus/icons-vue'
import { useI18n } from '../../utils/i18n'

type FilterType = 'all' | 'configured'

const props = defineProps<{
  searchQuery: string
  filterType: FilterType
  totalCount: number
  configuredCount: number
  showSystemApps: boolean
  loading?: boolean
}>()

const emit = defineEmits<{
  'update:searchQuery': [string]
  'update:filterType': [FilterType]
  'update:showSystemApps': [boolean]
}>()

const { t } = useI18n()

const searchModel = computed({
  get: () => props.searchQuery,
  set: (val: string) => emit('update:searchQuery', val),
})

const setFilter = (type: FilterType) => emit('update:filterType', type)
const toggleSystemApps = () => emit('update:showSystemApps', !props.showSystemApps)
</script>

<style scoped>
.page-header {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  gap: 1rem;
  margin-bottom: 0.5rem;
}

.page-header-top {
  display: flex;
  align-items: flex-start;
  justify-content: space-between;
  gap: 0.75rem;
  width: 100%;
  min-width: 0;
}

.page-title {
  flex: 1 1 auto;
  min-width: 0;
  margin: 0;
  font-size: 1.5rem;
  font-weight: 600;
  color: var(--text);
  line-height: 1.2;
}

.system-toggle-btn {
  flex: 0 1 11rem;
  min-width: 0;
  height: auto;
  margin-left: auto;
  padding: 0.5rem 0.875rem;
  border-radius: 0.75rem;
  border-color: var(--border);
  background: var(--card-bg);
  color: var(--text);
  box-shadow: 0 2px 6px var(--shadow);
  transition:
    transform 0.16s ease,
    opacity 0.16s ease,
    color 0.2s ease,
    border-color 0.2s ease,
    box-shadow 0.2s ease;
}

.system-toggle-btn :deep(span) {
  white-space: normal;
  text-align: center;
  line-height: 1.2;
}

.system-toggle-btn:hover,
.system-toggle-btn:focus-visible {
  border-color: var(--primary);
  color: var(--primary);
}

.system-toggle-btn:active {
  transform: scale(0.97);
  opacity: 0.88;
  box-shadow: 0 1px 3px var(--shadow);
}

.search-input {
  width: 100%;
  max-width: 400px;
}

.filter-tabs {
  display: flex;
  gap: 0.5rem;
  padding-bottom: 0.5rem;
}

.filter-tab {
  flex: 1;
  padding: 0.5rem 1rem;
  background: var(--card-bg);
  border: 1px solid var(--border);
  border-radius: 0.5rem;
  color: var(--text-secondary);
  font-size: 0.875rem;
  font-weight: 500;
  transition: all 0.2s ease;
  white-space: nowrap;
  text-align: center;
  -webkit-tap-highlight-color: transparent;
  user-select: none;
  -webkit-user-select: none;
}

.filter-tab:active {
  transform: scale(0.95);
  opacity: 0.8;
}

.filter-tab.active {
  background: var(--primary);
  color: white;
  border-color: var(--primary);
}
</style>
