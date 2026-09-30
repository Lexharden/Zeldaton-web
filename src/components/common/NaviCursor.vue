<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { t } from '@/i18n'

/**
 * A fairy companion that flies after the mouse (desktop only).
 * Procedural canvas art: glowing core, translucent flapping wings, comet trail and sparkles.
 * Physics: underdamped spring toward the cursor + layered sine flutter, so she overshoots,
 * hovers and loops instead of sticking to the pointer. Turns golden over interactive elements.
 * Disabled for touch devices and prefers-reduced-motion. Rendering pauses when the tab is hidden.
 *
 * Easter egg: play Zelda's Lullaby with the arrow keys (← ↑ → ← ↑ →, the ocarina's C-left, C-up,
 * C-right) and Navi plays the song, turns into a rainbow, draws the Triforce and shouts "Hey! Listen!".
 */
const canvas = ref<HTMLCanvasElement | null>(null)
const enabled = ref(false)

interface Particle {
  x: number
  y: number
  vx: number
  vy: number
  life: number
  max: number
  size: number
  hue: number
  star: boolean
  phase: number
}

const MAX_PARTICLES = 160
const TAU = Math.PI * 2

let ctx: CanvasRenderingContext2D | null = null
let raf = 0
let last = 0
let w = 0
let h = 0
let dpr = 1
let visible = false // mouse inside the window at least once
let alpha = 0 // fades in/out
let excite = 0 // 0 = calm blue, 1 = golden "target" glow
let clock = 0
let emitAcc = 0

/** Zelda's Lullaby on the ocarina: C-left, C-up, C-right, twice. */
const LULLABY = ['ArrowLeft', 'ArrowUp', 'ArrowRight', 'ArrowLeft', 'ArrowUp', 'ArrowRight']
const PARTY_SECONDS = 6
let played: string[] = []
let party = 0 // seconds left of the easter egg

const mouse = { x: -200, y: -200 }
const navi = { x: -200, y: -200, vx: 0, vy: 0 }
const trail: { x: number; y: number }[] = []
const particles: Particle[] = []
let cleanup: (() => void)[] = []

function resize() {
  const el = canvas.value
  if (!el) return
  dpr = Math.min(window.devicePixelRatio || 1, 2)
  w = window.innerWidth
  h = window.innerHeight
  el.width = Math.round(w * dpr)
  el.height = Math.round(h * dpr)
  ctx = el.getContext('2d')
  ctx?.setTransform(dpr, 0, 0, dpr, 0, 0)
}

function isInteractive(target: EventTarget | null): boolean {
  return (
    target instanceof Element &&
    !!target.closest('a, button, [role="button"], summary, input, select, textarea')
  )
}

function spawn(x: number, y: number, speed: number, burst = false) {
  if (particles.length >= MAX_PARTICLES) particles.shift()
  const angle = Math.random() * TAU
  const spread = burst ? 70 + Math.random() * 130 : 8 + Math.random() * 22
  const gold = Math.random() < excite * 0.8
  particles.push({
    x: x + (Math.random() - 0.5) * 6,
    y: y + (Math.random() - 0.5) * 6,
    vx: Math.cos(angle) * spread - navi.vx * 0.12,
    vy: Math.sin(angle) * spread - navi.vy * 0.12 + (burst ? 0 : 6),
    life: 0,
    max: burst
      ? 0.8 + Math.random() * 0.9
      : 0.55 + Math.random() * 0.95 + Math.min(speed / 1600, 0.4),
    size: burst ? 1.6 + Math.random() * 2.8 : 1.1 + Math.random() * 2.3,
    hue:
      party > 0 ? Math.random() * 360 : gold ? 44 + Math.random() * 10 : 190 + Math.random() * 30,
    star: Math.random() < 0.18,
    phase: Math.random() * TAU,
  })
}

