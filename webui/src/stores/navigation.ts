// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

import { defineStore } from 'pinia'
import { ref, readonly } from 'vue'

export type PageId = 'home' | 'templates' | 'apps' | 'settings'

interface ModalEntry {
  id: symbol
  close: () => void
}

export const useNavigationStore = defineStore('navigation', () => {
  const modalStack = ref<ModalEntry[]>([])
  const currentPage = ref<PageId>('home')
  const pageHistoryStack = ref<PageId[]>([])
  let ignoreNextPopstate = false
  let wasProgrammaticClose = false

  // -- Modal stack management --

  function registerModal(close: () => void): symbol {
    const id = Symbol('modal')
    modalStack.value = [...modalStack.value, { id, close }]
    return id
  }

  function unregisterModal(id: symbol) {
    modalStack.value = modalStack.value.filter((e) => e.id !== id)
  }

  function hasOpenModals(): boolean {
    return modalStack.value.length > 0
  }

  function topModal(): ModalEntry | undefined {
    return modalStack.value[modalStack.value.length - 1]
  }

  function dismissTopModal(): boolean {
    const stack = modalStack.value
    if (stack.length === 0) return false
    const top = stack[stack.length - 1]
    wasProgrammaticClose = true
    top.close()
    return true
  }

  function consumeProgrammaticClose(): boolean {
    if (wasProgrammaticClose) {
      wasProgrammaticClose = false
      return true
    }
    return false
  }

  // -- Page history stack --

  function setCurrentPage(page: PageId) {
    currentPage.value = page
  }

  function hasPageEntry() {
    return pageHistoryStack.value.length > 1
  }

  /**
   * 页面历史只有两级：home 是**基底条目**，所有非 home 页共用**一个**附加条目。
   *
   * 旧实现每次切页都 `pushState`，历史里会堆满页记录：于是「在主页按返回」
   * 只是回放上一条页记录（用户看到的是「又切到另一页」），怎么按都退不出 WebUI。
   * 现在：
   * - 离开 home：push 一条附加记录，之后页间切换一律 `replaceState` 复用它；
   * - 回到 home：`history.back()` 把这条附加记录吃掉（popstate 自己发起的，忽略它的回放）。
   * ⇒ 停在主页时 `canGoBack() === false`，宿主 KernelSU 的 BackHandler 被禁用，
   *   系统返回键直接 finish 掉 WebUIActivity（退出 WebUI）。
   */
  function syncPageHistory(page: PageId) {
    if (page === 'home') {
      const hadEntry = hasPageEntry()
      pageHistoryStack.value = ['home']
      if (hadEntry && window.history?.back) {
        ignoreNextPopstate = true
        window.history.back()
      }
      return
    }

    const hadEntry = hasPageEntry()
    pageHistoryStack.value = ['home', page]
    if (hadEntry) {
      window.history?.replaceState?.({ dfPage: page }, '', `#/${page}`)
    } else {
      window.history?.pushState?.({ dfPage: page }, '', `#/${page}`)
    }
  }

  /** 浏览器返回键落地后按条目重写栈（不再动 history，条目已经被浏览器弹掉了） */
  function applyPopstatePage(page: PageId) {
    pageHistoryStack.value = page === 'home' ? ['home'] : ['home', page]
  }

  // -- Modal history management --

  function replacePageHistory(page: PageId) {
    pageHistoryStack.value = [page]
    if (!window.history?.replaceState) return
    window.history.replaceState({ dfPage: page }, '', `#/${page}`)
  }

  function pushModalHistory() {
    if (!window.history?.pushState) return
    window.history.pushState(
      { dfPage: currentPage.value, modal: true },
      '',
      `#/${currentPage.value}/modal`
    )
  }

  function popModalHistory() {
    if (!window.history?.back) return
    ignoreNextPopstate = true
    window.history.back()
  }

  // -- Popstate control --

  function shouldIgnorePopstate(): boolean {
    if (ignoreNextPopstate) {
      ignoreNextPopstate = false
      return true
    }
    return false
  }

  function setIgnoreNextPopstate() {
    ignoreNextPopstate = true
  }

  return {
    modalStack: readonly(modalStack),
    currentPage: readonly(currentPage),
    pageHistoryStack: readonly(pageHistoryStack),
    registerModal,
    unregisterModal,
    hasOpenModals,
    topModal,
    dismissTopModal,
    consumeProgrammaticClose,
    setCurrentPage,
    syncPageHistory,
    applyPopstatePage,
    replacePageHistory,
    pushModalHistory,
    popModalHistory,
    shouldIgnorePopstate,
    setIgnoreNextPopstate,
  }
})
