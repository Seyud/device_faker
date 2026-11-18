// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

import { readFileSync } from 'node:fs'
import { fileURLToPath } from 'node:url'
import { defineConfig } from 'vitepress'
import locales from './locales/index.ts'

const BASE = '/device_faker/'
const SITE = 'https://seyud.github.io/device_faker/'

// Hero logo 内联为 data URI：图片随 HTML 一起到达，不再单独走一个被 GitHub Pages
// 限速的请求（线上实测 LCP P75 = 7.8s 就是耗在这张图上）。
// SVG 由 tmp/logo-svg/phone.py 从原 WebP 临摹生成（渐变背景 + 手机图形），5.7KB。
const LOGO_SVG = readFileSync(fileURLToPath(new URL('../public/logo.svg', import.meta.url)))
const LOGO_DATA_URI = `data:image/svg+xml;base64,${LOGO_SVG.toString('base64')}`

export default defineConfig({
  title: 'Device Faker',
  base: BASE,
  locales: locales.locales,
  lastUpdated: true,
  cleanUrls: false,

  sitemap: {
    hostname: SITE
  },

  // 首页 hero 图内联：构建时把 frontmatter 里的 /logo.webp 替换成 data URI，
  // img 与 HTML 同连接到达，消除单独请求（withBase 对 data: 原样放行，见 shared.js EXTERNAL_URL_RE）。
  transformPageData(pageData) {
    const fm: any = pageData.frontmatter
    const img = fm?.hero?.image
    if (fm?.layout === 'home' && img && typeof img === 'object' && typeof img.src === 'string' && img.src.includes('/logo.')) {
      img.src = LOGO_DATA_URI
    }
  },

  head: [
    // head 里的 URL 不会被自动补 base，必须手写前缀
    ['link', { rel: 'icon', type: 'image/png', href: `${BASE}favicon.png` }],
    ['link', { rel: 'apple-touch-icon', href: `${BASE}apple-touch-icon.png` }],
    ['meta', { name: 'theme-color', content: '#4f7cff' }],

    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:site_name', content: 'Device Faker' }],
    ['meta', { property: 'og:title', content: 'Device Faker — 基于 Zygisk 的机型伪装模块' }],
    ['meta', { property: 'og:description', content: '为每个应用配置不同的设备型号，基于 Zygisk 的机型伪装模块。' }],
    ['meta', { property: 'og:image', content: `${SITE}og-image.png` }],
    ['meta', { name: 'twitter:card', content: 'summary_large_image' }],

    ['link', { rel: 'alternate', hreflang: 'zh-CN', href: `${SITE}` }],
    ['link', { rel: 'alternate', hreflang: 'en-US', href: `${SITE}en/` }],
    ['link', { rel: 'alternate', hreflang: 'x-default', href: `${SITE}` }],

    // Cloudflare Web Analytics
    [
      'script',
      {
        type: 'module',
        src: 'https://static.cloudflareinsights.com/beacon.min.js',
        'data-cf-beacon': JSON.stringify({ token: '7a60b306ee8f46f38f2699681b26453e' })
      }
    ],
    // 语言分流：有记忆时按记忆语言主动跳转对应版本；无记忆时非中文浏览器自动跳英文
    [
      'script',
      {},
      `            ;(() => {
        const base = '/device_faker/'
        const KEY = 'lang-pref:/device_faker/'
        const LAST = 'lang-last:/device_faker/'
        const FLAG = 'lang-redir:/device_faker/'
        const path = location.pathname
        if (!path.startsWith(base)) return
        const sub = path.slice(base.length)
        const cur = sub.startsWith('en/') || sub === 'en'
        const auto = (navigator.language || '').toLowerCase().startsWith('zh') ? 'zh' : 'en'
        let pref = null
        try { pref = localStorage.getItem(KEY) } catch (e) {}
        try {
          // 跨语言落地检测：站内语言链接跳转而来（referrer 同源、非前进/后退）
          // 即使用户的点击事件被翻译类扩展拦截，原生导航仍会发生，此检测兜底
          const last = sessionStorage.getItem(LAST)
          if (sessionStorage.getItem(FLAG) !== null) {
            sessionStorage.removeItem(FLAG)
          } else if (last !== null && last !== (cur ? 'en' : 'zh')) {
            const nav = performance.getEntriesByType('navigation')[0]
            const ref = document.referrer
            const internal = !!ref && new URL(ref).origin === location.origin
            if (internal && (!nav || nav.type !== 'back_forward')) {
              const now = cur ? 'en' : 'zh'
              if (now === auto) { localStorage.removeItem(KEY); pref = null }
              else { localStorage.setItem(KEY, now); pref = now }
            }
          }
          sessionStorage.setItem(LAST, cur ? 'en' : 'zh')
        } catch (e) {}
        const go = u => {
          try { sessionStorage.setItem(FLAG, '1') } catch (e) {}
          location.replace(u)
        }
        if (pref === 'zh' || pref === 'en') {
          if (pref === 'en' && !cur) {
            go(base + 'en/' + sub + location.search + location.hash)
          } else if (pref === 'zh' && cur) {
            let rest = sub === 'en' ? '' : sub.slice(3)
            go(base + rest + location.search + location.hash)
          }
          return
        }
        if (!cur && auto !== 'zh') {
          go(base + 'en/' + sub + location.search + location.hash)
        }
      })()
      ;(() => {
        const base = '/device_faker/'
        const KEY = 'lang-pref:/device_faker/'
        const onPick = e => {
          const a = e.target && e.target.closest ? e.target.closest('a') : null
          if (!a) return
          const href = a.getAttribute('href') || ''
          let sub = null
          if (href.startsWith(base)) sub = href.slice(base.length)
          else {
            try {
              const u = new URL(href, location.origin)
              if (u.origin === location.origin && u.pathname.startsWith(base)) sub = u.pathname.slice(base.length)
            } catch (err) {}
          }
          if (sub === null) return
          const cur = location.pathname.slice(base.length)
          const t = sub.startsWith('en/') || sub === 'en'
          const c = cur.startsWith('en/') || cur === 'en'
          if (t !== c) {
            const lang = t ? 'en' : 'zh'
            const auto = (navigator.language || '').toLowerCase().startsWith('zh') ? 'zh' : 'en'
            try {
              if (lang === auto) localStorage.removeItem(KEY)
              else localStorage.setItem(KEY, lang)
            } catch (err) {}
          }
        }
        // window 捕获阶段注册早于一切内容脚本，先于扩展的事件拦截
        window.addEventListener('click', onPick, true)
        window.addEventListener('pointerdown', onPick, true)
      })()`
    ]
  ]
})
