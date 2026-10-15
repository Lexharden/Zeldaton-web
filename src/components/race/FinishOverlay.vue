<script setup lang="ts">
import { t } from '@/i18n'
import { computed, nextTick, onBeforeUnmount, ref, watch } from 'vue'
import { Heart, X } from 'lucide-vue-next'
import { useRaceStore } from '@/stores/race'
import { useRacersStore } from '@/stores/racers'
import { primaryChannel } from '@/utils/channels'
import { formatDuration } from '@/utils/format'
import { P, playNotes } from '@/utils/ocarina'
import Avatar from '@/components/common/Avatar.vue'

/**
 * The FINISH LINE moment, shown when GAME_FINISHED arrives. A short staged ceremony:
 *
 *   0.0s  the room goes dark and a column of golden light falls from the sky
 *   0.8s  the Triforce ignites
 *   1.4s  the champion's emblem appears, crowned by a spinning ring
 *   2.0s  the name rises letter by letter while fireworks start
 *   3.6s  the final time counts up to the real one
 *   5.0s  the hearts fill, one by one
 *   6.2s  a few lines of thanks, then the links
 *
 * It is all CSS timing plus one canvas for fireworks and drifting fairies. With
 * prefers-reduced-motion the canvas never runs and everything is simply shown. The tune is the
 * Song of Time and a rising close; browsers may refuse sound until the page has been touched, and
 * "replay" always works because it is a click.
 */
const race = useRaceStore()
const racers = useRacersStore()
const racer = computed(() => racers.getById(race.winner?.racerId ?? ''))
const show = computed(() => !!race.winner && !race.winnerDismissed && !!racer.value)

/** Xime gets her own words; everybody else gets the same ceremony with a general dedication. */
const personal = computed(() => {
  const r = racer.value
  return !!r && /xime/i.test(`${r.displayName} ${r.slug} ${r.id}`)
})
const tributeKey = computed(() => (personal.value ? 'finish.xime' : 'finish.tribute'))

const letters = computed(() => Array.from(racer.value?.displayName ?? ''))
const finalSeconds = computed(() => race.winner?.finalTimeSeconds ?? 0)

const NBSP = String.fromCharCode(160)
const HEARTS = 7
const run = ref(0)
const shownSeconds = ref(0)
const canvas = ref<HTMLCanvasElement | null>(null)

const reduced = () =>
  typeof window !== 'undefined' && window.matchMedia?.('(prefers-reduced-motion: reduce)').matches

// ---- Final time counting up -------------------------------------------------------------------
let countRaf = 0
function countUp() {
  cancelAnimationFrame(countRaf)
  const target = finalSeconds.value
  if (reduced() || target <= 0) {
    shownSeconds.value = target
    return
  }
  shownSeconds.value = 0
  const delay = 3600
  const length = 2200
  const t0 = performance.now()
  const step = (now: number) => {
    const k = Math.min(1, Math.max(0, (now - t0 - delay) / length))
    shownSeconds.value = Math.round(target * (1 - Math.pow(1 - k, 3)))
    if (k < 1) countRaf = requestAnimationFrame(step)
  }
  countRaf = requestAnimationFrame(step)
}

// ---- Sound ------------------------------------------------------------------------------------
function sing() {
  const A5 = 880
  playNotes(
    [
      [P.A4, 0.55],
      [P.D4, 0.95],
      [P.F4, 0.55],
      [P.A4, 0.55],
      [P.D4, 0.95],
      [P.F4, 0.55],
      [0, 0.15],
      [P.D5, 0.22],
      [P.F5, 0.22],
      [A5, 1.4],
    ],
    { volume: 0.14 },
  )
}

// ---- Fireworks and fairies --------------------------------------------------------------------
interface Spark {
  x: number
  y: number
  vx: number
  vy: number
  life: number
  max: number
  size: number
  hue: number
  /** Fairies drift and blink instead of falling. */
  fairy?: boolean
  /** A rocket climbing before it bursts. */
  rocket?: boolean
}

let raf = 0
let sparks: Spark[] = []
let w = 0
let h = 0
let startAt = 0
let nextRocket = 0
let last = 0

function fit() {
  const c = canvas.value
  if (!c) return
  const dpr = Math.min(2, window.devicePixelRatio || 1)
  w = c.clientWidth
  h = c.clientHeight
  c.width = Math.round(w * dpr)
  c.height = Math.round(h * dpr)
  c.getContext('2d')?.setTransform(dpr, 0, 0, dpr, 0, 0)
}

