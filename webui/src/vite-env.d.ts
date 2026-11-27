// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

/// <reference types="vite/client" />

declare module '*.vue' {
  import type { DefineComponent } from 'vue'
  // eslint-disable-next-line @typescript-eslint/no-explicit-any
  const component: DefineComponent<Record<string, never>, Record<string, never>, any>
  export default component
}

// element-plus >= 2.14.0 移除了 style 子路径的 .d.ts（上游变更），
// 为按需样式导入（message / message-box 等 style/css）补充模块声明
declare module 'element-plus/es/components/*/style/css'

interface ImportMetaEnv {
  readonly DEV: boolean
  readonly PROD: boolean
  readonly MODE: string
  readonly VITE_DEBUG?: string
}

interface ImportMeta {
  readonly env: ImportMetaEnv
}

// 扩展 Window 接口
interface Window {
  ksu?: unknown
}

interface ObjectConstructor {
  hasOwn(o: object, v: PropertyKey): boolean
}
