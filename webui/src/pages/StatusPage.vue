<template>
  <div class="status-page">
    <div class="status-card glass-effect">
      <h2 class="card-title">{{ t('status.items.module_status') }}</h2>

      <div class="status-grid">
        <div class="status-item">
          <div class="status-icon gradient-icon-1">
            <Shield :size="32" />
          </div>
          <div class="status-info">
            <span class="status-label">{{ t('status.items.module_version') }}</span>
            <span class="status-transition-slot">
              <Transition name="status-swap">
                <span
                  v-if="moduleMetaReady"
                  key="module-version"
                  class="status-value status-value--resolved"
                >
                  {{ moduleVersionDisplay }}
                </span>
                <span
                  v-else
                  key="module-version-skeleton"
                  class="status-value-skeleton status-value-skeleton--wide"
                ></span>
              </Transition>
            </span>
            <span class="status-transition-slot status-transition-slot--build">
              <Transition name="status-swap">
                <span v-if="moduleVersionBuild" key="module-build" class="status-build">{{
                  moduleVersionBuild
                }}</span>
                <span
                  v-else-if="!moduleMetaReady"
                  key="module-build-skeleton"
                  class="status-build-skeleton"
                ></span>
                <span v-else key="module-build-empty" class="status-build-placeholder"></span>
              </Transition>
            </span>
          </div>
        </div>

        <div class="status-item">
          <div class="status-icon gradient-icon-2">
            <Smartphone :size="32" />
          </div>
          <div class="status-info">
            <span class="status-label">{{ t('status.items.impersonated_apps_count') }}</span>
            <span class="status-transition-slot">
              <Transition name="status-swap">
                <span
                  v-if="configReady"
                  key="device-faker-count"
                  class="status-value status-value--resolved"
                >
                  {{ deviceFakerCountDisplay }}
                </span>
                <span
                  v-else
                  key="device-faker-count-skeleton"
                  class="status-value-skeleton status-value-skeleton--short"
                ></span>
              </Transition>
            </span>
          </div>
        </div>

        <div class="status-item">
          <div class="status-icon gradient-icon-3">
            <FileText :size="32" />
          </div>
          <div class="status-info">
            <span class="status-label">{{ t('status.items.templates_count') }}</span>
            <span class="status-transition-slot">
              <Transition name="status-swap">
                <span
                  v-if="configReady"
                  key="template-count"
                  class="status-value status-value--resolved"
                >
                  {{ templateCountDisplay }}
                </span>
                <span
                  v-else
                  key="template-count-skeleton"
                  class="status-value-skeleton status-value-skeleton--short"
                ></span>
              </Transition>
            </span>
          </div>
        </div>

        <div
          :class="['status-item', { clickable: canToggleWorkMode, disabled: !canToggleWorkMode }]"
          @click="handleToggleWorkMode"
        >
          <div class="status-icon gradient-icon-4">
            <Settings :size="32" />
          </div>
          <div class="status-info">
            <span class="status-label">{{ t('status.items.work_mode') }}</span>
            <span class="status-transition-slot">
              <Transition name="status-swap">
                <span
                  v-if="configReady"
                  key="work-mode"
                  class="status-value status-value--resolved"
                >
                  {{ workMode }}
                </span>
                <span
                  v-else
                  key="work-mode-skeleton"
                  class="status-value-skeleton status-value-skeleton--medium"
                ></span>
              </Transition>
            </span>
          </div>
        </div>

        <div class="status-item clickable" @click="followDialogVisible = true">
          <div class="status-icon gradient-icon-5">
            <HeartHandshake :size="32" />
          </div>
          <div class="status-info">
            <span class="status-label">{{ t('status.follow.title') }}</span>
            <span class="status-value">{{ t('status.follow.action') }}</span>
            <span class="status-build">{{ t('status.follow.channels') }}</span>
          </div>
        </div>

        <div class="status-item clickable" @click="translatorsDialogVisible = true">
          <div class="status-icon gradient-icon-5">
            <Languages :size="32" />
          </div>
          <div class="status-info">
            <span class="status-label">{{ t('status.translators.title') }}</span>
            <span class="status-value">{{ t('status.translators.action') }}</span>
          </div>
        </div>
      </div>
    </div>

    <el-dialog
      v-model="translatorsDialogVisible"
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

      <template #footer>
        <el-button @click="translatorsDialogVisible = false">{{ t('common.cancel') }}</el-button>
      </template>
    </el-dialog>

    <el-dialog
      v-model="followDialogVisible"
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
          <div class="follow-row-icon gradient-icon-4">
            <Github :size="20" />
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

      <template #footer>
        <el-button @click="followDialogVisible = false">{{ t('common.cancel') }}</el-button>
      </template>
    </el-dialog>
  </div>