function burst(x: number, y: number, hue: number) {
  const n = 46 + Math.floor(Math.random() * 20)
  for (let i = 0; i < n; i++) {
    const a = (i / n) * Math.PI * 2 + Math.random() * 0.2
    const v = 90 + Math.random() * 150
    sparks.push({
      x,
      y,
      vx: Math.cos(a) * v,
      vy: Math.sin(a) * v,
      life: 0,
      max: 1.3 + Math.random() * 0.9,
      size: 1.6 + Math.random() * 1.6,
      hue: hue + Math.random() * 24 - 12,
    })
  }
}

function frame(now: number) {
  const c = canvas.value
  const g = c?.getContext('2d')
  if (!c || !g) return
  const dt = Math.min(0.05, (now - last) / 1000)
  last = now
  const elapsed = (now - startAt) / 1000
  g.clearRect(0, 0, w, h)

  // Gold, green and blue: the colors of the game, not a generic confetti rainbow.
  if (elapsed > 2 && now > nextRocket) {
    sparks.push({
      x: w * (0.15 + Math.random() * 0.7),
      y: h,
      vx: (Math.random() - 0.5) * 40,
      vy: -(h * (0.75 + Math.random() * 0.35)),
      life: 0,
      max: 0.7 + Math.random() * 0.35,
      size: 2.4,
      hue: [45, 45, 130, 205][Math.floor(Math.random() * 4)]!,
      rocket: true,
    })
    nextRocket = now + 380 + Math.random() * 520
  }
  if (elapsed > 1.2 && sparks.length < 420 && Math.random() < dt * 9) {
    sparks.push({
      x: Math.random() * w,
      y: h + 10,
      vx: (Math.random() - 0.5) * 26,
      vy: -(18 + Math.random() * 34),
      life: 0,
      max: 7 + Math.random() * 5,
      size: 2.2 + Math.random() * 2.2,
      hue: [48, 100, 190][Math.floor(Math.random() * 3)]!,
      fairy: true,
    })
  }

  const next: Spark[] = []
  for (const s of sparks) {
    s.life += dt
    if (s.life >= s.max) {
      if (s.rocket) burst(s.x, s.y, s.hue)
      continue
    }
    const k = s.life / s.max
    if (s.fairy) {
      s.x += (s.vx + Math.sin(s.life * 2.3 + s.hue) * 22) * dt
      s.y += s.vy * dt
      const blink = 0.55 + 0.45 * Math.sin(s.life * 5 + s.hue)
      const a = Math.min(1, k * 6) * (1 - k) * blink
      const grad = g.createRadialGradient(s.x, s.y, 0, s.x, s.y, s.size * 5)
      grad.addColorStop(0, `hsla(${s.hue} 95% 85% / ${a})`)
      grad.addColorStop(1, `hsla(${s.hue} 95% 60% / 0)`)
      g.fillStyle = grad
      g.fillRect(s.x - s.size * 5, s.y - s.size * 5, s.size * 10, s.size * 10)
    } else if (s.rocket) {
      s.x += s.vx * dt
      s.y += s.vy * dt * (1 - k * 0.7)
      g.fillStyle = `hsla(${s.hue} 100% 80% / 0.95)`
      g.beginPath()
      g.arc(s.x, s.y, s.size, 0, Math.PI * 2)
      g.fill()
    } else {
      s.vy += 120 * dt
      s.vx *= 1 - dt * 0.9
      s.vy *= 1 - dt * 0.9
      s.x += s.vx * dt
      s.y += s.vy * dt
      g.fillStyle = `hsla(${s.hue} 100% ${70 - k * 20}% / ${1 - k})`
      g.beginPath()
      g.arc(s.x, s.y, s.size * (1 - k * 0.5), 0, Math.PI * 2)
      g.fill()
    }
    next.push(s)
  }
  sparks = next
  raf = requestAnimationFrame(frame)
}

function stop() {
  cancelAnimationFrame(raf)
  cancelAnimationFrame(countRaf)
  raf = 0
  sparks = []
}

async function begin() {
  stop()
  countUp()
  await nextTick()
  if (reduced() || !canvas.value) return
  fit()
  startAt = last = performance.now()
  nextRocket = startAt
  raf = requestAnimationFrame(frame)
}

function replay() {
  run.value++
  sing()
  void begin()
}

watch(
  show,
  (on) => {
    if (on) {
      sing()
      void begin()
    } else stop()
  },
  { immediate: true },
)
onBeforeUnmount(stop)
</script>

