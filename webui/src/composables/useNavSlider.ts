// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (c) 2025-2026 Seyud
// Device Faker: https://github.com/Seyud/device_faker

import { computed, onBeforeUnmount, onMounted, ref, type Ref } from 'vue'

/**
 * 底部导航「悬浮滑块」拖动切页。
 *
 * 悬浮底栏本身就是一个分段控件，所以除点击外还支持：按住高亮滑块横向拖动，
 * 页面轨道跟着手指实时移动（onPreview），松手吸附到最近的页（onCommit）。
 *
 * 与 `usePageSwipe` 的分工：滑动整页内容仍由 usePageSwipe 负责（它挂在
 * `.page-stage` 上）；这里只管底栏自身的指针手势，并通过回调把进度同步给页面
 * 轨道，两者互不抢夺指针（底栏在 stage 之外）。
 *
 * 关键细节：
 * - 拖到别处才 setPointerCapture：否则 pointerup 的 target 变成底栏，
 *   按钮的 click 不再派发，点按切页会失效；
 * - 起手先判方向，竖向或幅度过小（< DRAG_START_PX）一律当点按，不做任何抑制；
 * - 真正拖过之后用 320ms 抑制随后的 click，避免「松手吸附到 B」又被
 *   「抬手处按钮的 click」拉回 A；
 * - 几何量（每个 tab 的 offset/宽度/步长）在挂载、resize、ResizeObserver
 *   时重新测量，拖动过程只用快照，避免高频 getBoundingClientRect。
 */
const DRAG_START_PX = 4
const VERTICAL_INTENT_RATIO = 1.2
const FLICK_VELOCITY = 0.3
const FLICK_MIN_PROGRESS = 0.15
const CLICK_SUPPRESS_MS = 320

interface NavMetrics {
  /** 每个 tab 相对底栏左边缘的偏移（px） */
  offsets: number[]
  /** 单个 tab 的宽度（px） */
  width: number
  /** 相邻两个 tab 的间距（px），也就是「一页」对应的拖动距离 */
  step: number
}

const EMPTY_METRICS: NavMetrics = { offsets: [], width: 0, step: 0 }

export interface NavSliderOptions {
  navRef: Ref<HTMLElement | null>
  /** 当前页下标 */
  activeIndex: Ref<number>
  /** tab 数量（正常恒为 4，仍按数据算，避免以后加页时写死） */
  itemCount: Ref<number>
  /** 拖动中同步页面轨道；progress 以「页」为单位（-0.5 表示向左半页） */
  onPreview: (index: number, progress: number) => void
  /** 松手落点；index 可能与起手页相同（= 回弹） */
  onCommit: (index: number) => void
  /** 拖动经过的页发生变化，用于预热相邻页 */
  onHoverIndex?: (index: number) => void
}

