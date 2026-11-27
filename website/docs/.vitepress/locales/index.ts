// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

import { defineConfig } from 'vitepress'
import zh from './zh.ts'
import en from './en.ts'

export default defineConfig({
  locales: {
    root: {
      label: '简体中文',
      lang: 'zh-CN',
      description: '基于 Zygisk 的机型伪装模块，可以为不同的应用配置不同的设备型号。',
      themeConfig: zh.themeConfig
    },
    en: {
      label: 'English',
      lang: 'en-US',
      description:
        'A Zygisk-based device model spoofing module that configures different device models for different apps.',
      themeConfig: en.themeConfig
    }
  }
})
