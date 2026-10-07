<script setup lang="ts">
import { onBeforeUnmount, onMounted, ref } from 'vue'
import { t } from '@/i18n'
import { type Note, P, type Wave, playNotes, playSample } from '@/utils/ocarina'

/**
 * Hidden surprises for people who know the game. Nothing here is advertised:
 *
 *   ↑ ↑ ↓ ↓ ← → ← → B A      Konami code: a rain of rupees and the "secret found" jingle
 *   ↓ → ← ↓ → ←              Saria's Song: green notes and hearts float up
 *   ↑ ← → ↑ ← →              Epona's Song: Epona gallops across the screen
 *   → ↓ ↑ → ↓ ↑              Sun's Song: the sun floods the page with light
 *   A ↓ ↑ A ↓ ↑              Song of Storms: rain and lightning
 *   c u c c o                Cuccos everywhere
 *   g a n o n                Ganondorf's darkness and embers
 *   c l a c o m e            Plays public/sounds/solenacho.ogg while notes float up
 *
 * (Zelda's Lullaby, ← ↑ → ← ↑ →, belongs to Navi: see NaviCursor.vue.)
 * One shared canvas; it only runs while an effect is playing. Keys typed into a form never count,
 * and with prefers-reduced-motion only the banner and the sound play.
 */
const canvas = ref<HTMLCanvasElement | null>(null)
const banner = ref('')
let bannerTimer = 0

type Kind = 'rupee' | 'drop' | 'glyph' | 'spark'
interface Particle {
  kind: Kind
  x: number
  y: number
  vx: number
  vy: number
  life: number
  max: number
  size: number
  rot: number
  vr: number
  hue: number
  glyph: string
  /** Chickens hop on the ground instead of flying free. */
  ground?: number
}

type Overlay = 'sun' | 'storm' | 'ganon' | 'saria' | null
interface Effect {
  /** Key sequence (see `token`) that triggers it. */
  keys: string
  seconds: number
  /** Title shown as a banner. */
  title: () => string
  overlay?: Overlay
  /** A synthesized tune (the default way to make sound)... */
  melody?: Note[]
  wave?: Wave
  /** ...or a recorded sound served by the site, played instead of the tune. */
  sound?: string
  /** Called every frame while the effect lasts; spawn particles here. */
  tick: (dt: number, elapsed: number) => void
  /** Called once when it starts. */
  start?: () => void
}

const TAU = Math.PI * 2
const MAX_PARTICLES = 420
const particles: Particle[] = []
let ctx: CanvasRenderingContext2D | null = null
let w = 0
let h = 0
let raf = 0
let last = 0
let clock = 0
let active: { effect: Effect; elapsed: number } | null = null
let overlayAlpha = 0
let flash = 0 // lightning
let acc = 0
let reduce = false
let buffer = ''

function add(p: Partial<Particle> & Pick<Particle, 'kind'>) {
  if (particles.length >= MAX_PARTICLES) particles.shift()
  particles.push({
    x: 0,
    y: 0,
    vx: 0,
    vy: 0,
    life: 0,
    max: 2,
    size: 10,
    rot: 0,
    vr: 0,
    hue: 120,
    glyph: '',
    ...p,
  })
}

/** Run `fn` at `perSecond` times per second (frame-rate independent). */
function every(perSecond: number, dt: number, fn: () => void) {
  acc += dt * perSecond
  while (acc >= 1) {
    fn()
    acc -= 1
  }
}
const rand = (a: number, b: number) => a + Math.random() * (b - a)

const RUPEE_HUES = [128, 128, 128, 215, 215, 0, 285, 45]