</template>

<script setup lang="ts">
import { computed, onActivated, ref } from 'vue'
import {
  Shield,
  Smartphone,
  FileText,
  Settings,
  Languages,
  HeartHandshake,
  MessageCircleMore,
  Github,
} from 'lucide-vue-next'
import { siGithub, siQq, siTelegram } from 'simple-icons'
import { useConfigStore } from '../stores/config'
import { projectTranslators, useI18n } from '../utils/i18n'
import { execCommand } from '../utils/ksu'

const configStore = useConfigStore()
const { t } = useI18n()
const followDialogVisible = ref(false)
const translatorsDialogVisible = ref(false)

const translators = computed(() => {
  return projectTranslators || {}
})

const qqGroupUrl =
  'https://qun.qq.com/universal-share/share?ac=1&authKey=ls4nlfcsF%2Bxp5SPnVsXRgpbeV1axPZb%2FmJCMXms6ZCHjgAwvOyl1LV%2BDNVL1btgL&busi_data=eyJncm91cENvZGUiOiI4NTQxODgyNTIiLCJ0b2tlbiI6IlE1WVVyZTZxUXVjZUtGUUxWSGFmbzkvMEd3UWNRSiszdklTZDhHejU0RDRyT0lWRTFqS3d4UGJSM1ltaXpkS3MiLCJ1aW4iOiIxMTA1NzgzMDMzIn0%3D&data=IbvhTKt9HwCSsCsl_610-rQ8p6H2NgLmxhEKkMcn-BMWPb86jygWBZJfWLQGm7J8LwpVV2yhPafxTMXYGkjRVA&svctype=4&tempid=h5_group_info'
const telegramIntentUrl = 'tg://resolve?domain=device_faker'
const telegramWebUrl = 'https://t.me/device_faker'
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
    followDialogVisible.value = false
    translatorsDialogVisible.value = false
  }
}

