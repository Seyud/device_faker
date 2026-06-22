import { ref, type Ref } from 'vue'
import type { PageId } from '../stores/navigation'
import { PAGE_ORDER } from './usePageNavigation'

type SwipeIntent = 'horizontal' | 'vertical' | null

const SWIPE_LOCK_DISTANCE_PX = 8
const SWIPE_LOCK_AXIS_RATIO = 1.2
const SWIPE_DISTANCE_RATIO = 0.12
const SWIPE_VELOCITY_THRESHOLD = 0.5
const EDGE_RESISTANCE = 0.35
const CLICK_SUPPRESS_WINDOW_MS = 320
const SETTLE_DURATION_MS = 280
const SETTLE_EASING = 'cubic-bezier(0.25, 1, 0.5, 1)'

const SWIPE_IGNORE_SELECTOR = [
  '[data-page-swipe-ignore]',
  'a',
  'button',
  'input',
  'textarea',
  'select',
  'label',
  '[role="button"]',
  '.el-button',
  '.el-input',
  '.el-input__wrapper',
  '.el-input__inner',
  '.el-textarea',
  '.el-select',
  '.el-switch',
  '.el-radio',
  '.el-checkbox',
  '.el-slider',
  '.el-dialog',
  '.el-overlay',
  '.el-popper',
  '.el-picker-panel',
  '.el-message-box',
].join(', ')

function normalizeTargetElement(target: globalThis.EventTarget | null) {
  if (target instanceof HTMLElement) return target
  if (target instanceof window.Node) return target.parentElement
  return null
}

function hasHorizontalScrollableAncestor(target: HTMLElement | null, boundary: HTMLElement | null) {
  let current = target
  while (current && current !== boundary) {
    const style = window.getComputedStyle(current)
    const overflowX = style.overflowX
    if (
      (overflowX === 'auto' || overflowX === 'scroll' || overflowX === 'overlay') &&
      current.scrollWidth > current.clientWidth + 4
    ) {
      return true
    }
    current = current.parentElement
  }
  return false
}

export function hasVerticalScrollableAncestor(target: HTMLElement | null, boundary: HTMLElement) {
  let current = target
  while (current && current !== boundary) {
    const style = window.getComputedStyle(current)
    const overflowY = style.overflowY
    if (
      (overflowY === 'auto' || overflowY === 'scroll' || overflowY === 'overlay') &&
      current.scrollHeight > current.clientHeight + 4
    ) {
      return true
    }
    current = current.parentElement
  }
  return false
}