const EFFECTS: Effect[] = [
  {
    keys: 'UUDDLRLRba',
    seconds: 5.5,
    title: () => t('eggs.konami'),
    melody: [
      [783.99, 0.12],
      [739.99, 0.12],
      [622.25, 0.12],
      [440, 0.12],
      [415.3, 0.12],
      [659.25, 0.12],
      [830.61, 0.12],
      [1046.5, 0.5],
    ],
    tick(dt, elapsed) {
      if (elapsed > 4) return
      every(46, dt, () =>
        add({
          kind: 'rupee',
          x: rand(0, w),
          y: -20,
          vx: rand(-20, 20),
          vy: rand(220, 460),
          max: 6,
          size: rand(9, 15),
          rot: rand(-0.4, 0.4),
          vr: rand(-2, 2),
          hue: RUPEE_HUES[Math.floor(Math.random() * RUPEE_HUES.length)],
        }),
      )
    },
  },
  {
    keys: 'DRLDRL',
    seconds: 6,
    title: () => t('eggs.saria'),
    overlay: 'saria',
    melody: [
      [P.F4, 0.25],
      [P.A4, 0.25],
      [P.B4, 0.5],
      [P.F4, 0.25],
      [P.A4, 0.25],
      [P.B4, 0.5],
      [P.E5, 0.25],
      [P.D5, 0.25],
      [P.B4, 0.25],
      [P.C5, 0.25],
      [P.B4, 0.25],
      [P.G5 * 0.5, 0.5],
    ],
    tick(dt) {
      every(16, dt, () =>
        add({
          kind: 'glyph',
          glyph: ['♪', '♫', '♥', '♪'][Math.floor(Math.random() * 4)],
          x: rand(0, w),
          y: h + 20,
          vx: rand(-25, 25),
          vy: rand(-170, -80),
          max: rand(3, 5),
          size: rand(18, 38),
          hue: rand(95, 150),
          vr: rand(-0.6, 0.6),
        }),
      )
    },
  },
  {
    keys: 'ULRULR',
    seconds: 4.4,
    title: () => t('eggs.epona'),
    melody: [
      [P.D5, 0.3],
      [P.B4, 0.15],
      [P.A4, 0.6],
      [P.D5, 0.3],
      [P.B4, 0.15],
      [P.A4, 0.9],
    ],
    start() {
      // One big horse; the dust is spawned per frame below.
      add({
        kind: 'glyph',
        glyph: '🐎',
        x: -90,
        y: h * 0.72,
        vx: (w + 220) / 3.6,
        vy: 0,
        max: 3.8,
        size: Math.min(110, Math.max(64, w * 0.09)),
        hue: -1,
        ground: h * 0.72,
      })
    },
    tick(dt, elapsed) {
      const horse = particles.find((p) => p.glyph === '🐎')
      if (!horse || elapsed > 3.8) return
      horse.y = (horse.ground ?? horse.y) - Math.abs(Math.sin(elapsed * 11)) * 16
      every(40, dt, () =>
        add({
          kind: 'spark',
          x: horse.x - horse.size * 0.3,
          y: (horse.ground ?? horse.y) + horse.size * 0.45,
          vx: rand(-90, -20),
          vy: rand(-60, -10),
          max: rand(0.5, 1.1),
          size: rand(3, 7),
          hue: 35,
        }),
      )
    },
  },
  {
    keys: 'RDURDU',
    seconds: 5.5,
    title: () => t('eggs.sun'),
    overlay: 'sun',
    melody: [
      [P.A4, 0.25],
      [P.F4, 0.25],
      [P.D5, 0.5],
      [P.A4, 0.25],
      [P.F4, 0.25],
      [P.D5, 0.5],
    ],
    tick(dt) {
      every(30, dt, () =>
        add({
          kind: 'spark',
          x: rand(0, w),
          y: rand(0, h * 0.6),
          vx: rand(-10, 10),
          vy: rand(-20, 10),
          max: rand(0.8, 1.8),
          size: rand(2, 5),
          hue: rand(40, 52),
        }),
      )
    },
  },
  {
    keys: 'aDUaDU',
    seconds: 6,
    title: () => t('eggs.storms'),
    overlay: 'storm',
    melody: [
      [P.D4, 0.2],
      [P.F4, 0.2],
      [P.D5, 0.6],
      [P.D4, 0.2],
      [P.F4, 0.2],
      [P.D5, 0.6],
    ],
    tick(dt, elapsed) {
      every(210, dt, () =>
        add({
          kind: 'drop',
          x: rand(-40, w + 40),
          y: -20,
          vx: -120,
          vy: rand(900, 1300),
          max: 1.4,
          size: rand(10, 22),
        }),
      )
      // A couple of lightning strikes, never in the first second.
      if (elapsed > 1 && Math.random() < dt * 0.55) flash = 1
    },
  },
  {
    keys: 'cucco',
    seconds: 6,
    title: () => t('eggs.cucco'),
    melody: [
      [P.D5, 0.08],
      [P.E5, 0.08],
      [P.D5, 0.08],
      [P.E5, 0.08],
      [P.D5, 0.08],
      [P.E5, 0.08],
    ],
    wave: 'square',
    start() {
      for (let i = 0; i < 16; i++) {
        const dir = Math.random() < 0.5 ? 1 : -1
        add({
          kind: 'glyph',
          glyph: '🐔',
          x: dir > 0 ? rand(-300, -30) : w + rand(30, 300),
          y: h - rand(24, 70),
          vx: dir * rand(180, 340),
          max: 5.6,
          size: rand(30, 52),
          hue: -1,
          vr: rand(8, 14),
          ground: h - rand(24, 70),
        })
      }
    },
    tick() {
      for (const p of particles) {
        if (p.glyph === '🐔' && p.ground !== undefined)
          p.y = p.ground - Math.abs(Math.sin(clock * p.vr + p.x * 0.01)) * 26
      }
    },
  },
  {
    keys: 'clacome',
    seconds: 6, // the sound lasts about 5.9 s
    title: () => t('eggs.clacome'),
    sound: '/sounds/solenacho.ogg',
    tick(dt) {
      every(9, dt, () =>
        add({
          kind: 'glyph',
          glyph: ['🔊', '🎵', '🎶'][Math.floor(Math.random() * 3)],
          x: rand(w * 0.1, w * 0.9),
          y: h + 20,
          vx: rand(-30, 30),
          vy: rand(-220, -110),
          max: rand(2.5, 4),
          size: rand(26, 46),
          hue: -1,
          vr: rand(-0.6, 0.6),
        }),
      )
    },
  },
  {
    keys: 'ganon',
    seconds: 5,
    title: () => t('eggs.ganon'),
    overlay: 'ganon',
    melody: [
      [P.D3, 0.7],
      [P.D3 * 1.189, 0.7],
      [P.D3 * 1.122, 0.7],
      [P.D3, 1.4],
    ],
    wave: 'sawtooth',
    tick(dt) {
      every(34, dt, () =>
        add({
          kind: 'spark',
          x: rand(0, w),
          y: h + 10,
          vx: rand(-30, 30),
          vy: rand(-220, -90),
          max: rand(1.4, 3),
          size: rand(2, 5),
          hue: rand(0, 25),
        }),
      )
    },
  },
]

