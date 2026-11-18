<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  Copyright (c) 2025-2026 Seyud
  Device Faker: https://github.com/Seyud/device_faker
-->

<template>
  <div class="form-row">
    <el-form-item :label="t('templates.fields.manufacturer')">
      <el-input
        v-model="formData.manufacturer"
        :placeholder="t('templates.placeholders.manufacturer')"
      />
    </el-form-item>

    <el-form-item :label="t('templates.fields.brand')">
      <el-input v-model="formData.brand" :placeholder="t('templates.placeholders.brand')" />
    </el-form-item>
  </div>

  <el-form-item :label="t('templates.fields.model')">
    <el-input v-model="formData.model" :placeholder="t('templates.placeholders.model')" />
  </el-form-item>

  <el-form-item :label="t('templates.fields.device')">
    <el-input v-model="formData.device" :placeholder="t('templates.placeholders.device')" />
  </el-form-item>

  <el-form-item :label="t('templates.fields.product')">
    <el-input v-model="formData.product" :placeholder="t('templates.placeholders.product')" />
  </el-form-item>

  <el-form-item :label="t('templates.fields.name_field')">
    <el-input v-model="formData.name" :placeholder="t('templates.placeholders.name_field')" />
  </el-form-item>

  <el-form-item :label="t('templates.fields.market_name')">
    <el-input
      v-model="formData.marketname"
      :placeholder="t('templates.placeholders.market_name')"
    />
  </el-form-item>

  <el-form-item :label="t('templates.fields.fingerprint')">
    <el-input
      v-model="formData.fingerprint"
      type="textarea"
      :rows="3"
      :placeholder="t('templates.placeholders.fingerprint')"
    />
  </el-form-item>

  <el-form-item :label="t('templates.fields.hardware')">
    <el-input v-model="formData.hardware" :placeholder="t('templates.placeholders.hardware')" />
  </el-form-item>

  <el-form-item :label="t('templates.fields.board')">
    <el-input v-model="formData.board" :placeholder="t('templates.placeholders.board')" />
  </el-form-item>

  <el-form-item :label="t('templates.fields.soc_model')">
    <el-input v-model="formData.soc_model" :placeholder="t('templates.placeholders.soc_model')" />
  </el-form-item>

  <el-collapse>
    <el-collapse-item :title="t('templates.fields.system')" name="system">
      <el-form-item :label="t('templates.fields.build_id')">
        <el-input v-model="formData.build_id" :placeholder="t('templates.placeholders.build_id')" />
      </el-form-item>

      <el-form-item :label="t('templates.fields.display_id')">
        <el-input
          v-model="formData.display_id"
          :placeholder="t('templates.placeholders.display_id')"
        />
      </el-form-item>

      <el-form-item :label="t('templates.fields.incremental')">
        <el-input
          v-model="formData.incremental"
          :placeholder="t('templates.placeholders.incremental')"
        />
      </el-form-item>

      <el-form-item :label="t('templates.fields.security_patch')">
        <el-input
          v-model="formData.security_patch"
          :placeholder="t('templates.placeholders.security_patch')"
        />
      </el-form-item>

      <el-form-item :label="t('templates.fields.android_version')">
        <el-input
          v-model="formData.android_version"
          :placeholder="t('templates.placeholders.android_version')"
        />
      </el-form-item>

      <el-form-item :label="t('templates.fields.sdk_int')">
        <el-input
          v-model="formData.sdk_int"
          type="number"
          :placeholder="t('templates.placeholders.sdk_int')"
        />
      </el-form-item>

      <el-form-item :label="t('templates.fields.dpi')">
        <el-input
          v-model="formData.dpi"
          type="number"
          min="120"
          max="640"
          :placeholder="t('templates.placeholders.dpi')"
        />
      </el-form-item>
    </el-collapse-item>
  </el-collapse>

  <el-form-item :label="t('templates.fields.characteristics')">
    <el-input
      v-model="formData.characteristics"
      :placeholder="t('templates.placeholders.characteristics')"
    />
  </el-form-item>

  <el-form-item :label="t('templates.fields.companion_resetprop')">
    <el-select
      v-model="formData.companion_resetprop"
      :placeholder="t('common.default') + ' (' + t('common.disabled') + ')'"
      clearable
      style="width: 100%"
    >
      <el-option :label="t('common.enabled')" :value="true" />
      <el-option :label="t('common.disabled')" :value="false" />
    </el-select>
  </el-form-item>

  <el-collapse>
    <el-collapse-item :title="t('templates.fields.custom_props')" name="customProps">
      <div v-if="formData.customProps.length === 0" class="custom-props-empty">
        {{ t('templates.customProps.empty') }}
      </div>

      <div v-for="(entry, index) in formData.customProps" :key="index" class="custom-prop-entry">
        <div class="custom-prop-row">
          <el-input
            v-model="entry.key"
            :placeholder="t('templates.placeholders.custom_prop_key')"
          />
          <span class="custom-prop-separator">=</span>
          <el-input
            v-model="entry.value"
            :disabled="isSpecialValue(entry.value)"
            :placeholder="t('templates.placeholders.custom_prop_value')"
          />
        </div>
        <div class="custom-prop-actions">
          <el-button class="custom-prop-icon-btn" @click="removeCustomProp(index)">
            <Trash :size="16" />
          </el-button>
          <el-dropdown
            trigger="click"
            @command="(command: string) => applySpecialValue(index, command)"
          >
            <el-button
              class="custom-prop-icon-btn"
              :type="isSpecialValue(entry.value) ? 'primary' : 'default'"
            >
              <ChevronDown :size="16" />
            </el-button>
            <template #dropdown>
              <el-dropdown-menu>
                <el-dropdown-item command="__EMPTY__">
                  {{ t('templates.customProps.set_empty') }}
                </el-dropdown-item>
                <el-dropdown-item command="__DELETE__">
                  {{ t('templates.customProps.set_delete') }}
                </el-dropdown-item>
                <el-dropdown-item
                  command="__CUSTOM__"
                  :disabled="!isSpecialValue(entry.value)"
                  divided
                >
                  {{ t('templates.customProps.manual') }}
                </el-dropdown-item>
              </el-dropdown-menu>
            </template>
          </el-dropdown>
        </div>
      </div>

      <el-button class="custom-prop-add" @click="addCustomProp">
        <Plus :size="16" />
        {{ t('templates.customProps.add') }}
      </el-button>
    </el-collapse-item>
  </el-collapse>

  <slot name="packages" />
