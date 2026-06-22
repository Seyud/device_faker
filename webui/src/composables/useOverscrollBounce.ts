import { Spring } from '../utils/Spring'
import { hasVerticalScrollableAncestor } from './usePageSwipe'

const OVERSCROLL_BOUNCE_MAX_PX = 120
const OVERSCROLL_BOUNCE_RESISTANCE = 0.4
const OVERSCROLL_STRETCH_RATIO = 0.0008

function normalizeTargetElement(target: globalThis.EventTarget | null) {
  if (target instanceof HTMLElement) return target
  if (target instanceof window.Node) return target.parentElement
  return null
}

function findTouchById(
  touchList: globalThis.TouchList,
  touchId: number | null
): globalThis.Touch | null {
  if (touchId === null) return null
  for (let i = 0; i < touchList.length; i += 1) {
    const touch = touchList.item(i)
    if (touch?.identifier === touchId) return touch
  }
  return null
}

export function useOverscrollBounce() {
  const spring = new Spring({ stiffness: 300, damping: 28, restThreshold: 0.3 })

  let scrollElement: HTMLElement | null = null
  let touchId: number | null = null
  let releasing = false
  let startX = 0
  let startY = 0
  let offset = 0

  function applyTransform(el: HTMLElement, offsetPx: number) {
    const content = el.querySelector('.page-scroll-content') as HTMLElement | null
    if (!content) return
    const stretchScale = 1 + Math.abs(offsetPx) * OVERSCROLL_STRETCH_RATIO
    content.style.transform = `scaleY(${stretchScale})`
    content.style.transformOrigin = offsetPx >= 0 ? '50% top' : '50% bottom'
  }

  function clearState() {
    scrollElement = null
    touchId = null
    startX = 0
    startY = 0
    offset = 0
    releasing = false
  }

  function releaseSpring(el: HTMLElement) {
    if (releasing) return
    releasing = true
    spring.stop()
    spring.pos = offset
    spring.vel = 0

    const content = el.querySelector('.page-scroll-content') as HTMLElement | null
    if (content) content.style.willChange = 'transform'

    spring.setTarget(
      0,
      (pos) => {
        applyTransform(el, pos)
      },
      () => {
        if (content) {
          content.style.transform = ''
          content.style.transformOrigin = ''
          content.style.willChange = ''
        }
        clearState()
      }
    )
  }

  function handleTouchStart(event: globalThis.TouchEvent) {
    if (event.touches.length !== 1) return

    const el = event.currentTarget
    if (!(el instanceof HTMLElement)) return

    if (hasVerticalScrollableAncestor(normalizeTargetElement(event.target), el)) {
      clearState()
      return
    }

    const touch = event.touches.item(0)
    if (!touch) return

    spring.stop()
    releasing = false
    scrollElement = el
    touchId = touch.identifier
    startX = touch.clientX
    startY = touch.clientY
    offset = 0
  }

  function handleTouchMove(event: globalThis.TouchEvent) {
    const el = scrollElement
    if (!el || event.currentTarget !== el) return
    if (releasing) return

    const touch = findTouchById(event.touches, touchId)
    if (!touch) return

    const deltaX = touch.clientX - startX
    const deltaY = touch.clientY - startY

    if (Math.abs(deltaY) <= Math.abs(deltaX)) {
      if (offset !== 0) releaseSpring(el)
      return
    }

    const maxScrollTop = Math.max(el.scrollHeight - el.clientHeight, 0)
    const isAtTop = el.scrollTop <= 0
    const isAtBottom = el.scrollTop >= maxScrollTop - 1
    const pullingPastTop = isAtTop && deltaY > 0
    const pullingPastBottom = isAtBottom && deltaY < 0

    if (!pullingPastTop && !pullingPastBottom) {
      if (offset !== 0) releaseSpring(el)
      return
    }

    const absDeltaY = Math.abs(deltaY)
    const direction = Math.sign(deltaY)
    const damped = Math.sqrt(absDeltaY) * OVERSCROLL_BOUNCE_RESISTANCE * 8
    const resistedOffset = direction * Math.min(OVERSCROLL_BOUNCE_MAX_PX, damped)

    offset = resistedOffset
    spring.pos = resistedOffset
    spring.vel = 0
    applyTransform(el, resistedOffset)
    event.preventDefault()
  }

  function handleTouchEnd(event: globalThis.TouchEvent) {
    const el = scrollElement
    if (!el || event.currentTarget !== el) return

    if (offset !== 0) {
      releaseSpring(el)
    } else {
      const content = el.querySelector('.page-scroll-content') as HTMLElement | null
      if (content) {
        content.style.transform = ''
        content.style.transformOrigin = ''
      }
      clearState()
    }
  }

  function cleanup() {
    spring.stop()
    clearState()
  }

  return {
    handleTouchStart,
    handleTouchMove,
    handleTouchEnd,
    cleanup,
  }
}