function showBanner(text: string, seconds: number) {
  banner.value = text
  window.clearTimeout(bannerTimer)
  bannerTimer = window.setTimeout(() => (banner.value = ''), seconds * 1000)
}

function trigger(effect: Effect) {
  showBanner(effect.title(), Math.min(4, effect.seconds))
  if (effect.sound) playSample(effect.sound)
  else if (effect.melody)
    playNotes(effect.melody, { type: effect.wave, volume: effect.wave ? 0.07 : 0.16 })
  if (reduce) return
  particles.length = 0
  acc = 0
  flash = 0
  active = { effect, elapsed: 0 }
  effect.start?.()
  if (!raf) {
    last = performance.now()
    raf = requestAnimationFrame(frame)
  }
}

/** One character per key: arrows are U D L R, letters are themselves, other keys reset the run. */
function token(e: KeyboardEvent): string | null {
  switch (e.key) {
    case 'ArrowUp':
      return 'U'
    case 'ArrowDown':
      return 'D'
    case 'ArrowLeft':
      return 'L'
    case 'ArrowRight':
      return 'R'
    case 'Shift':
    case 'Control':
    case 'Alt':
    case 'Meta':
    case 'CapsLock':
      return null // modifiers neither add to nor break a sequence
    default:
      return e.key.length === 1 && /[a-z]/i.test(e.key) ? e.key.toLowerCase() : '_'
  }
}

/** Keys typed into a form are never part of a song. */
function isTyping(target: EventTarget | null): boolean {
  return (
    target instanceof HTMLElement &&
    (target.isContentEditable || !!target.closest('input, textarea, select'))
  )
}

function onKey(e: KeyboardEvent) {
  if (e.repeat || e.altKey || e.ctrlKey || e.metaKey || isTyping(e.target)) return
  const k = token(e)
  if (k === null) return
  buffer = (buffer + k).slice(-12)
  const hit = EFFECTS.find((fx) => buffer.endsWith(fx.keys))
  if (hit) {
    buffer = ''
    trigger(hit)
  }
}

function resize() {
  const el = canvas.value
  if (!el) return
  const dpr = Math.min(window.devicePixelRatio || 1, 2)
  w = window.innerWidth
  h = window.innerHeight
  el.width = Math.round(w * dpr)
  el.height = Math.round(h * dpr)
  ctx = el.getContext('2d')
  ctx?.setTransform(dpr, 0, 0, dpr, 0, 0)
}

