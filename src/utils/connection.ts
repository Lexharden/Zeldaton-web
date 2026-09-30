import type { ConnectionQuality, ConnectionStatus } from '@/types/websocket'

/** Quality is judged by message freshness and retry history, not by wall-clock skew. */
export function connectionQuality(
  status: ConnectionStatus,
  msSinceLastMessage: number | null,
  staleAfterMs: number,
): ConnectionQuality {
  if (status !== 'connected') return 'unknown'
  if (msSinceLastMessage === null) return 'good'
  if (msSinceLastMessage > staleAfterMs) return 'poor'
  if (msSinceLastMessage > staleAfterMs / 2) return 'degraded'
  return 'good'
}
