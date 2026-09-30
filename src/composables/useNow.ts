import { getCurrentScope, onScopeDispose, readonly, ref } from 'vue'

/**
 * One shared 1s ticker for the whole app. Components that need "now" subscribe here instead
 * of creating their own interval; the timer runs only while somebody is subscribed.
 */
const now = ref(Date.now())
const monotonic = ref(performance.now())
let subscribers = 0
let timer: ReturnType<typeof setInterval> | null = null

function tick() {
  now.value = Date.now()
  monotonic.value = performance.now()
}

export function useNow() {
  if (getCurrentScope()) {
    subscribers += 1
    if (!timer) {
      tick()
      timer = setInterval(tick, 1000)
    }
    onScopeDispose(() => {
      subscribers -= 1
      if (subscribers <= 0 && timer) {
        clearInterval(timer)
        timer = null
        subscribers = 0
      }
    })
  }
  return { now: readonly(now), monotonic: readonly(monotonic) }
}