function update(dt: number) {
  clock += dt
  if (active) {
    active.elapsed += dt
    active.effect.tick(dt, active.elapsed)
    if (active.elapsed >= active.effect.seconds) active = null
  }
  // The overlay eases in and out with the effect.
  const wanted = active && active.effect.overlay ? 1 : 0
  overlayAlpha += (wanted - overlayAlpha) * Math.min(1, dt * 2.2)
  flash = Math.max(0, flash - dt * 3.2)

  for (let i = particles.length - 1; i >= 0; i--) {
    const p = particles[i]
    p.life += dt
    if (p.life >= p.max || p.y > h + 80 || p.x > w + 400 || p.x < -500) {
      particles.splice(i, 1)
      continue
    }
    p.x += p.vx * dt
    if (p.ground === undefined) p.y += p.vy * dt
    p.rot += p.vr * dt
  }
}

function drawRupee(p: Particle, a: number) {
  if (!ctx) return
  const s = p.size
  ctx.save()
  ctx.translate(p.x, p.y)
  ctx.rotate(p.rot)
  ctx.globalAlpha = a
  ctx.beginPath()
  ctx.moveTo(0, -s)
  ctx.lineTo(s * 0.62, -s * 0.38)
  ctx.lineTo(s * 0.62, s * 0.38)
  ctx.lineTo(0, s)
  ctx.lineTo(-s * 0.62, s * 0.38)
  ctx.lineTo(-s * 0.62, -s * 0.38)
  ctx.closePath()
  const g = ctx.createLinearGradient(-s, -s, s, s)
  g.addColorStop(0, `hsl(${p.hue}, 90%, 78%)`)
  g.addColorStop(0.5, `hsl(${p.hue}, 80%, 50%)`)
  g.addColorStop(1, `hsl(${p.hue}, 85%, 30%)`)
  ctx.fillStyle = g
  ctx.fill()
  ctx.strokeStyle = `hsla(${p.hue}, 90%, 85%, 0.9)`
  ctx.lineWidth = 1
  ctx.stroke()
  ctx.restore()
}

function drawOverlay(kind: Exclude<Overlay, null>, a: number) {
  if (!ctx || a < 0.01) return
  ctx.save()
  if (kind === 'sun') {
    const cx = w / 2
    const cy = h * 0.18
    const g = ctx.createRadialGradient(cx, cy, 0, cx, cy, Math.max(w, h))
    g.addColorStop(0, `rgba(255, 244, 190, ${0.85 * a})`)
    g.addColorStop(0.35, `rgba(255, 214, 90, ${0.38 * a})`)
    g.addColorStop(1, 'rgba(255, 190, 40, 0)')
    ctx.fillStyle = g
    ctx.fillRect(0, 0, w, h)
    // Slowly turning rays.
    ctx.globalCompositeOperation = 'lighter'
    ctx.translate(cx, cy)
    ctx.rotate(clock * 0.25)
    for (let i = 0; i < 14; i++) {
      ctx.rotate(TAU / 14)
      const rg = ctx.createLinearGradient(0, 0, Math.max(w, h), 0)
      rg.addColorStop(0, `rgba(255, 230, 140, ${0.22 * a})`)
      rg.addColorStop(1, 'rgba(255, 230, 140, 0)')
      ctx.fillStyle = rg
      ctx.beginPath()
      ctx.moveTo(0, 0)
      ctx.lineTo(Math.max(w, h), -60)
      ctx.lineTo(Math.max(w, h), 60)
      ctx.closePath()
      ctx.fill()
    }
  } else if (kind === 'storm') {
    ctx.fillStyle = `rgba(8, 14, 28, ${0.5 * a})`
    ctx.fillRect(0, 0, w, h)
  } else if (kind === 'ganon') {
    const pulse = 0.75 + 0.25 * Math.sin(clock * 3)
    const g = ctx.createRadialGradient(
      w / 2,
      h / 2,
      Math.min(w, h) * 0.2,
      w / 2,
      h / 2,
      Math.max(w, h) * 0.75,
    )
    g.addColorStop(0, `rgba(40, 0, 10, ${0.25 * a})`)
    g.addColorStop(1, `rgba(90, 0, 10, ${0.8 * a * pulse})`)
    ctx.fillStyle = g
    ctx.fillRect(0, 0, w, h)
  } else if (kind === 'saria') {
    const g = ctx.createRadialGradient(w / 2, h, 0, w / 2, h, Math.max(w, h))
    g.addColorStop(0, `rgba(90, 220, 120, ${0.3 * a})`)
    g.addColorStop(1, 'rgba(60, 200, 100, 0)')
    ctx.fillStyle = g
    ctx.fillRect(0, 0, w, h)
  }
  ctx.restore()
}

