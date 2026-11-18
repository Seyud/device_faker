// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

import { defineConfig } from 'vitepress'

export default defineConfig({
  lang: 'en-US',

  themeConfig: {
    nav: nav(),

    sidebar: {
      '/en/guide/': sidebarGuide()
    },

    outlineTitle: 'On this page',
    lastUpdatedText: 'Last updated',
    docFooter: { prev: 'Previous page', next: 'Next page' },
    langMenuLabel: 'Change language',
    returnToTopLabel: 'Return to top',
    sidebarMenuLabel: 'Menu',
    darkModeSwitchLabel: 'Appearance',
    lightModeSwitchTitle: 'Switch to light theme',
    darkModeSwitchTitle: 'Switch to dark theme',

    socialLinks: [
      { icon: 'github', link: 'https://github.com/Seyud/device_faker' },
      { icon: 'telegram', link: 'https://t.me/device_faker' }
    ],

    footer: {
      message: 'Released under the GPL-3.0 License.',
      copyright: 'Copyright © 2025-2026-present 酷安@瓦力喀 / GitHub@Seyud'
    },

    editLink: {
      pattern: 'https://github.com/Seyud/device_faker/edit/main/website/docs/:path',
      text: 'Edit this page on GitHub'
    },

    search: {
      provider: 'local',
      options: {
        detailedView: true
      }
    }
  }
})

function nav() {
  return [
    { text: 'Guide', link: '/en/guide/', activeMatch: '/en/guide/(installation|webui|$)' },
    { text: 'Configuration', link: '/en/guide/configuration/', activeMatch: '/en/guide/configuration/' },
    { text: 'Changelog', link: '/en/guide/changelog' },
    {
      text: 'Community',
      items: [
        { text: '💬 Telegram Group', link: 'https://t.me/device_faker' },
        { text: '📦 Template Repository', link: 'https://github.com/Seyud/device_faker_config' }
      ]
    },
    { text: 'Downloads', link: 'https://github.com/Seyud/device_faker/releases/latest' }
  ]
}

function sidebarGuide() {
  return [
    {
      text: 'Getting Started',
      items: [
        { text: 'Introduction', link: '/en/guide/' },
        { text: 'Installation', link: '/en/guide/installation' },
        { text: 'WebUI', link: '/en/guide/webui' }
      ]
    },
    {
      text: 'Configuration',
      items: [
        { text: 'Basics', link: '/en/guide/configuration/' },
        { text: 'Field Reference', link: '/en/guide/configuration/fields' },
        { text: 'Advanced Usage', link: '/en/guide/configuration/advanced' }
      ]
    },
    {
      text: 'Help',
      items: [
        { text: 'FAQ', link: '/en/guide/faq' },
        { text: 'Changelog', link: '/en/guide/changelog' }
      ]
    }
  ]
}