export function useNavSlider(opts: NavSliderOptions) {
  const isDragging = ref(false)
  const dragOffset = ref(0)
  const metrics = ref<NavMetrics>(EMPTY_METRICS)

  let activePointerId: number | null = null
  let startIndex = 0
  let startX = 0
  let startY = 0
  let lastX = 0
  let lastTime = 0
  let velocity = 0
  let intent: 'none' | 'horizontal' | 'vertical' = 'none'
  /** 起手时的几何快照，拖动全程只读它 */
  let dragMetrics: NavMetrics = EMPTY_METRICS
  let hoverIndex = -1
  let suppressClickUntil = 0
  let resizeObserver: ResizeObserver | null = null

  const thumbStyle = computed(() => {
    const { offsets, width } = metrics.value
    const base = offsets[opts.activeIndex.value] ?? 0
    const x = base + (isDragging.value ? dragOffset.value : 0)
    return {
      width: `${width}px`,
      transform: `translate3d(${x}px, 0, 0)`,
    }
  })

  function clampIndex(index: number) {
    const last = Math.max(opts.itemCount.value - 1, 0)
    return Math.min(Math.max(index, 0), last)
  }

  function measure() {
    const nav = opts.navRef.value
    if (!nav) return
    const navRect = nav.getBoundingClientRect()
    const rects = Array.from(nav.querySelectorAll<HTMLElement>('.nav-item')).map((el) =>
      el.getBoundingClientRect()
    )
    if (rects.length === 0) return
    metrics.value = {
      offsets: rects.map((rect) => Math.round(rect.left - navRect.left)),
      width: Math.round(rects[0].width),
      step: rects.length > 1 ? Math.round(rects[1].left - rects[0].left) : 0,
    }
  }

  function reset() {
    const nav = opts.navRef.value
    if (activePointerId !== null && nav?.hasPointerCapture?.(activePointerId)) {
      nav.releasePointerCapture(activePointerId)
    }
    activePointerId = null
    intent = 'none'
    isDragging.value = false
    dragOffset.value = 0
    hoverIndex = -1
  }

  function handlePointerDown(event: globalThis.PointerEvent) {
    if (event.button !== 0 || activePointerId !== null) return
    // 布局可能刚变（横竖屏、分屏），先刷新一次几何量再开拖
    measure()
    dragMetrics = metrics.value
    if (dragMetrics.step <= 0 || dragMetrics.offsets.length === 0) return

    activePointerId = event.pointerId
    startIndex = clampIndex(opts.activeIndex.value)
    startX = lastX = event.clientX
    startY = event.clientY
    lastTime = event.timeStamp
    velocity = 0
    intent = 'none'
    dragOffset.value = 0
    hoverIndex = startIndex
  }

  function handlePointerMove(event: globalThis.PointerEvent) {
    if (activePointerId === null || event.pointerId !== activePointerId) return

    const deltaX = event.clientX - startX
    const deltaY = event.clientY - startY
    const nav = opts.navRef.value

    if (intent === 'none') {
      const absX = Math.abs(deltaX)
      const absY = Math.abs(deltaY)
      if (absX < DRAG_START_PX && absY < DRAG_START_PX) return
      if (absX <= absY * VERTICAL_INTENT_RATIO) {
        // 竖向意图：交还事件，不当作拖动
        intent = 'vertical'
        reset()
        return
      }
      intent = 'horizontal'
      isDragging.value = true
      nav?.setPointerCapture?.(event.pointerId)
    }

    if (intent !== 'horizontal') return

    const now = window.performance.now()
    const elapsed = now - lastTime
    if (elapsed > 0) {
      velocity = (event.clientX - lastX) / elapsed
    }
    lastX = event.clientX
    lastTime = now

    const { step } = dragMetrics
    const minOffset = -startIndex * step
    const maxOffset = (opts.itemCount.value - 1 - startIndex) * step
    dragOffset.value = Math.min(Math.max(deltaX, minOffset), maxOffset)

    if (step > 0) {
      opts.onPreview(startIndex, dragOffset.value / step)
      const hovered = clampIndex(startIndex + Math.round(dragOffset.value / step))
      if (hovered !== hoverIndex) {
        hoverIndex = hovered
        opts.onHoverIndex?.(hovered)
      }
    }

    event.preventDefault()
  }

  function handlePointerEnd(event: globalThis.PointerEvent) {
    if (activePointerId === null || event.pointerId !== activePointerId) return

    const wasDragging = intent === 'horizontal'
    let target = startIndex

    if (wasDragging) {
      const { step } = dragMetrics
      const progress = step > 0 ? dragOffset.value / step : 0
      let delta = Math.round(progress)
      if (delta === 0 && Math.abs(progress) >= FLICK_MIN_PROGRESS) {
        // 快速一甩即便没到半格也切一页
        if (velocity <= -FLICK_VELOCITY) delta = -1
        else if (velocity >= FLICK_VELOCITY) delta = 1
      }
      target = clampIndex(startIndex + delta)
      suppressClickUntil = window.performance.now() + CLICK_SUPPRESS_MS
    }

    reset()
    opts.onCommit(target)
  }

  function handlePointerCancel(event: globalThis.PointerEvent) {
    if (activePointerId === null || event.pointerId !== activePointerId) return
    const wasDragging = intent === 'horizontal'
    reset()
    if (wasDragging) {
      suppressClickUntil = window.performance.now() + CLICK_SUPPRESS_MS
    }
    opts.onCommit(startIndex)
  }

  /** click 处理器先问一句：这次 click 是不是拖动尾巴，是就丢掉 */
  function shouldSuppressClick() {
    return window.performance.now() < suppressClickUntil
  }

  onMounted(() => {
    measure()
    if (typeof window.ResizeObserver === 'function' && opts.navRef.value) {
      resizeObserver = new window.ResizeObserver(() => measure())
      resizeObserver.observe(opts.navRef.value)
    }
    window.addEventListener('resize', measure, { passive: true })
  })

  onBeforeUnmount(() => {
    resizeObserver?.disconnect()
    resizeObserver = null
    window.removeEventListener('resize', measure)
    reset()
  })

  return {
    isDragging,
    thumbStyle,
    measure,
    shouldSuppressClick,
    handlePointerDown,
    handlePointerMove,
    handlePointerEnd,
    handlePointerCancel,
  }
}