function update(dt: number) {
  clock += dt

  // Target hovers up and to the right of the pointer, drifting on layered sines (flutter).
  const fx = Math.cos(clock * 1.7) * 16 + Math.cos(clock * 3.3 + 1.2) * 6
  const fy = Math.sin(clock * 2.1) * 13 + Math.sin(clock * 4.1) * 4 - 26
  const tx = mouse.x + 22 + fx
  const ty = mouse.y + fy

  // Underdamped spring: overshoots slightly, reads as flying rather than snapping.
  const k = 46
  const c = 8.2
  navi.vx += ((tx - navi.x) * k - navi.vx * c) * dt
  navi.vy += ((ty - navi.y) * k - navi.vy * c) * dt
  // Small erratic gust so she never looks mechanical.
  navi.vx += Math.sin(clock * 5.7) * 30 * dt
  navi.vy += Math.cos(clock * 6.3) * 30 * dt
  navi.x += navi.vx * dt
  navi.y += navi.vy * dt

  trail.push({ x: navi.x, y: navi.y })
  if (trail.length > 16) trail.shift()

  alpha += ((visible ? 1 : 0) - alpha) * Math.min(1, dt * 6)

  party = Math.max(0, party - dt)
  const speed = Math.hypot(navi.vx, navi.vy)
  emitAcc += dt * (26 + speed * 0.09 + (party > 0 ? 60 : 0))
  while (emitAcc >= 1) {
    spawn(navi.x, navi.y, speed)
    emitAcc -= 1
  }

  for (let i = particles.length - 1; i >= 0; i--) {
    const p = particles[i]
    p.life += dt
    if (p.life >= p.max) {
      particles.splice(i, 1)
      continue
    }
    p.vx *= 1 - Math.min(1, dt * 1.6)
    p.vy = p.vy * (1 - Math.min(1, dt * 1.6)) + 14 * dt // faint gravity
    p.x += p.vx * dt
    p.y += p.vy * dt
  }
}

function glow(x: number, y: number, r: number, stops: [number, string][]) {
  if (!ctx) return
  const g = ctx.createRadialGradient(x, y, 0, x, y, r)
  for (const [o, col] of stops) g.addColorStop(o, col)
  ctx.fillStyle = g
  ctx.beginPath()
  ctx.arc(x, y, r, 0, TAU)
  ctx.fill()
}

function drawWing(x: number, y: number, angle: number, len: number, wid: number, a: number) {
  if (!ctx) return
  ctx.save()
  ctx.translate(x, y)
  ctx.rotate(angle)
  const g = ctx.createLinearGradient(0, 0, len, 0)
  g.addColorStop(0, `hsla(200, 100%, 96%, ${0.85 * a})`)
  g.addColorStop(0.6, `hsla(195, 100%, 80%, ${0.4 * a})`)
  g.addColorStop(1, `hsla(205, 100%, 70%, 0)`)
  ctx.fillStyle = g
  ctx.beginPath()
  ctx.ellipse(len / 2, 0, len / 2, wid, 0, 0, TAU)
  ctx.fill()
  ctx.restore()
}

function draw() {
  if (!ctx) return
  ctx.clearRect(0, 0, w, h)
  if (alpha < 0.01) return
  ctx.globalCompositeOperation = 'lighter'

  // Blue -> gold over links; the easter egg cycles through every colour.
  const hue = party > 0 ? (clock * 140) % 360 : 200 - excite * 150
  const core = `hsla(${hue}, 100%, 96%, ${alpha})`
  const mid = `hsla(${hue}, 100%, 72%, ${0.75 * alpha})`
  const halo = `hsla(${hue}, 100%, 60%, ${0.28 * alpha})`

  // Comet trail.
  for (let i = 0; i < trail.length - 1; i++) {
    const f = i / trail.length
    glow(trail[i].x, trail[i].y, 3 + f * 6, [
      [0, `hsla(${hue}, 100%, 85%, ${f * 0.22 * alpha})`],
      [1, `hsla(${hue}, 100%, 60%, 0)`],
    ])
  }

  // Particles.
  for (const p of particles) {
    const f = 1 - p.life / p.max
    const tw = 0.65 + 0.35 * Math.sin(clock * 18 + p.phase)
    const a = f * f * tw * alpha
    const r = p.size * (0.5 + f)
    if (p.star) {
      ctx.strokeStyle = `hsla(${p.hue}, 100%, 88%, ${a})`
      ctx.lineWidth = 0.9
      const s = r * 3.2
      ctx.beginPath()
      ctx.moveTo(p.x - s, p.y)
      ctx.lineTo(p.x + s, p.y)
      ctx.moveTo(p.x, p.y - s)
      ctx.lineTo(p.x, p.y + s)
      ctx.stroke()
    }
    glow(p.x, p.y, r * 3, [
      [0, `hsla(${p.hue}, 100%, 92%, ${a})`],
      [0.4, `hsla(${p.hue}, 100%, 70%, ${a * 0.5})`],
      [1, `hsla(${p.hue}, 100%, 60%, 0)`],
    ])
  }

  // Navi: bobbing, banking a little with horizontal speed.
  const bob = Math.sin(clock * 9) * 1.6
  const x = navi.x
  const y = navi.y + bob
  const bank = Math.max(-0.5, Math.min(0.5, navi.vx / 900))
  const flap = Math.sin(clock * (34 + Math.hypot(navi.vx, navi.vy) * 0.01))
  const spread = 0.55 + 0.5 * flap // wing opening angle

  ctx.save()
  ctx.translate(x, y)
  ctx.rotate(bank)
  // Outer atmosphere + pulsing halo.
  const pulse = 1 + Math.sin(clock * 5) * 0.08
  glow(0, 0, 34 * pulse, [
    [0, halo],
    [1, `hsla(${hue}, 100%, 55%, 0)`],
  ])
  // Wings: upper pair larger, lower pair smaller; mirrored.
  drawWing(-1, -1, -Math.PI + 0.35 - spread * 0.5, 24, 8.5, alpha)
  drawWing(1, -1, -0.35 + spread * 0.5, 24, 8.5, alpha)
  drawWing(-1, 1, Math.PI - 0.2 + spread * 0.35, 17, 6, alpha * 0.85)
  drawWing(1, 1, 0.2 - spread * 0.35, 17, 6, alpha * 0.85)
  // Body.
  glow(0, 0, 11, [
    [0, core],
    [0.5, mid],
    [1, `hsla(${hue}, 100%, 60%, 0)`],
  ])
  glow(0, 0, 3.6, [
    [0, `hsla(0, 0%, 100%, ${alpha})`],
    [1, `hsla(${hue}, 100%, 90%, 0)`],
  ])
  ctx.restore()

  ctx.globalCompositeOperation = 'source-over'
  if (party > 0) drawParty(x, y)
}

