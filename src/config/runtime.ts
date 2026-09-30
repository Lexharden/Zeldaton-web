import { resolveWsUrl } from '@/utils/url'

const env = import.meta.env

const flag = (value: string | undefined, fallback: boolean) =>
  value === undefined || value === '' ? fallback : value === 'true'

/** Single place where environment configuration is read. */
export const runtime = {
  apiUrl: env.VITE_API_URL ?? '',
  wsUrl:
    typeof window === 'undefined'
      ? (env.VITE_WS_URL ?? '')
      : resolveWsUrl(env.VITE_WS_URL ?? '', window.location),
  /** Mock mode is the default; the real backend is used only when explicitly disabled. */
  mockMode: flag(env.VITE_MOCK_MODE, true),
  demoMode: flag(env.VITE_DEMO_MODE, true),
  requestTimeoutMs: 8000,
  ws: {
    maxAttempts: 8,
    baseDelayMs: 1000,
    maxDelayMs: 20000,
    heartbeatMs: 15000,
    staleAfterMs: 45000,
  },
  /** How often the client asks the backend for a fresh clock snapshot. */
  clockResyncMs: 30000,
  demoTickMs: 3200,
} as const
