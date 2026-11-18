<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  Copyright (c) 2025-2026 Seyud
  Device Faker: https://github.com/Seyud/device_faker
-->

<template>
  <el-dialog
    v-model="visible"
    :title="t('status.translators.dialog_title')"
    width="90%"
    :close-on-click-modal="false"
    :append-to-body="true"
    :destroy-on-close="true"
    :z-index="2001"
    class="translators-dialog"
    modal-class="translators-dialog-modal"
  >
    <div class="translators-dialog-content">
      <div class="translators-grid">
        <div v-for="(translator, locale) in translators" :key="locale" class="translator-card">
          <div class="translator-avatar">
            <img :src="translator.pp_url" :alt="translator.full_name" class="avatar-image" />
          </div>

          <div class="language-field">
            {{ translator.locale_name }}
          </div>

          <div class="translator-info">
            <h4 class="translator-name">{{ translator.full_name }}</h4>
            <p class="translator-username">@{{ translator.user_name }}</p>

            <div v-if="translator.socials" class="translator-socials">
              <button
                v-if="translator.socials.github"
                type="button"
                class="social-link github"
                :title="`GitHub: ${translator.socials.github}`"
                @click="openExternalUrl(`https://github.com/${translator.socials.github}`)"
              >
                <span class="brand-logo github-logo" aria-hidden="true">
                  <svg viewBox="0 0 24 24" role="img">
                    <path :d="siGithub.path" fill="currentColor" />
                  </svg>
                </span>
              </button>
              <button
                v-if="translator.socials.website"
                type="button"
                class="social-link website"
                title="Website"
                @click="openExternalUrl(translator.socials.website)"
              >
                <svg
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  stroke-width="2"
                  stroke-linecap="round"
                  stroke-linejoin="round"
                >
                  <circle cx="12" cy="12" r="10"></circle>
                  <path d="M2 12h20"></path>
                  <path
                    d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"
                  ></path>
                </svg>
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>

    <!-- 原生 button 替代 el-button:避免为 footer 单个取消按钮引入 Element Plus 组件 -->
    <template #footer>
      <button type="button" class="dialog-cancel-btn" @click="visible = false">
        {{ t('common.cancel') }}
      </button>
    </template>
  </el-dialog>
</template>

<script setup lang="ts">
import { computed } from 'vue'
import { siGithub } from 'simple-icons'
import { projectTranslators, useI18n } from '../../utils/i18n'
import { execCommand } from '../../utils/ksu'

const props = defineProps<{ modelValue: boolean }>()

const emit = defineEmits<{ 'update:modelValue': [boolean] }>()

const { t } = useI18n()

const visible = computed({
  get: () => props.modelValue,
  set: (val: boolean) => emit('update:modelValue', val),
})

const translators = computed(() => {
  return projectTranslators || {}
})

function escapeShellArg(value: string) {
  return value.replace(/'/g, `'\\''`)
}

async function openExternalUrl(url: string, fallbackUrl: string = url) {
  try {
    await execCommand(
      `am start -a android.intent.action.VIEW -c android.intent.category.BROWSABLE -d '${escapeShellArg(url)}' >/dev/null 2>&1`
    )
  } catch {
    window.open(fallbackUrl, '_blank', 'noopener,noreferrer')
  } finally {
    visible.value = false
  }
}
</script>

<style scoped>
.dialog-cancel-btn {
  padding: 0.5rem 1.25rem;
  border-radius: 0.5rem;
  border: 1px solid var(--border);
  background: transparent;
  color: var(--text);
  font-size: 0.875rem;
  cursor: pointer;
  -webkit-tap-highlight-color: transparent;
  user-select: none;
  -webkit-user-select: none;
}

.dialog-cancel-btn:active {
  transform: scale(0.97);
  opacity: 0.85;
}

.translators-dialog-content {
  display: flex;
  flex-direction: column;
  gap: 1.5rem;
}

.translators-grid {
  display: grid;
  grid-template-columns: repeat(auto-fill, minmax(200px, 1fr));
  gap: 1.25rem;
}

.translator-card {
  display: flex;
  flex-direction: column;
  align-items: center;
  padding: 1.5rem;
  background: var(--background);
  border-radius: 1rem;
  text-align: center;
  transition: all 0.3s ease;
}

.translator-card:hover {
  transform: translateY(-4px);
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.12);
}

.translator-avatar {
  position: relative;
  margin-bottom: 0.75rem;
}

.avatar-image {
  width: 100px;
  height: 100px;
  border-radius: 50%;
  object-fit: cover;
  border: 2px solid var(--primary);
}

.language-field {
  display: inline-block;
  background: linear-gradient(135deg, #0ea5e9 0%, #38bdf8 100%);
  color: white;
  font-size: 0.9rem;
  font-weight: 700;
  padding: 0.5rem 1rem;
  border-radius: 999px;
  margin-bottom: 0.75rem;
  white-space: nowrap;
}

.translator-info {
  display: flex;
  flex-direction: column;
  gap: 0.5rem;
  width: 100%;
}

.translator-name {
  font-size: 1.1rem;
  font-weight: 700;
  color: var(--text);
  margin: 0;
}

.translator-username {
  font-size: 0.85rem;
  color: var(--text-secondary);
  margin: 0;
}

.translator-socials {
  display: flex;
  justify-content: center;
  gap: 0.75rem;
  margin-top: 0.75rem;
}

.social-link {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 36px;
  height: 36px;
  border-radius: 50%;
  border: none;
  background: rgba(14, 165, 233, 0.12);
  color: var(--primary);
  cursor: pointer;
  transition: all 0.2s ease;
}

.social-link:hover {
  background: rgba(14, 165, 233, 0.25);
  transform: scale(1.1);
}

.social-link svg {
  width: 18px;
  height: 18px;
}

.brand-logo {
  display: inline-flex;
  align-items: center;
  justify-content: center;
  width: 1rem;
  height: 1rem;
  flex-shrink: 0;
}

.brand-logo svg {
  width: 100%;
  height: 100%;
}

.github-logo {
  color: currentColor;
}
</style>
