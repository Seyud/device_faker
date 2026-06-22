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

      if (navigationStore.hasPageHistory()) {
        navigationStore.jumpToHome()
        const state = window.history?.state as { dfPage?: PageId } | null
        const restored = state?.dfPage
        if (restored && PAGE_ORDER.includes(restored)) {
          setActivePage?.(restored, { skipHistory: true })
        }
        handlingPopstate = false
        return
      }

      const state = window.history?.state as { dfPage?: PageId; modal?: boolean } | null
      const targetPage = state?.dfPage
      if (targetPage && PAGE_ORDER.includes(targetPage)) {
        setActivePage?.(targetPage, { skipHistory: true })
      }
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