<template>
  <Transition name="page">
    <div
      v-if="show && racer"
      class="finish fixed inset-0 z-[70] overflow-y-auto overflow-x-hidden"
      role="dialog"
      aria-modal="true"
      :aria-label="t('finish.aria')"
      @vue:mounted="fit"
    >
      <div :key="run" class="finish-stage relative grid min-h-full place-items-center px-4 py-10">
        <div class="finish-bg" aria-hidden="true" />
        <div class="finish-rays" aria-hidden="true" />
        <div class="finish-beam" aria-hidden="true" />
        <canvas
          ref="canvas"
          class="pointer-events-none absolute inset-0 size-full"
          aria-hidden="true"
        />

        <button
          type="button"
          class="fixed right-4 top-4 z-10 text-muted hover:text-white"
          :aria-label="t('finish.close')"
          @click="race.winnerDismissed = true"
        >
          <X class="size-7" />
        </button>

        <div class="relative z-[1] flex w-full max-w-3xl flex-col items-center text-center">
          <svg class="triforce" viewBox="0 0 100 88" width="76" height="67" aria-hidden="true">
            <defs>
              <linearGradient id="tf-gold" x1="0" y1="0" x2="0" y2="1">
                <stop offset="0" stop-color="#fff3b0" />
                <stop offset="1" stop-color="#e0a21f" />
              </linearGradient>
            </defs>
            <polygon points="50,2 74,44 26,44" fill="url(#tf-gold)" />
            <polygon points="26,44 50,86 2,86" fill="url(#tf-gold)" />
            <polygon points="74,44 98,86 50,86" fill="url(#tf-gold)" />
          </svg>

          <div class="champion">
            <div class="champion-ring" aria-hidden="true" />
            <Avatar
              :name="racer.displayName"
              :src="racer.avatarUrl"
              :seed="racer.id"
              :size="132"
              class="champion-avatar"
            />
          </div>

          <p class="hud-label rise !text-accent" style="--d: 1.7s">
            {{ t('finish.first') }} · {{ t('finish.line') }}
          </p>

          <h2 class="display name mt-3 text-6xl sm:text-8xl" :aria-label="racer.displayName">
            <span
              v-for="(ch, i) in letters"
              :key="i"
              class="letter text-gold"
              aria-hidden="true"
              :style="{ '--d': `${2 + i * 0.07}s` }"
              >{{ ch === ' ' ? NBSP : ch }}</span
            >
          </h2>

          <p
            class="rise mt-4 font-display text-xl font-semibold uppercase tracking-widest text-white sm:text-2xl"
            style="--d: 3s"
          >
            {{ t('finish.completed') }}
          </p>

          <div v-if="finalSeconds" class="rise mt-7" style="--d: 3.4s">
            <p class="hud-label">{{ t('finish.finalTime') }}</p>
            <p class="num time mt-1 text-5xl font-bold text-white sm:text-6xl">
              {{ formatDuration(shownSeconds) }}
            </p>
          </div>

          <div class="mt-7 flex gap-1.5" aria-hidden="true">
            <Heart
              v-for="n in HEARTS"
              :key="n"
              class="heart size-7 fill-magenta text-magenta"
              :style="{ '--d': `${5 + n * 0.16}s` }"
            />
          </div>

          <div class="mt-8 max-w-xl space-y-2 text-base text-text sm:text-lg">
            <p
              v-for="n in 3"
              :key="n"
              class="rise"
              :class="n === 3 ? 'font-semibold text-accent' : 'text-text/90'"
              :style="{ '--d': `${6.4 + (n - 1) * 1.3}s` }"
            >
              {{ t(`${tributeKey}.l${n}`, { name: racer.displayName }) }}
            </p>
          </div>

          <div
            class="rise mt-9 flex flex-wrap items-center justify-center gap-3"
            style="--d: 10.2s"
          >
            <a
              v-if="primaryChannel(racer)"
              :href="primaryChannel(racer)?.url"
              target="_blank"
              rel="noopener noreferrer"
              class="btn btn-gold"
              >{{ t('finish.celebration') }}</a
            >
            <button type="button" class="btn" @click="replay">{{ t('finish.replay') }}</button>
          </div>
        </div>
      </div>
    </div>
  </Transition>
</template>