/** Easter egg overlay: a golden Triforce above Navi and a "Hey! Listen!" bubble. */
function drawParty(x: number, y: number) {
  if (!ctx) return
  // Fade in over the first 0.4 s and out over the last second.
  const shown = PARTY_SECONDS - party
  const a = Math.min(1, shown / 0.4, party) * alpha

  // Triforce: three golden triangles, gently spinning in and pulsing.
  const size = 15 * (0.8 + Math.min(1, shown * 2) * 0.2 + Math.sin(clock * 6) * 0.04)
  const tx = x
  const ty = y - 58
  ctx.save()
  ctx.globalCompositeOperation = 'lighter'
  glow(tx, ty + size * 0.3, size * 3, [
    [0, `hsla(46, 100%, 70%, ${0.35 * a})`],
    [1, 'hsla(46, 100%, 60%, 0)'],
  ])
  ctx.globalCompositeOperation = 'source-over'
  ctx.translate(tx, ty)
  ctx.rotate(Math.max(0, 1 - shown * 2) * Math.PI)
  const hgt = size * Math.sqrt(3)
  const tri = (cx: number, cy: number) => {
    if (!ctx) return
    ctx.moveTo(cx, cy - hgt / 2)
    ctx.lineTo(cx + size, cy + hgt / 2)
    ctx.lineTo(cx - size, cy + hgt / 2)
    ctx.closePath()
  }
  ctx.beginPath()
  tri(0, -hgt / 2)
  tri(-size, hgt / 2)
  tri(size, hgt / 2)
  ctx.fillStyle = `hsla(46, 95%, 58%, ${a})`
  ctx.strokeStyle = `hsla(40, 90%, 30%, ${a})`
  ctx.lineWidth = 1.2
  ctx.fill()
  ctx.stroke()
  ctx.restore()

  // Speech bubble to the right of Navi.
  const text = t('navi.listen')
  ctx.save()
  ctx.font = '700 16px "Barlow Condensed", "Inter Variable", system-ui, sans-serif'
  const padX = 10
  const bw = ctx.measureText(text).width + padX * 2
  const bh = 26
  const bx = Math.min(x + 26, w - bw - 8)
  const by = Math.max(8, y - 44)
  ctx.globalAlpha = a
  ctx.fillStyle = 'rgba(6, 12, 16, 0.88)'
  ctx.strokeStyle = 'rgba(120, 200, 255, 0.9)'
  ctx.lineWidth = 1.5
  ctx.beginPath()
  ctx.roundRect(bx, by, bw, bh, 8)
  ctx.moveTo(bx + 10, by + bh)
  ctx.lineTo(bx + 4, by + bh + 8)
  ctx.lineTo(bx + 18, by + bh)
  ctx.fill()
  ctx.stroke()
  ctx.fillStyle = '#e8f6ff'
  ctx.textBaseline = 'middle'
  ctx.fillText(text, bx + padX, by + bh / 2 + 1)
  ctx.restore()
}

