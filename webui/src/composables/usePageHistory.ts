// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

import { useNavigationStore } from '../stores/navigation'
import type { PageId } from '../stores/navigation'
import { PAGE_ORDER } from './usePageNavigation'

export function usePageHistory() {
  const navigationStore = useNavigationStore()

  let popstateHandler: ((event: Event) => void) | null = null
  let handlingPopstate = false
  let setActivePage: ((pageId: PageId, options?: { skipHistory?: boolean }) => void) | null = null

  function bindSetActivePage(fn: typeof setActivePage) {
    setActivePage = fn
  }

  function setupHistory() {
    navigationStore.replacePageHistory('home')

    popstateHandler = (_event: Event) => {
      if (navigationStore.shouldIgnorePopstate()) return
      handlingPopstate = true

      if (navigationStore.hasOpenModals()) {
        navigationStore.dismissTopModal()
        handlingPopstate = false
        return
      }

      /*
       * 返回键落点由浏览器条目状态决定（见 stores/navigation.ts 的 syncPageHistory）：
       * 非 home 页有一条附加记录，返回即回到 home；home 已是基底记录，
       * 再返回时 canGoBack() 已为 false，由宿主 finish 掉 WebUIActivity。
       */
      const state = window.history?.state as { dfPage?: PageId } | null
      const targetPage = state?.dfPage && PAGE_ORDER.includes(state.dfPage) ? state.dfPage : 'home'
      navigationStore.applyPopstatePage(targetPage)
      setActivePage?.(targetPage, { skipHistory: true })
      handlingPopstate = false
    }

    window.addEventListener('popstate', popstateHandler)
  }

  function isHandlingPopstate() {
    return handlingPopstate
  }

  function cleanup() {
    if (popstateHandler) {
      window.removeEventListener('popstate', popstateHandler)
      popstateHandler = null
    }
  }

  return { setupHistory, isHandlingPopstate, bindSetActivePage, cleanup }
}
