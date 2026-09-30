import { ref } from 'vue'

export interface Toast {
  id: number
  kind: 'success' | 'error' | 'info'
  message: string
}

const toasts = ref<Toast[]>([])
let nextId = 1

function push(kind: Toast['kind'], message: string, ms = 4500) {
  const id = nextId++
  toasts.value = [...toasts.value, { id, kind, message }]
  if (ms > 0) setTimeout(() => dismiss(id), ms)
}

function dismiss(id: number) {
  toasts.value = toasts.value.filter((t) => t.id !== id)
}

/** Small non-blocking notifications ("Guardado", "No se pudo…"). Shared by the whole panel. */
export function useToasts() {
  return {
    toasts,
    dismiss,
    success: (m: string) => push('success', m),
    info: (m: string) => push('info', m),
    error: (m: string) => push('error', m, 7000),
  }
}

/** Turns anything thrown into a sentence for a toast. */
export function messageOf(e: unknown): string {
  return e instanceof Error ? e.message : 'Ocurrió un error inesperado.'
}
