// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

export const THEME_META_COLORS = {
  light: '#f2f9ff',
  dark: '#1a2538',
} as const

export function applyThemeToDocument(isDarkMode: boolean) {
  const root = document.documentElement

  root.classList.toggle('dark', isDarkMode)
  root.dataset.theme = isDarkMode ? 'dark' : 'light'
  root.style.colorScheme = isDarkMode ? 'dark' : 'light'

  document
    .getElementById('theme-color')
    ?.setAttribute('content', isDarkMode ? THEME_META_COLORS.dark : THEME_META_COLORS.light)
}
