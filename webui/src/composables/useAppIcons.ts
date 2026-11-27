// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

import { nextTick, ref } from 'vue'
import { parsePackageUser } from '../utils/package'

const ICON_CONTAINER_SELECTOR = '.app-icon-container'

type IconMap = Record<string, string>
type IconLoadedMap = Record<string, boolean>

export function useAppIcons() {
  const appIcons = ref<IconMap>({})
  const iconLoaded = ref<IconLoadedMap>({})
  const iconObserver = ref<IntersectionObserver | null>(null)

  const onIconLoad = (packageName: string) => {
    iconLoaded.value[packageName] = true
  }

  const onIconError = (packageName: string) => {
    appIcons.value[packageName] = 'fallback'
    iconLoaded.value[packageName] = true
  }

  const loadAppIcon = async (packageName: string) => {
    if (appIcons.value[packageName]) return

    const { base: normalizedPackage } = parsePackageUser(packageName)

    try {
      // KernelSU 原生图标协议（APK 设备级存在，用 base 名即可）
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      if (typeof (globalThis as any).ksu?.getPackagesInfo !== 'undefined') {
        appIcons.value[packageName] = `ksu://icon/${normalizedPackage}`
        return
      }

      appIcons.value[packageName] = 'fallback'
    } catch {
      appIcons.value[packageName] = 'fallback'
    }
  }

  const observeContainers = () => {
    nextTick(() => {
      const containers = document.querySelectorAll(ICON_CONTAINER_SELECTOR)
      containers.forEach((container) => {
        iconObserver.value?.observe(container)
      })
    })
  }

  const setupIconObserver = () => {
    if (iconObserver.value) {
      iconObserver.value.disconnect()
    }

    iconObserver.value = new IntersectionObserver(
      (entries) => {
        entries.forEach((entry) => {
          if (entry.isIntersecting) {
            const container = entry.target as HTMLElement
            const packageName = container.dataset.package
            if (packageName) {
              loadAppIcon(packageName)
              iconObserver.value?.unobserve(container)
            }
          }
        })
      },
      {
        rootMargin: '100px',
        threshold: 0.1,
      }
    )

    observeContainers()
  }

  const teardownIconObserver = () => {
    if (iconObserver.value) {
      iconObserver.value.disconnect()
      iconObserver.value = null
    }
  }

  return {
    appIcons,
    iconLoaded,
    loadAppIcon,
    onIconLoad,
    onIconError,
    setupIconObserver,
    teardownIconObserver,
  }
}
