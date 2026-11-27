// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

import { defineConfig } from 'vitepress'

export default defineConfig({
  lang: 'zh-CN',

  themeConfig: {
    nav: nav(),

    sidebar: {
      '/guide/': sidebarGuide()
    },

    outlineTitle: '本页目录',
    lastUpdatedText: '最后更新于',
    docFooter: { prev: '上一页', next: '下一页' },
    langMenuLabel: '切换语言',
    returnToTopLabel: '回到顶部',
    sidebarMenuLabel: '菜单',
    darkModeSwitchLabel: '外观',
    lightModeSwitchTitle: '切换到浅色模式',
    darkModeSwitchTitle: '切换到深色模式',

    socialLinks: [
      { icon: 'github', link: 'https://github.com/Seyud/device_faker' },
      { icon: 'telegram', link: 'https://t.me/device_faker' }
    ],

    footer: {
      message: '基于 GPL-3.0 许可证发布。',
      copyright: 'Copyright © 2025-2026-present 酷安@瓦力喀 / GitHub@Seyud'
    },

    editLink: {
      pattern: 'https://github.com/Seyud/device_faker/edit/main/website/docs/:path',
      text: '在 GitHub 上编辑此页'
    },

    search: {
      provider: 'local',
      options: {
        detailedView: true,
        translations: {
          button: {
            buttonText: '搜索',
            buttonAriaLabel: '搜索文档'
          },
          modal: {
            displayDetails: '显示详细列表',
            resetButtonTitle: '清除查询条件',
            backButtonTitle: '返回',
            noResultsText: '未找到相关结果',
            footer: {
              selectText: '选择',
              navigateText: '切换',
              closeText: '关闭'
            }
          }
        }
      }
    }
  }
})

function nav() {
  return [
    { text: '指南', link: '/guide/', activeMatch: '/guide/(installation|webui|$)' },
    { text: '配置', link: '/guide/configuration/', activeMatch: '/guide/configuration/' },
    { text: '更新日志', link: '/guide/changelog' },
    {
      text: '社区',
      items: [
        { text: '💬 Telegram 群组', link: 'https://t.me/device_faker' },
        {
          text: '🐧 QQ 群：854188252',
          link: 'https://qun.qq.com/universal-share/share?ac=1&authKey=ls4nlfcsF%2Bxp5SPnVsXRgpbeV1axPZb%2FmJCMXms6ZCHjgAwvOyl1LV%2BDNVL1btgL&busi_data=eyJncm91cENvZGUiOiI4NTQxODgyNTIiLCJ0b2tlbiI6IlE1WVVyZTZxUXVjZUtGUUxWSGFmbzkvMEd3UWNRSiszdklTZDhHejU0RDRyT0lWRTFqS3d4UGJSM1ltaXpkS3MiLCJ1aW4iOiIxMTA1NzgzMDMzIn0%3D&data=IbvhTKt9HwCSsCsl_610-rQ8p6H2NgLmxhEKkMcn-BMWPb86jygWBZJfWLQGm7J8LwpVV2yhPafxTMXYGkjRVA&svctype=4&tempid=h5_group_info'
        },
        { text: '📦 模板贡献仓库', link: 'https://github.com/Seyud/device_faker_config' }
      ]
    },
    { text: '下载', link: 'https://github.com/Seyud/device_faker/releases/latest' }
  ]
}

function sidebarGuide() {
  return [
    {
      text: '开始',
      items: [
        { text: '项目简介', link: '/guide/' },
        { text: '安装', link: '/guide/installation' },
        { text: 'WebUI 管理', link: '/guide/webui' }
      ]
    },
    {
      text: '配置',
      items: [
        { text: '基础配置', link: '/guide/configuration/' },
        { text: '字段参考', link: '/guide/configuration/fields' },
        { text: '高级用法', link: '/guide/configuration/advanced' }
      ]
    },
    {
      text: '帮助',
      items: [
        { text: '常见问题', link: '/guide/faq' },
        { text: '更新日志', link: '/guide/changelog' }
      ]
    }
  ]
}