async function openCoolapkProfile() {
  try {
    await execCommand("am start -d 'coolmarket://u/4621247' >/dev/null 2>&1")
  } catch {
    window.open('https://www.coolapk.com/u/4621247', '_blank', 'noopener,noreferrer')
  } finally {
    followDialogVisible.value = false
    translatorsDialogVisible.value = false
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

// 直接使用 store 中的 computed 属性，避免重复计算
const moduleVersion = computed(() => configStore.moduleVersion)
const moduleAuthor = computed(() => configStore.moduleAuthor)
const configReady = computed(() => configStore.configReady)
const moduleMetaReady = computed(() => configStore.moduleMetaReady)
const canToggleWorkMode = computed(() => configReady.value)
const moduleVersionDisplay = computed(() =>
  moduleMetaReady.value ? moduleVersionMain.value : '--'
)
const moduleVersionMain = computed(() => {
  const v = moduleVersion.value
  const idx = v.indexOf('(')
  return idx > 0 ? v.substring(0, idx).trim() : v
})
const moduleVersionBuild = computed(() => {
  if (!moduleMetaReady.value) {
    return ''
  }

  const v = moduleVersion.value
  const match = v.match(/\((.+)\)/)
  return match ? match[1] : ''
})
const deviceFakerCountDisplay = computed(() =>
  configReady.value ? String(configStore.deviceFakerCount) : '--'
)
const templateCountDisplay = computed(() =>
  configReady.value ? String(configStore.templateCount) : '--'
)
const workMode = computed(() => {
  if (!configReady.value) {
    return '--'
  }

  const mode = configStore.config.default_mode || 'lite'
  if (mode === 'lite') return t('status.mode.lite')
  if (mode === 'full') return t('status.mode.full')
  return t('status.mode.companion')
})

async function handleToggleWorkMode() {
  if (!configReady.value) {
    return
  }

  await configStore.toggleWorkMode()
}

// KeepAlive 激活时的钩子
onActivated(() => {
  // 页面激活
})
</script>

<style scoped>
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

.status-page {
  display: flex;
  flex-direction: column;
  gap: 1rem;
  width: 100%;
  max-width: 100%;
  box-sizing: border-box;
  /* 确保宽度稳定，不受滚动条影响 */
  overflow: hidden;
}

.status-card {
  padding: 1.5rem;
  border-radius: 1rem;
  box-shadow: 0 4px 12px var(--shadow);
  position: relative;
  overflow: hidden;
}

.status-card::before {
  content: '';
  position: absolute;
  top: 0;
  left: 0;
  right: 0;
  bottom: 0;
  background: linear-gradient(135deg, rgba(14, 165, 233, 0.03) 0%, rgba(168, 85, 247, 0.03) 100%);
  pointer-events: none;
}

.card-title {
  font-size: 1.25rem;
  font-weight: 600;
  margin-bottom: 1.5rem;
  color: var(--text);
  position: relative;
}

.status-grid {
  display: grid;
  grid-template-columns: 1fr;
  gap: 1rem;
}

.status-item {
  display: flex;
  align-items: center;
  gap: 1rem;
  padding: 1rem;
  background: var(--background);
  border-radius: 0.75rem;
  transition: all 0.15s ease;
  -webkit-tap-highlight-color: transparent;
}

.status-item.clickable {
  user-select: none;
  -webkit-user-select: none;
}

.status-item.disabled {
  opacity: 0.65;
}

.status-item.clickable:active {
  background: linear-gradient(135deg, rgba(14, 165, 233, 0.1) 0%, rgba(168, 85, 247, 0.1) 100%);
  transform: scale(0.98);
  opacity: 0.9;
}

.status-icon {
  display: flex;
  align-items: center;
  justify-content: center;
  width: 48px;
  height: 48px;
  border-radius: 12px;
  position: relative;
  flex-shrink: 0;
}

.gradient-icon-1 {
  background: linear-gradient(135deg, #0ea5e9 0%, #38bdf8 100%);
  color: white;
  box-shadow: 0 4px 12px rgba(14, 165, 233, 0.3);
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

.gradient-icon-5 {
  background: linear-gradient(135deg, #f97316 0%, #fb7185 100%);
  color: white;
  box-shadow: 0 4px 12px rgba(249, 115, 22, 0.3);
}

.status-info {
  display: flex;
  flex-direction: column;
  min-width: 0;
}

.status-label {
  font-size: 0.875rem;
  color: var(--text-secondary);
}

.status-value {
  font-size: 1.25rem;
  font-weight: 600;
  color: var(--text);
}

.status-value--resolved {
  display: inline-flex;
  align-items: center;
}

.status-build {
  font-size: 0.75rem;
  color: var(--text-secondary);
  opacity: 0.7;
  font-family: monospace;
}

.status-build-placeholder {
  display: inline-flex;
  height: 0.75rem;
}

.status-transition-slot {
  display: inline-grid;
  align-items: center;
  justify-items: start;
  min-height: 1.75rem;
}

.status-transition-slot > * {
  grid-area: 1 / 1;
}

.status-transition-slot--build {
  min-height: 0.75rem;
  margin-top: 0.25rem;
}

.status-value-skeleton,
.status-build-skeleton {
  display: inline-flex;
  border-radius: 999px;
  background: linear-gradient(90deg, var(--border) 25%, var(--card-bg) 50%, var(--border) 75%);
  background-size: 200% 100%;
  animation: status-skeleton-shimmer 1.3s linear infinite;
  opacity: 0.8;
}

.status-value-skeleton {
  height: 1.35rem;
  margin-top: 0.15rem;
}

.status-value-skeleton--short {
  width: 2.75rem;
}

.status-value-skeleton--medium {
  width: 5.5rem;
}

.status-value-skeleton--wide {
  width: 7.5rem;
}

.status-build-skeleton {
  width: 4.25rem;
  height: 0.75rem;
  margin-top: 0.25rem;
}

@keyframes status-skeleton-shimmer {
  from {
    background-position: -200% 0;
  }

  to {
    background-position: 200% 0;
  }
}

.status-swap-enter-active,
.status-swap-leave-active {
  transition:
    opacity 0.22s ease,
    transform 0.22s ease;
}

.status-swap-enter-from,
.status-swap-leave-to {
  opacity: 0;
  transform: translateY(6px);
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
