export class Spring {
  pos: number
  vel: number
  target: number
  stiffness: number
  damping: number
  restThreshold: number
  private _rafId: number | null = null
  private _onUpdate: ((pos: number) => void) | null = null
  private _onRest: (() => void) | null = null
  private _lastTime = 0

  constructor(opts: { stiffness?: number; damping?: number; restThreshold?: number } = {}) {
    this.pos = 0
    this.vel = 0
    this.target = 0
    this.stiffness = opts.stiffness ?? 280
    this.damping = opts.damping ?? 26
    this.restThreshold = opts.restThreshold ?? 0.5
  }

  setTarget(target: number, onUpdate: (pos: number) => void, onRest?: () => void) {
    this.target = target
    this._onUpdate = onUpdate
    this._onRest = onRest ?? null
    this._lastTime = window.performance.now()
    if (!this._rafId) {
      this._tick()
    }
  }

  retarget(target: number, vel?: number, onUpdate?: (pos: number) => void, onRest?: () => void) {
    this.target = target
    if (vel !== undefined) this.vel = vel
    if (onUpdate) this._onUpdate = onUpdate
    if (onRest) this._onRest = onRest
    if (!this._rafId) {
      this._lastTime = window.performance.now()
      this._tick()
    }
  }

  setPos(pos: number) {
    this.pos = pos
  }

  stop() {
    if (this._rafId) {
      cancelAnimationFrame(this._rafId)
      this._rafId = null
    }
  }

  private _tick = () => {
    this._rafId = requestAnimationFrame(this._tick)
    const now = window.performance.now()
    const dt = Math.min((now - this._lastTime) / 1000, 0.064)
    this._lastTime = now

    const displacement = this.pos - this.target
    const accel = -this.stiffness * displacement - this.damping * this.vel
    this.vel += accel * dt
    this.pos += this.vel * dt

    this._onUpdate?.(this.pos)

    if (
      Math.abs(this.pos - this.target) < this.restThreshold &&
      Math.abs(this.vel) < this.restThreshold
    ) {
      this.pos = this.target
      this.vel = 0
      this._onUpdate?.(this.pos)
      this._onRest?.()
      cancelAnimationFrame(this._rafId!)
      this._rafId = null
    }
  }
}