</template>

<script setup lang="ts">
import { ChevronDown, Plus, Trash } from '@lucide/vue'
import { useI18n } from '../../utils/i18n'
import { useDeviceFakerFormField } from '../../composables/useDeviceFakerForm'

const formData = useDeviceFakerFormField()

const { t } = useI18n()

const SPECIAL_PROP_VALUES = ['__EMPTY__', '__DELETE__']

function isSpecialValue(value: string): boolean {
  return SPECIAL_PROP_VALUES.includes(value)
}

function addCustomProp() {
  formData.value.customProps.push({ key: '', value: '' })
}

function removeCustomProp(index: number) {
  formData.value.customProps.splice(index, 1)
}

function applySpecialValue(index: number, command: string) {
  const entry = formData.value.customProps[index]
  if (!entry) return
  // 恢复手动输入时清空旧的特殊值；其余命令直接写入字面量
  entry.value = command === '__CUSTOM__' ? '' : command
}
</script>

<style scoped>
.form-row {
  display: flex;
  gap: 0.75rem;
}

.form-row .el-form-item {
  flex: 1;
  min-width: 0;
}

.custom-props-empty {
  color: var(--text-secondary, #909399);
  font-size: 0.875rem;
  padding: 0.25rem 0 0.75rem;
}

.custom-prop-entry {
  display: flex;
  flex-direction: column;
  gap: 0.375rem;
  margin-bottom: 0.75rem;
}

.custom-prop-row {
  display: flex;
  align-items: center;
  gap: 0.5rem;
}

.custom-prop-row .el-input {
  flex: 1;
  min-width: 0;
}

.custom-prop-separator {
  flex-shrink: 0;
  color: var(--text-secondary, #909399);
  font-weight: 600;
  user-select: none;
}

.custom-prop-actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.custom-prop-actions :deep(.el-dropdown) {
  flex-shrink: 0;
}

.custom-prop-actions :deep(.el-dropdown:focus-visible) {
  outline: none;
}

.custom-prop-icon-btn {
  flex-shrink: 0;
  padding: 0.5rem;
}

.custom-prop-add {
  width: 100%;
  margin-top: 0.25rem;
}
</style>