/** Zelda's Lullaby on a soft, ocarina-like synth (no audio files). */
function playLullaby() {
  try {
    const Ctx =
      window.AudioContext ??
      (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext
    if (!Ctx) return
    const ac = new Ctx()
    // B4 D5 A4, B4 D5 A4 — long, short, long.
    const notes: [number, number][] = [
      [493.88, 0.6],
      [587.33, 0.3],
      [440.0, 0.9],
      [493.88, 0.6],
      [587.33, 0.3],
      [440.0, 1.3],
    ]
    const out = ac.createGain()
    out.gain.value = 0.18
    out.connect(ac.destination)
    let at = ac.currentTime + 0.05
    for (const [freq, dur] of notes) {
      const osc = ac.createOscillator()
      const env = ac.createGain()
      const vibrato = ac.createOscillator()
      const depth = ac.createGain()
      osc.type = 'sine'
      osc.frequency.value = freq
      vibrato.frequency.value = 5.5
      depth.gain.value = freq * 0.006
      vibrato.connect(depth).connect(osc.frequency)
      env.gain.setValueAtTime(0, at)
      env.gain.linearRampToValueAtTime(1, at + 0.05)
      env.gain.setValueAtTime(1, at + dur * 0.75)
      env.gain.linearRampToValueAtTime(0, at + dur)
      osc.connect(env).connect(out)
      osc.start(at)
      vibrato.start(at)
      osc.stop(at + dur + 0.02)
      vibrato.stop(at + dur + 0.02)
      at += dur
    }
    setTimeout(() => void ac.close(), (at - ac.currentTime + 0.5) * 1000)
  } catch {
    // Audio is a bonus: the visual part still plays.
  }
}

/** Keys typed into a form are never part of the song. */
function isTyping(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable || !!target.closest('input, textarea, select'))
  )
}

function onSongKey(e: KeyboardEvent) {
  if (e.altKey || e.ctrlKey || e.metaKey || isTyping(e.target)) return
  if (!e.key.startsWith('Arrow')) {
    played = []
    return
  }
  played = [...played, e.key].slice(-LULLABY.length)
  if (played.length === LULLABY.length && played.every((k, i) => k === LULLABY[i])) {
    played = []
    party = PARTY_SECONDS
    // If the mouse never entered the page, show her in the middle of the screen.
    if (!visible) {
      visible = true
      mouse.x = navi.x = w / 2
      mouse.y = navi.y = h / 2
    }
    for (let i = 0; i < 70; i++) spawn(navi.x, navi.y, 0, true)
    playLullaby()
  }
}

function frame(t: number) {
  raf = requestAnimationFrame(frame)
  const dt = Math.min(0.05, (t - last) / 1000 || 0.016)
  last = t
  update(dt)
  draw()
}

function start() {
  const move = (e: MouseEvent) => {
    if (!visible) {
      // First movement: appear beside the pointer instead of flying in from the corner.
      navi.x = e.clientX + 20
      navi.y = e.clientY - 24
      trail.length = 0
    }
    visible = true
    mouse.x = e.clientX
    mouse.y = e.clientY
  }
  const over = (e: MouseEvent) => {
    excite = isInteractive(e.target) ? 1 : 0
  }
  const leave = () => (visible = false)
  const down = (e: MouseEvent) => {
    for (let i = 0; i < 22; i++) spawn(e.clientX, e.clientY, 0, true)
  }
  const vis = () => {
    if (document.hidden) {
      cancelAnimationFrame(raf)
      raf = 0
    } else if (!raf) {
      last = performance.now()
      raf = requestAnimationFrame(frame)
    }
  }
  window.addEventListener('mousemove', move, { passive: true })
  window.addEventListener('mouseover', over, { passive: true })
  window.addEventListener('mousedown', down, { passive: true })
  window.addEventListener('keydown', onSongKey)
  window.addEventListener('resize', resize)
  document.documentElement.addEventListener('mouseleave', leave)
  document.addEventListener('visibilitychange', vis)
  cleanup = [
    () => window.removeEventListener('mousemove', move),
    () => window.removeEventListener('mouseover', over),
    () => window.removeEventListener('mousedown', down),
    () => window.removeEventListener('keydown', onSongKey),
    () => window.removeEventListener('resize', resize),
    () => document.documentElement.removeEventListener('mouseleave', leave),
    () => document.removeEventListener('visibilitychange', vis),
  ]
  resize()
  last = performance.now()
  raf = requestAnimationFrame(frame)
}

onMounted(() => {
  // Touch-first devices (phones/tablets) report a coarse primary pointer: no cursor to follow.
  const fine = !window.matchMedia('(pointer: coarse)').matches
  const reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches
  if (!fine || reduce) return
  enabled.value = true
  // Wait a tick so the canvas exists in the DOM.
  requestAnimationFrame(start)
})

onBeforeUnmount(() => {
  cancelAnimationFrame(raf)
  cleanup.forEach((fn) => fn())
})
</script>

<template>
  <canvas
    v-if="enabled"
    ref="canvas"
    class="pointer-events-none fixed inset-0 z-[90] size-full"
    aria-hidden="true"
  />
</template>
