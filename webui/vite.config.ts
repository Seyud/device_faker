// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import { resolve } from 'path'
import AutoImport from 'unplugin-auto-import/vite'
import Components from 'unplugin-vue-components/vite'
import { ElementPlusResolver } from 'unplugin-vue-components/resolvers'

export default defineConfig({
  plugins: [
    vue(),
    AutoImport({
      resolvers: [ElementPlusResolver()],
    }),
    Components({
      resolvers: [ElementPlusResolver()],
    }),
  ],
  resolve: {
    alias: {
      '@': resolve(import.meta.dirname, 'src'),
    },
  },
  build: {
    outDir: '../module/webroot',
    emptyOutDir: true, // 清空目录，避免旧文件堆积
    assetsDir: 'assets',
    rollupOptions: {
      checks: {
        // unplugin-vue-components 的 transformInclude 过滤在 JS 包装 hook 内执行，
        // 会对每个模块（含 node_modules）调用一次，异步耗时与 vite:css 等并发
        // 工作重叠，导致 pluginTimings 归因失真（误报 56% 耗时）。关闭该诊断。
        pluginTimings: false,
      },
    },
  },
  server: {
    port: 3000,
    host: true,
  },
})
