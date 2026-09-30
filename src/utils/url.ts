/**
 * Turns a configured socket URL into an absolute ws(s) URL. A relative value such as "/ws"
 * (backend behind the same domain) follows the page protocol: https -> wss, http -> ws.
 */
export function resolveWsUrl(url: string, loc: { protocol: string; host: string }): string {
  if (!url.startsWith('/')) return url
  const scheme = loc.protocol === 'https:' ? 'wss:' : 'ws:'
  return `${scheme}//${loc.host}${url}`
}
