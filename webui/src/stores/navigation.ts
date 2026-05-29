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

  function pushPageToStack(page: PageId) {
    pageHistoryStack.value = [...pageHistoryStack.value, page]
    if (window.history?.pushState) {
      window.history.pushState({ dfPage: page }, '', `#/${page}`)
    }
  }

  /** Returns true if there are pages to go back to (stack has more than home). */
  function hasPageHistory(): boolean {
    return pageHistoryStack.value.length > 1
  }

  /** Pop all pages off the stack back to home, then history.go(-n). */
  function jumpToHome() {
    const stack = pageHistoryStack.value
    const stepsBack = stack.length - 1
    pageHistoryStack.value = [stack[0]]

    if (stepsBack > 0 && window.history?.go) {
      window.history.go(-stepsBack)
    }
  }

  // -- Modal history management --

  function replacePageHistory(page: PageId) {
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
    pushPageToStack,
    hasPageHistory,
    jumpToHome,
    replacePageHistory,
    pushModalHistory,
    popModalHistory,
    shouldIgnorePopstate,
    setIgnoreNextPopstate,
  }
})
