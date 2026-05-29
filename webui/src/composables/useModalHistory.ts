import { watch, onUnmounted, type Ref } from 'vue'
import { useNavigationStore } from '../stores/navigation'

export function useModalHistory(visible: Ref<boolean>, close: () => void) {
  const nav = useNavigationStore()
  let modalId: symbol | null = null

  watch(visible, (isOpen) => {
    if (isOpen) {
      modalId = nav.registerModal(close)
      nav.pushModalHistory()
    } else if (modalId !== null) {
      // If close was triggered by the popstate handler (programmatic close),
      // the handler already manages history.back() — don't call it again.
      const programmatic = nav.consumeProgrammaticClose()
      nav.unregisterModal(modalId)
      modalId = null
      if (!programmatic) {
        nav.popModalHistory()
      }
    }
  })

  onUnmounted(() => {
    if (modalId !== null) {
      nav.unregisterModal(modalId)
      modalId = null
    }
  })
}
