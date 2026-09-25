<template>
  <el-dialog
    v-model="visible"
    :title="t('status.follow.dialog_title')"
    width="90%"
    :close-on-click-modal="false"
    :append-to-body="true"
    :destroy-on-close="true"
    :z-index="2001"
    class="follow-dialog"
    modal-class="follow-dialog-modal"
  >
    <div class="follow-dialog-content">
      <div class="follow-row">
        <div class="follow-row-icon gradient-icon-2 author-icon-wrapper">
          <img
            src="https://avatars.githubusercontent.com/u/97807424?v=4"
            alt="Author"
            class="author-avatar-icon"
          />
        </div>
        <div class="follow-row-body">
          <span class="follow-row-label">{{ t('status.follow.author') }}</span>
          <div v-if="authorLinks.length > 0" class="community-links">
            <button
              v-for="authorLink in authorLinks"
              :key="authorLink.platform + authorLink.label"
              type="button"
              class="community-link author-pill"
              @click="openAuthorLink(authorLink.platform)"
            >
              <span
                v-if="authorLink.platform.toLowerCase() === 'github'"
                class="brand-logo github-logo"
                aria-hidden="true"
              >
                <svg viewBox="0 0 24 24" role="img">
                  <path :d="siGithub.path" fill="currentColor" />
                </svg>
              </span>
              <span
                v-else-if="authorLink.platform === '酷安'"
                class="brand-logo coolapk-logo"
                aria-hidden="true"
                >C</span
              >
              <span>{{ authorLink.fullText }}</span>
            </button>
          </div>
          <span v-else class="follow-row-value">{{ moduleAuthor }}</span>
        </div>
      </div>

      <div class="follow-row follow-row-communities">
        <div class="follow-row-icon gradient-icon-3">
          <MessageCircleMore :size="20" />
        </div>
        <div class="follow-row-body">
          <span class="follow-row-label">{{ t('status.follow.communities') }}</span>
          <div class="community-links">
            <button class="community-link" type="button" @click="openExternalUrl(qqGroupUrl)">
              <span class="brand-logo qq-logo" aria-hidden="true">
                <svg viewBox="0 0 24 24" role="img">
                  <path :d="siQq.path" fill="currentColor" />
                </svg>
              </span>
              <span>{{ t('status.follow.qq_group') }}</span>
            </button>
            <button
              class="community-link"
              type="button"
              @click="openExternalUrl(telegramIntentUrl, telegramWebUrl)"
            >
              <span class="brand-logo telegram-logo" aria-hidden="true">
                <svg viewBox="0 0 24 24" role="img">
                  <path :d="siTelegram.path" fill="currentColor" />
                </svg>
              </span>
              <span>{{ t('status.follow.telegram') }}</span>
            </button>
          </div>
        </div>
      </div>

      <div class="follow-row">
        <div class="follow-row-icon gradient-icon-6">
          <Globe :size="20" />
        </div>
        <div class="follow-row-body">
          <span class="follow-row-label">{{ t('status.follow.website') }}</span>
          <button class="repo-link" type="button" @click="openExternalUrl(websiteUrl)">
            <span class="brand-logo website-logo" aria-hidden="true">
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
            </span>
            <span class="repo-link-text">
              <span>{{ t('status.follow.website_action_primary') }}</span>
              <span>{{ t('status.follow.website_action_secondary') }}</span>
            </span>
          </button>
        </div>
      </div>

      <div class="follow-row">
        <div class="follow-row-icon gradient-icon-4">
          <svg viewBox="0 0 24 24" role="img" width="20" height="20" aria-hidden="true">
            <path :d="siGithub.path" fill="currentColor" />
          </svg>
        </div>
        <div class="follow-row-body">
          <span class="follow-row-label">{{ t('status.follow.repository') }}</span>
          <button class="repo-link" type="button" @click="openExternalUrl(repositoryUrl)">
            <span class="brand-logo github-logo" aria-hidden="true">
              <svg viewBox="0 0 24 24" role="img">
                <path :d="siGithub.path" fill="currentColor" />
              </svg>
            </span>
            <span class="repo-link-text">
              <span>{{ t('status.follow.repository_action_primary') }}</span>
              <span>{{ t('status.follow.repository_action_secondary') }}</span>
            </span>
          </button>
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
import { siGithub, siQq, siTelegram } from 'simple-icons'
import { Globe, MessageCircleMore } from '@lucide/vue'
import { useConfigStore } from '../../stores/config'
import { useI18n } from '../../utils/i18n'
import { execCommand } from '../../utils/ksu'