function draw() {
  if (!ctx) return
  ctx.clearRect(0, 0, w, h)
  const kind = active?.effect.overlay ?? lastOverlay
  if (active?.effect.overlay) lastOverlay = active.effect.overlay
  if (kind) drawOverlay(kind, overlayAlpha)

  for (const p of particles) {
    const f = 1 - p.life / p.max
    const a = Math.min(1, f * 3, p.life * 4 + 0.2)
    if (p.kind === 'rupee') drawRupee(p, a)
    else if (p.kind === 'drop') {
      if (!ctx) return
      ctx.strokeStyle = `rgba(170, 205, 255, ${0.55 * a})`
      ctx.lineWidth = 1.2
      ctx.beginPath()
      ctx.moveTo(p.x, p.y)
      ctx.lineTo(p.x + (p.vx / p.vy) * p.size, p.y - p.size)
      ctx.stroke()
    } else if (p.kind === 'spark') {
      ctx.save()
      ctx.globalCompositeOperation = 'lighter'
      const g = ctx.createRadialGradient(p.x, p.y, 0, p.x, p.y, p.size * 3)
      g.addColorStop(0, `hsla(${p.hue}, 100%, 85%, ${a})`)
      g.addColorStop(1, `hsla(${p.hue}, 100%, 55%, 0)`)
      ctx.fillStyle = g
      ctx.beginPath()
      ctx.arc(p.x, p.y, p.size * 3, 0, TAU)
      ctx.fill()
      ctx.restore()
    } else {
      ctx.save()
      ctx.globalAlpha = p.hue < 0 ? Math.min(1, f * 4) : a
      ctx.translate(p.x, p.y)
      if (p.hue >= 0) {
        ctx.rotate(Math.sin(p.rot * 2) * 0.35)
        ctx.fillStyle = `hsl(${p.hue}, 85%, 62%)`
        ctx.shadowColor = `hsl(${p.hue}, 100%, 60%)`
        ctx.shadowBlur = 12
      } else if (p.vx < 0) ctx.scale(-1, 1) // emoji face right; flip when running left
      ctx.font = `${p.size}px "Segoe UI Emoji", "Apple Color Emoji", "Noto Color Emoji", system-ui, sans-serif`
      ctx.textAlign = 'center'
      ctx.textBaseline = 'middle'
      ctx.fillText(p.glyph, 0, 0)
      ctx.restore()
    }
  }

  if (flash > 0.01) {
    ctx.fillStyle = `rgba(235, 242, 255, ${flash * 0.75})`
    ctx.fillRect(0, 0, w, h)
  }
}
let lastOverlay: Overlay = null

function frame(now: number) {
  const dt = Math.min(0.05, (now - last) / 1000 || 0.016)
  last = now
  update(dt)
  draw()
  if (!active && !particles.length && overlayAlpha < 0.01 && flash < 0.01) {
    ctx?.clearRect(0, 0, w, h)
    lastOverlay = null
    raf = 0
    return
  }
  raf = requestAnimationFrame(frame)
}

function onVisibility() {
  if (document.hidden && raf) {
    cancelAnimationFrame(raf)
    raf = 0
    active = null // do not resume a half-played effect later
    particles.length = 0
    overlayAlpha = 0
  }
}

onMounted(() => {
  reduce = window.matchMedia('(prefers-reduced-motion: reduce)').matches
  resize()
  window.addEventListener('keydown', onKey)
  window.addEventListener('resize', resize)
  document.addEventListener('visibilitychange', onVisibility)
})

onBeforeUnmount(() => {
  cancelAnimationFrame(raf)
  window.clearTimeout(bannerTimer)
  window.removeEventListener('keydown', onKey)
  window.removeEventListener('resize', resize)
  document.removeEventListener('visibilitychange', onVisibility)
})
</script>

<template>
  <canvas
    ref="canvas"
    class="pointer-events-none fixed inset-0 z-[88] size-full"
    aria-hidden="true"
  />
  <Transition name="egg">
    <div
      v-if="banner"
      role="status"
      class="pointer-events-none fixed inset-x-0 bottom-10 z-[89] flex justify-center px-4"
    >
      <p
        class="display border border-primary/60 bg-black/80 px-5 py-2 text-center text-2xl tracking-wider text-white shadow-[0_0_30px_rgba(61,220,255,0.35)] backdrop-blur-sm"
      >
        {{ banner }}
      </p>
    </div>
  </Transition>
</template>

<style scoped>
.egg-enter-active,
.egg-leave-active {
  transition:
    opacity 0.4s ease,
    transform 0.4s ease;
}
.egg-enter-from,
.egg-leave-to {
  opacity: 0;
  transform: translateY(12px);
}
</style>
