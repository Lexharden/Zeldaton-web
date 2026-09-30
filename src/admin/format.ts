/** Formatting helpers for the organizer panel (Spanish, the organizers' language). */

const pad = (n: number) => String(Math.floor(n)).padStart(2, '0')

/** 12345 -> "03:25:45". */
export function hms(totalSeconds: number): string {
  const s = Math.max(0, Math.floor(totalSeconds))
  return `${pad(s / 3600)}:${pad((s % 3600) / 60)}:${pad(s % 60)}`
}

/** "hace 3 s", "hace 5 min", "hace 2 h". */
export function ago(seconds: number | null | undefined): string {
  if (seconds === null || seconds === undefined) return '—'
  if (seconds < 5) return 'ahora'
  if (seconds < 60) return `hace ${Math.floor(seconds)} s`
  if (seconds < 3600) return `hace ${Math.floor(seconds / 60)} min`
  return `hace ${Math.floor(seconds / 3600)} h`
}

/** Local date and time of an ISO instant, e.g. "7 oct, 13:05:09". */
export function localDateTime(iso: string | null | undefined): string {
  if (!iso) return '—'
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return '—'
  return new Intl.DateTimeFormat('es-MX', {
    day: 'numeric',
    month: 'short',
    hour: '2-digit',
    minute: '2-digit',
    second: '2-digit',
    hourCycle: 'h23',
  }).format(d)
}

/** ISO instant -> value for `<input type="datetime-local">` in the browser's timezone. */
export function isoToLocalInput(iso: string | undefined): string {
  if (!iso) return ''
  const d = new Date(iso)
  if (Number.isNaN(d.getTime())) return ''
  return `${d.getFullYear()}-${pad(d.getMonth() + 1)}-${pad(d.getDate())}T${pad(d.getHours())}:${pad(d.getMinutes())}`
}

/** Value of `<input type="datetime-local">` (browser timezone) -> ISO instant in UTC. */
export function localInputToIso(value: string): string | undefined {
  if (!value) return undefined
  const d = new Date(value)
  return Number.isNaN(d.getTime()) ? undefined : d.toISOString()
}

/** "2h 30m" from seconds. */
export function shortDuration(totalSeconds: number): string {
  const s = Math.max(0, Math.floor(totalSeconds))
  const h = Math.floor(s / 3600)
  const m = Math.floor((s % 3600) / 60)
  return h ? `${h} h${m ? ` ${m} min` : ''}` : `${m} min`
}

/** Readable text for an audit action code, e.g. "racer.adjust-time" -> "Ajustar tiempo". */
const ACTIONS: Record<string, string> = {
  'auth.login': 'Inició sesión',
  'auth.login.failed': 'Intento de acceso fallido',
  'auth.logout': 'Cerró sesión',
  'event.start': 'Inició el evento',
  'event.pause': 'Pausó el evento',
  'event.resume': 'Reanudó el evento',
  'event.finish': 'Terminó el evento',
  'event.update': 'Editó el evento',
  'racer.create': 'Creó un corredor',
  'racer.update': 'Editó un corredor',
  'racer.delete': 'Eliminó un corredor',
  'racer.token.rotate': 'Rotó el token',
  'racer.pause': 'Pausó a un corredor',
  'racer.resume': 'Reanudó a un corredor',
  'racer.force-close': 'Cerró el juego de un corredor',
  'racer.reset-day': 'Reinició el día de un corredor',
  'racer.adjust-time': 'Ajustó el tiempo de un corredor',
  'racer.finish': 'Marcó que un corredor terminó',
  'user.create': 'Creó una cuenta',
  'user.update': 'Editó una cuenta',
  'user.delete': 'Eliminó una cuenta',
  'user.password.change': 'Cambió su contraseña',
  'user.password.reset': 'Restableció una contraseña',
  'catalog.item.save': 'Guardó un ítem',
  'catalog.item.delete': 'Eliminó un ítem',
  'catalog.objective.save': 'Guardó un objetivo',
  'catalog.objective.delete': 'Eliminó un objetivo',
}

export const actionLabel = (code: string) => ACTIONS[code] ?? code