export function usePageSwipe(opts: {
  activePage: Ref<PageId>
  pageStageWidth: Ref<number>
  pageStageRef: Ref<HTMLElement | null>
  primeNeighborPages: (index: number) => void
  setActivePageSilent: (pageId: PageId) => void
}) {
  const { activePage, pageStageWidth, pageStageRef, primeNeighborPages, setActivePageSilent } = opts

  const isSwipeDragging = ref(false)
  const activePointerId = ref<number | null>(null)

  let targetIndex = 0
  let pendingSettleCleanup: (() => void) | null = null
  let pointerStartX = 0
  let pointerStartY = 0
  let pointerStartTime = 0
  let pointerIntent: SwipeIntent = null
  let suppressClickUntil = 0
  let dragLastX = 0
  let dragLastTime = 0

  function getActiveIndex() {
    return PAGE_ORDER.indexOf(activePage.value)
  }

  function getTrack(): HTMLElement | null {
    const stage = pageStageRef.value
    if (!stage) return null
    return stage.querySelector('.page-track') as HTMLElement | null
  }

  function applyDragTransform(offsetPx: number) {
    const track = getTrack()
    if (!track) return
    const translateX = -(targetIndex * pageStageWidth.value) + offsetPx
    track.style.transform = `translate3d(${translateX}px, 0, 0)`
  }

  function shouldIgnoreSwipeTarget(target: globalThis.EventTarget | null) {
    const element = normalizeTargetElement(target)
    if (!element) return false
    if (element.closest(SWIPE_IGNORE_SELECTOR)) return true
    return hasHorizontalScrollableAncestor(element, pageStageRef.value)
  }

  function clampSwipeOffset(offsetPx: number) {
    const maxOffset = pageStageWidth.value
    let nextOffset = Math.max(Math.min(offsetPx, maxOffset), -maxOffset)
    if (targetIndex === 0 && nextOffset > 0) nextOffset *= EDGE_RESISTANCE
    if (targetIndex === PAGE_ORDER.length - 1 && nextOffset < 0) nextOffset *= EDGE_RESISTANCE
    return nextOffset
  }

  function releaseSwipeCapture(pointerId: number | null) {
    const el = pageStageRef.value
    if (pointerId !== null && el?.hasPointerCapture?.(pointerId)) {
      el.releasePointerCapture(pointerId)
    }
  }

  function resetSwipeTracking(pointerId: number | null = activePointerId.value) {
    releaseSwipeCapture(pointerId)
    activePointerId.value = null
    isSwipeDragging.value = false
    pointerIntent = null
  }

  function cancelPendingSettle() {
    const track = getTrack()
    if (track && pendingSettleCleanup) {
      pendingSettleCleanup()
      pendingSettleCleanup = null
    }
  }

  function settleTo(newIndex: number) {
    const track = getTrack()
    if (!track) return

    cancelPendingSettle()

    track.style.transition = `transform ${SETTLE_DURATION_MS}ms ${SETTLE_EASING}`
    track.style.transform = `translate3d(${-(newIndex * pageStageWidth.value)}px, 0, 0)`

    const onEnd = (e: TransitionEvent) => {
      if (e.propertyName !== 'transform') return
      pendingSettleCleanup = null
      track.style.transition = ''
    }
    track.addEventListener('transitionend', onEnd, { once: true })
    pendingSettleCleanup = () => {
      track.removeEventListener('transitionend', onEnd)
      track.style.transition = ''
    }

    // Sync state immediately — don't wait for transitionend
    const targetPage = PAGE_ORDER[newIndex]
    if (targetPage && targetPage !== activePage.value) {
      setActivePageSilent(targetPage)
    }
  }

  function handlePointerDown(event: globalThis.PointerEvent) {
    if (event.button !== 0 || activePointerId.value !== null) return
    if (shouldIgnoreSwipeTarget(event.target)) return

    // Abort any in-flight settle
    const track = getTrack()
    cancelPendingSettle()
    if (track) {
      const currentTransform = getComputedStyle(track).transform
      track.style.transition = ''
      if (currentTransform && currentTransform !== 'none') {
        track.style.transform = currentTransform
      }
    }

    targetIndex = getActiveIndex()
    activePointerId.value = event.pointerId
    pointerStartX = event.clientX
    pointerStartY = event.clientY
    pointerStartTime = event.timeStamp
    pointerIntent = null
    dragLastX = event.clientX
    dragLastTime = event.timeStamp
    isSwipeDragging.value = false
    primeNeighborPages(targetIndex)
  }

  function handlePointerMove(event: globalThis.PointerEvent) {
    if (activePointerId.value !== event.pointerId) return

    const deltaX = event.clientX - pointerStartX
    const deltaY = event.clientY - pointerStartY

    if (pointerIntent === null) {
      const absX = Math.abs(deltaX)
      const absY = Math.abs(deltaY)
      if (absX < SWIPE_LOCK_DISTANCE_PX && absY < SWIPE_LOCK_DISTANCE_PX) return

      if (absX > absY * SWIPE_LOCK_AXIS_RATIO) {
        pointerIntent = 'horizontal'
        isSwipeDragging.value = true
        pageStageRef.value?.setPointerCapture?.(event.pointerId)
      } else {
        pointerIntent = 'vertical'
        resetSwipeTracking(event.pointerId)
        return
      }
    }

    if (pointerIntent !== 'horizontal') return

    const now = window.performance.now()
    const elapsed = now - dragLastTime
    if (elapsed > 0) {
      // Track per-frame velocity for future use
      void ((event.clientX - dragLastX) / elapsed)
    }
    dragLastX = event.clientX
    dragLastTime = now
    applyDragTransform(clampSwipeOffset(deltaX))
    event.preventDefault()
  }

  function handlePointerEnd(event: globalThis.PointerEvent) {
    if (activePointerId.value !== event.pointerId) return

    const totalDeltaX = event.clientX - pointerStartX
    const elapsedMs = Math.max(event.timeStamp - pointerStartTime, 1)
    const velocityX = totalDeltaX / elapsedMs
    const swipeThresholdPx = pageStageWidth.value * SWIPE_DISTANCE_RATIO
    const currentIndex = getActiveIndex()
    let newIndex = currentIndex

    if (pointerIntent === 'horizontal') {
      if (
        (totalDeltaX <= -swipeThresholdPx || velocityX <= -SWIPE_VELOCITY_THRESHOLD) &&
        currentIndex < PAGE_ORDER.length - 1
      ) {
        newIndex = currentIndex + 1
      } else if (
        (totalDeltaX >= swipeThresholdPx || velocityX >= SWIPE_VELOCITY_THRESHOLD) &&
        currentIndex > 0
      ) {
        newIndex = currentIndex - 1
      }
      if (Math.abs(totalDeltaX) > 8) {
        suppressClickUntil = window.performance.now() + CLICK_SUPPRESS_WINDOW_MS
      }
    }

    resetSwipeTracking(event.pointerId)
    settleTo(newIndex)
  }

  function handlePointerCancel(event: globalThis.PointerEvent) {
    if (activePointerId.value !== event.pointerId) return
    resetSwipeTracking(event.pointerId)
    settleTo(getActiveIndex())
  }

  function handleClickCapture(event: globalThis.MouseEvent) {
    if (window.performance.now() >= suppressClickUntil) return
    event.preventDefault()
    event.stopPropagation()
  }

  function snapToCurrentPage() {
    const track = getTrack()
    if (!track) return
    track.style.transition = ''
    track.style.transform = `translate3d(${-(getActiveIndex() * pageStageWidth.value)}px, 0, 0)`
  }

  function cleanup() {
    resetSwipeTracking(activePointerId.value)
  }

  return {
    isSwipeDragging,
    handlePointerDown,
    handlePointerMove,
    handlePointerEnd,
    handlePointerCancel,
    handleClickCapture,
    settleTo,
    snapToCurrentPage,
    cleanup,
  }
}