const props = defineProps<{ modelValue: boolean }>()

const emit = defineEmits<{ 'update:modelValue': [boolean] }>()

const configStore = useConfigStore()
const { t } = useI18n()

const visible = computed({
  get: () => props.modelValue,
  set: (val: boolean) => emit('update:modelValue', val),
})

const moduleAuthor = computed(() => configStore.moduleAuthor)

const qqGroupUrl =
  'https://qun.qq.com/universal-share/share?ac=1&authKey=ls4nlfcsF%2Bxp5SPnVsXRgpbeV1axPZb%2FmJCMXms6ZCHjgAwvOyl1LV%2BDNVL1btgL&busi_data=eyJncm91cENvZGUiOiI4NTQxODgyNTIiLCJ0b2tlbiI6IlE1WVVyZTZxUXVjZUtGUUxWSGFmbzkvMEd3UWNRSiszdklTZDhHejU0RDRyT0lWRTFqS3d4UGJSM1ltaXpkS3MiLCJ1aW4iOiIxMTA1NzgzMDMzIn0%3D&data=IbvhTKt9HwCSsCsl_610-rQ8p6H2NgLmxhEKkMcn-BMWPb86jygWBZJfWLQGm7J8LwpVV2yhPafxTMXYGkjRVA&svctype=4&tempid=h5_group_info'
const telegramIntentUrl = 'tg://resolve?domain=device_faker'
const telegramWebUrl = 'https://t.me/device_faker'
const websiteUrl = 'https://seyud.github.io/device_faker/'
const repositoryUrl = 'https://github.com/Seyud/device_faker'
const authorGithubUrl = 'https://github.com/Seyud'

