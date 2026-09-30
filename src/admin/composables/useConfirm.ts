import { ref } from 'vue'

export interface ConfirmOptions {
  title: string
  message: string
  confirmLabel?: string
  /** Red button: for things that cannot be undone. */
  danger?: boolean
  /** The user must type this exact text to enable the button (e.g. the id being deleted). */
  requireText?: string
}

interface Pending extends ConfirmOptions {
  resolve: (ok: boolean) => void
}

export const pending = ref<Pending | null>(null)

/** Promise-based confirmation dialog: `if (await confirm({...})) { ... }`. Rendered by <ConfirmHost>. */
export function confirm(options: ConfirmOptions): Promise<boolean> {
  return new Promise((resolve) => {
    pending.value?.resolve(false)
    pending.value = { ...options, resolve }
  })
}

export function answer(ok: boolean) {
  pending.value?.resolve(ok)
  pending.value = null
}
