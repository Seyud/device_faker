<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  Copyright (c) 2025-2026 Seyud
  Device Faker: https://github.com/Seyud/device_faker
-->

<template>
  <div :class="['base-select', { 'is-open': open }]">
    <button
      ref="triggerRef"
      type="button"
      class="base-select__trigger"
      :aria-label="ariaLabel"
      aria-haspopup="listbox"
      :aria-expanded="open ? 'true' : 'false'"
      @click.stop="toggle"
    >
      <span class="base-select__value">{{ currentLabel }}</span>
      <ChevronDown :size="14" :class="['base-select__arrow', { 'is-open': open }]" />
    </button>

    <!-- 弹层 teleport 到 body：规避 glass-effect(backdrop-filter) 等祖先的
         stacking context 裁剪与 page-track(will-change:transform) 对 fixed 的劫持 -->
    <Teleport to="body">
      <div v-if="open" class="base-select__backdrop" @click="close" />
      <transition name="base-select-pop">
        <ul v-if="open" class="base-select__menu" role="listbox" :style="menuStyle">
          <li
            v-for="opt in options"
            :key="String(opt.value)"
            role="option"
            :aria-selected="opt.value === modelValue"
            :class="['base-select__option', { 'is-selected': opt.value === modelValue }]"
            @click.stop="select(opt.value)"
          >
            <span class="base-select__option-label">{{ opt.label }}</span>
            <Check v-if="opt.value === modelValue" :size="16" />
          </li>
          <li v-if="!options.length" class="base-select__empty">--</li>
        </ul>
      </transition>
    </Teleport>
  </div>
</template>

<script setup lang="ts">
import { computed, onBeforeUnmount, onMounted, ref } from 'vue'
import { Check, ChevronDown } from '@lucide/vue'

interface SelectOption {
  label: string
  value: string | number
}

const props = defineProps<{
  modelValue: string | number
  options: SelectOption[]
  ariaLabel?: string
}>()

const emit = defineEmits<{
  'update:modelValue': [string | number]
  change: [string | number]
}>()

/** 触发器下方剩余空间小于该值时向上翻转 */
const FLIP_SPACE_PX = 268

const triggerRef = ref<HTMLElement | null>(null)
const open = ref(false)
const menuStyle = ref<Record<string, string>>({})

const currentLabel = computed(
  () => props.options.find((o) => o.value === props.modelValue)?.label ?? String(props.modelValue)
)

function show() {
  const rect = triggerRef.value?.getBoundingClientRect()
  if (rect) {
    const base: Record<string, string> = {
      left: `${rect.left}px`,
      width: `${rect.width}px`,
    }
    if (window.innerHeight - rect.bottom < FLIP_SPACE_PX) {
      base.bottom = `${window.innerHeight - rect.top + 4}px`
    } else {
      base.top = `${rect.bottom + 4}px`
    }
    menuStyle.value = base
  }
  open.value = true
}

function toggle() {
  if (open.value) {
    close()
  } else {
    show()
  }
}

function close() {
  open.value = false
}

function select(value: string | number) {
  if (value !== props.modelValue) {
    emit('update:modelValue', value)
    emit('change', value)
  }
  close()
}

function onKeydown(event: KeyboardEvent) {
  if (open.value && event.key === 'Escape') close()
}

function onViewportChange() {
  // 滚动/旋转/缩放时不追踪位置（与常见移动端 popper 一致），直接收起
  if (open.value) close()
}

onMounted(() => {
  document.addEventListener('keydown', onKeydown)
  window.addEventListener('scroll', onViewportChange, true)
  window.addEventListener('resize', onViewportChange)
})

onBeforeUnmount(() => {
  document.removeEventListener('keydown', onKeydown)
  window.removeEventListener('scroll', onViewportChange, true)
  window.removeEventListener('resize', onViewportChange)
})
</script>

<style scoped>
.base-select {
  position: relative;
  width: 100%;
}

/* 触发器：对齐原 el-select 的输入框观感 */
.base-select__trigger {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  width: 100%;
  height: 2rem;
  padding: 0 0.625rem;
  font-size: 0.875rem;
  color: var(--text);
  text-align: left;
  background-color: var(--card-bg);
  border: 1px solid var(--border);
  border-radius: 0.5rem;
  cursor: pointer;
  transition:
    border-color 0.2s ease,
    box-shadow 0.2s ease;
}

.base-select__value {
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.base-select.is-open .base-select__trigger,
.base-select__trigger:focus-visible {
  outline: none;
  border-color: var(--primary);
}

.base-select.is-open .base-select__trigger {
  box-shadow: 0 0 0 1px var(--primary) inset;
}

.base-select__arrow {
  flex-shrink: 0;
  color: var(--text-secondary);
  transition: transform 0.2s ease;
}

.base-select__arrow.is-open {
  transform: rotate(180deg);
}

/* 以下弹层节点被 teleport 到 body（脱离组件 DOM 层级，但 scoped 属性仍生效） */
.base-select__backdrop {
  position: fixed;
  inset: 0;
  z-index: 2499;
  background: transparent;
}

.base-select__menu {
  position: fixed;
  z-index: 2500;
  margin: 0;
  padding: 0.25rem;
  list-style: none;
  max-height: 15rem;
  overflow-y: auto;
  overscroll-behavior: contain;
  background-color: var(--card-bg);
  border: 1px solid var(--border);
  border-radius: 0.5rem;
  box-shadow: 0 8px 24px rgba(0, 0, 0, 0.18);
}

.dark .base-select__menu,
.dark .base-select__trigger {
  background-color: var(--el-fill-color-blank);
  border-color: var(--el-border-color);
}

.dark .base-select__menu {
  box-shadow: 0 8px 32px rgba(0, 0, 0, 0.55);
}

.base-select__option {
  display: flex;
  align-items: center;
  justify-content: space-between;
  gap: 0.5rem;
  min-height: 2.25rem;
  padding: 0.375rem 0.75rem;
  font-size: 0.875rem;
  color: var(--text-secondary);
  border-radius: 0.375rem;
  cursor: pointer;
  user-select: none;
  -webkit-user-select: none;
  transition:
    background-color 0.15s ease,
    color 0.15s ease;
}

@media (hover: hover) and (pointer: fine) {
  .base-select__option:hover {
    color: var(--text);
    background-color: rgba(148, 163, 184, 0.12);
  }
}

.dark .base-select__option:hover {
  color: var(--el-text-color-primary);
  background-color: var(--el-fill-color);
}

.base-select__option.is-selected,
.base-select__option.is-selected svg {
  font-weight: 500;
  color: var(--primary);
}

.base-select__option-label {
  overflow: hidden;
  white-space: nowrap;
  text-overflow: ellipsis;
}

.base-select__empty {
  padding: 0.5rem 0.75rem;
  font-size: 0.875rem;
  color: var(--text-secondary);
  text-align: center;
}

/* popper 过渡 */
.base-select-pop-enter-active,
.base-select-pop-leave-active {
  transition:
    opacity 0.15s ease,
    transform 0.15s ease;
  will-change: opacity, transform;
}

.base-select-pop-enter-from,
.base-select-pop-leave-to {
  opacity: 0;
  transform: translateY(-6px) scale(0.98);
}
</style>