const authorLinks = computed(() => {
  return moduleAuthor.value
    .split('/')
    .map((entry) => entry.trim())
    .filter(Boolean)
    .map((entry) => {
      const [platform, label] = entry.split('@')
      return {
        platform: platform?.trim() || '',
        label: label?.trim() || entry,
        fullText: entry,
      }
    })
    .filter((entry) => entry.label)
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

async function openCoolapkProfile() {
  try {
    await execCommand("am start -d 'coolmarket://u/4621247' >/dev/null 2>&1")
  } catch {
    window.open('https://www.coolapk.com/u/4621247', '_blank', 'noopener,noreferrer')
  } finally {
    visible.value = false
  }
}

function openAuthorLink(platform: string) {
  if (platform === '酷安') {
    void openCoolapkProfile()
    return
  }

  if (platform.toLowerCase() === 'github') {
    void openExternalUrl(authorGithubUrl)
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

.follow-dialog-content {
  display: flex;
  flex-direction: column;
  gap: 0.875rem;
}

.follow-row {
  display: flex;
  align-items: flex-start;
  gap: 0.875rem;
  padding: 1rem;
  background: var(--background);
  border-radius: 0.875rem;
}

.follow-row-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 40px;
  height: 40px;
  border-radius: 12px;
  flex-shrink: 0;
}

.gradient-icon-2 {
  background: linear-gradient(135deg, #06b6d4 0%, #0ea5e9 100%);
  color: white;
  box-shadow: 0 4px 12px rgba(6, 182, 212, 0.3);
}

.gradient-icon-3 {
  background: linear-gradient(135deg, #8b5cf6 0%, #a855f7 100%);
  color: white;
  box-shadow: 0 4px 12px rgba(139, 92, 246, 0.3);
}

.gradient-icon-4 {
  background: linear-gradient(135deg, #a855f7 0%, #c084fc 100%);
  color: white;
  box-shadow: 0 4px 12px rgba(168, 85, 247, 0.3);
}

.gradient-icon-6 {
  background: linear-gradient(135deg, #14b8a6 0%, #06b6d4 100%);
  color: white;
  box-shadow: 0 4px 12px rgba(20, 184, 166, 0.3);
}

.author-icon-wrapper {
  padding: 2px;
  overflow: hidden;
  background: linear-gradient(135deg, rgba(148, 163, 184, 0.18), rgba(148, 163, 184, 0.06));
  box-shadow:
    0 0 0 1px rgba(148, 163, 184, 0.22),
    0 2px 8px rgba(0, 0, 0, 0.08);
}

.dark .author-icon-wrapper {
  background: linear-gradient(135deg, rgba(148, 163, 184, 0.14), rgba(148, 163, 184, 0.04));
  box-shadow:
    0 0 0 1px rgba(148, 163, 184, 0.16),
    0 2px 8px rgba(0, 0, 0, 0.24);
}

.author-avatar-icon {
  width: 100%;
  height: 100%;
  border-radius: 10px;
  object-fit: cover;
  display: block;
}

.follow-row-body {
  display: flex;
  flex-direction: column;
  gap: 0.35rem;
  min-width: 0;
  flex: 1;
}

.follow-row-label {
  font-size: 0.875rem;
  color: var(--text-secondary);
}

.follow-row-value {
  font-size: 1rem;
  font-weight: 600;
  color: var(--text);
  word-break: break-word;
}

.community-links {
  display: flex;
  flex-wrap: wrap;
  gap: 0.75rem;
}

.community-link,
.repo-link {
  display: inline-flex;
  align-items: center;
  gap: 0.45rem;
  color: var(--primary);
  text-decoration: none;
  word-break: break-all;
  border: none;
  cursor: pointer;
}

.community-link {
  padding: 0.55rem 0.8rem;
  border-radius: 999px;
  background: rgba(14, 165, 233, 0.12);
  font-size: 0.875rem;
  font-weight: 500;
}

.repo-link {
  width: fit-content;
  padding: 0.6rem 0.85rem;
  border-radius: 0.75rem;
  background: rgba(14, 165, 233, 0.12);
  font-size: 0.95rem;
  line-height: 1.5;
}

.repo-link-text {
  display: flex;
  flex-direction: column;
  align-items: flex-start;
  line-height: 1.35;
}

.author-pill {
  font-weight: 600;
}

.author-pill:focus-visible {
  outline: 2px solid var(--primary);
  outline-offset: 2px;
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

.qq-logo {
  color: #12b7f5;
}

.telegram-logo {
  color: #27a7e7;
}

.coolapk-logo {
  color: #4caf50;
  font-size: 0.9rem;
  font-weight: 700;
}

.github-logo {
  color: currentColor;
}

.website-logo {
  color: currentColor;
}

@media (max-width: 520px) {
  .repo-link {
    width: 100%;
    justify-content: center;
  }

  .repo-link-text {
    align-items: center;
  }
}
</style>

<style>
/*
 * 注意:Element Plus 会把 class="follow-dialog" 落在 .el-dialog 元素本身,
 * 因此历史遗留的后代选择器 `.follow-dialog .el-dialog` 实际匹配不到,
 * 玻璃底样式从未生效(弹窗一直是 EP 默认不透明底,与其它弹窗一致)。
 * 这里保留原始选择器形态以维持既有外观;若要启用毛玻璃需改用
 * `.follow-dialog.el-dialog` 并同步统一所有弹窗,属视觉改版范畴。
 */
.follow-dialog .el-dialog {
  background: rgba(255, 255, 255, 0.15) !important;
  backdrop-filter: blur(40px) saturate(150%) brightness(1.1);
  border: 1px solid rgba(255, 255, 255, 0.4);
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.15);
}

.follow-dialog-modal {
  backdrop-filter: blur(12px) saturate(120%) !important;
  background-color: rgba(0, 0, 0, 0.25) !important;
}

@media (prefers-color-scheme: dark) {
  .follow-dialog .el-dialog {
    background: rgba(20, 20, 20, 0.6) !important;
    backdrop-filter: blur(40px) saturate(150%) brightness(0.9);
    border: 1px solid rgba(255, 255, 255, 0.15);
    box-shadow: 0 8px 32px rgba(0, 0, 0, 0.5);
  }

  .follow-dialog-modal {
    backdrop-filter: blur(12px) saturate(120%) !important;
    background-color: rgba(0, 0, 0, 0.4) !important;
  }
}
</style>