<style scoped>
.finish {
  background: #03070a;
}
.finish-bg {
  position: absolute;
  inset: 0;
  background:
    radial-gradient(60% 50% at 50% 38%, rgb(245 196 81 / 0.22), transparent 70%),
    radial-gradient(90% 70% at 50% 100%, rgb(76 154 82 / 0.2), transparent 70%);
  opacity: 0;
  animation: fin 2.4s ease 0.5s forwards;
}
.finish-rays {
  position: absolute;
  left: 50%;
  top: 34%;
  width: 190vmax;
  height: 190vmax;
  translate: -50% -50%;
  background: repeating-conic-gradient(
    from 0deg,
    rgb(245 196 81 / 0.11) 0deg 6deg,
    transparent 6deg 18deg
  );
  mask-image: radial-gradient(circle, #000 0%, transparent 38%);
  opacity: 0;
  animation:
    fin 3s ease 1s forwards,
    spin-slow 90s linear infinite;
}
.finish-beam {
  position: absolute;
  left: 50%;
  top: 0;
  width: min(34vw, 15rem);
  height: 60%;
  translate: -50% 0;
  background: linear-gradient(
    to bottom,
    rgb(255 244 190 / 0.85),
    rgb(245 196 81 / 0.28) 55%,
    transparent
  );
  clip-path: polygon(40% 0, 60% 0, 100% 100%, 0 100%);
  filter: blur(6px);
  transform-origin: top;
  opacity: 0;
  animation: beam 2.6s ease-out 0.1s forwards;
}
.triforce {
  filter: drop-shadow(0 0 14px rgb(245 196 81 / 0.9));
  opacity: 0;
  animation:
    triforce-in 1.1s cubic-bezier(0.2, 0.8, 0.2, 1) 0.8s forwards,
    glow 2.8s ease-in-out 2s infinite;
}
.champion {
  position: relative;
  margin-top: 1.1rem;
  margin-bottom: 1.4rem;
  width: 168px;
  height: 168px;
  display: grid;
  place-items: center;
}
.champion-avatar {
  opacity: 0;
  animation: pop 1s cubic-bezier(0.2, 1.3, 0.3, 1) 1.4s forwards;
  filter: drop-shadow(0 0 22px rgb(245 196 81 / 0.55));
}
.champion-ring {
  position: absolute;
  inset: 0;
  border-radius: 9999px;
  background: conic-gradient(
    from 0deg,
    transparent,
    rgb(245 196 81 / 0.95),
    transparent 40%,
    rgb(255 244 190 / 0.8) 60%,
    transparent
  );
  mask-image: radial-gradient(circle, transparent 66%, #000 68%, #000 74%, transparent 76%);
  opacity: 0;
  animation:
    fin 1.2s ease 1.6s forwards,
    spin-slow 7s linear infinite;
}
.name {
  filter: drop-shadow(0 0 24px rgb(245 196 81 / 0.45));
}
.letter {
  display: inline-block;
  opacity: 0;
  transform: translateY(0.5em) scale(0.6);
  animation: letter 0.7s cubic-bezier(0.2, 1.2, 0.3, 1) var(--d) forwards;
}
.rise {
  opacity: 0;
  animation: rise 0.9s ease var(--d) forwards;
}
.time {
  text-shadow: 0 0 22px rgb(245 196 81 / 0.35);
}
.heart {
  opacity: 0;
  transform: scale(0.2);
  animation:
    heart-in 0.6s cubic-bezier(0.2, 1.5, 0.3, 1) var(--d) forwards,
    beat 1.6s ease-in-out calc(var(--d) + 1.5s) infinite;
  filter: drop-shadow(0 0 6px rgb(214 74 58 / 0.7));
}

@keyframes fin {
  to {
    opacity: 1;
  }
}
@keyframes beam {
  0% {
    opacity: 0;
    transform: scaleY(0);
  }
  35% {
    opacity: 1;
    transform: scaleY(1);
  }
  100% {
    opacity: 0.35;
    transform: scaleY(1);
  }
}
@keyframes triforce-in {
  from {
    opacity: 0;
    transform: translateY(-30px) scale(0.5);
  }
  to {
    opacity: 1;
    transform: none;
  }
}
@keyframes glow {
  50% {
    filter: drop-shadow(0 0 26px rgb(255 232 140 / 1));
  }
}
@keyframes pop {
  from {
    opacity: 0;
    transform: scale(0.3) rotate(-14deg);
  }
  to {
    opacity: 1;
    transform: none;
  }
}
@keyframes letter {
  to {
    opacity: 1;
    transform: none;
  }
}
@keyframes heart-in {
  to {
    opacity: 1;
    transform: scale(1);
  }
}
@keyframes beat {
  0%,
  100% {
    transform: scale(1);
  }
  50% {
    transform: scale(1.14);
  }
}

@media (prefers-reduced-motion: reduce) {
  .finish-bg,
  .finish-rays,
  .triforce,
  .champion-avatar,
  .champion-ring,
  .letter,
  .rise,
  .heart {
    opacity: 1;
    transform: none;
  }
  .finish-beam {
    opacity: 0.35;
  }
}
</style>
