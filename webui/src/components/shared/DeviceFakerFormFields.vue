<template>
  <el-form-item :label="t('templates.fields.manufacturer')">
    <el-input
      v-model="formData.manufacturer"
      :placeholder="t('templates.placeholders.manufacturer')"
    />
  </el-form-item>

  <el-form-item :label="t('templates.fields.brand')">
    <el-input v-model="formData.brand" :placeholder="t('templates.placeholders.brand')" />
  </el-form-item>

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

  <el-collapse>
    <el-collapse-item :title="t('templates.fields.system')" name="system">
      <el-form-item :label="t('templates.fields.build_id')">
        <el-input v-model="formData.build_id" :placeholder="t('templates.placeholders.build_id')" />
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
    </el-collapse-item>
  </el-collapse>

  <el-form-item :label="t('templates.fields.mode')">
    <el-select
      v-model="formData.mode"
      :placeholder="t('templates.placeholders.mode')"
      clearable
      popper-class="mode-select-popper"
      style="width: 100%"
    >
      <el-option :label="t('templates.options.mode_lite')" value="lite" />
      <el-option :label="t('templates.options.mode_full')" value="full" />
      <el-option :label="t('templates.options.mode_companion')" value="companion" />
    </el-select>
  </el-form-item>

  <el-form-item
    v-if="
      formData.mode === 'full' || (!formData.mode && configStore.config.default_mode === 'full')
    "
    :label="t('templates.fields.characteristics')"
  >
    <el-input
      v-model="formData.characteristics"
      :placeholder="t('templates.placeholders.characteristics')"
    />
  </el-form-item>

  <el-form-item :label="t('templates.fields.force_denylist_unmount')">
    <el-select
      v-model="formData.force_denylist_unmount"
      :placeholder="t('common.default')"
      style="width: 100%"
    >
      <el-option :label="t('common.default')" :value="undefined" />
      <el-option :label="t('common.enabled')" :value="true" />
      <el-option :label="t('common.disabled')" :value="false" />
    </el-select>
  </el-form-item>

  <el-form-item :label="t('templates.fields.cpu_spoof')">
    <el-select
      v-model="formData.cpu_spoof"
      :placeholder="t('templates.placeholders.cpu_spoof')"
      clearable
      style="width: 100%"
    >
      <el-option v-for="name in availableCpuPresets" :key="name" :label="name" :value="name" />
    </el-select>
  </el-form-item>

  <el-form-item :label="t('templates.fields.cpu_spoof_custom')">
    <el-input
      v-model="formData.cpu_spoof_custom"
      type="textarea"
      :rows="8"
      :placeholder="t('templates.placeholders.cpu_spoof_custom')"
    />
  </el-form-item>

  <slot name="packages" />
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { useI18n } from '../../utils/i18n'
import { useConfigStore } from '../../stores/config'
import { useDeviceFakerFormField } from '../../composables/useDeviceFakerForm'

const formData = useDeviceFakerFormField()

const { t } = useI18n()
const configStore = useConfigStore()

const availableCpuPresets = computed(() => {
  const presets = configStore.config.cpu_presets
  if (!presets) return []
  return Object.keys(presets)
})
</script>

<style>
.mode-select-popper .el-select-dropdown__item {
  white-space: pre-line;
  line-height: 1.4;
  height: auto;
  padding-top: 8px;
  padding-bottom: 8px;
  word-break: break-word;
}

.mode-select-popper .el-select-dropdown__item span {
  white-space: pre-line;
}
</style>
