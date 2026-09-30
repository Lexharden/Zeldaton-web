import { describe, expect, it } from 'vitest'
import { resolveWsUrl } from './url'

describe('resolveWsUrl', () => {
  it('leaves absolute urls untouched', () => {
    expect(resolveWsUrl('ws://localhost:8080/ws', { protocol: 'http:', host: 'x' })).toBe(
      'ws://localhost:8080/ws',
    )
  })

  it('follows the page protocol for relative paths', () => {
    expect(resolveWsUrl('/ws', { protocol: 'https:', host: 'zeldathon.example.com' })).toBe(
      'wss://zeldathon.example.com/ws',
    )
    expect(resolveWsUrl('/ws', { protocol: 'http:', host: 'localhost:5173' })).toBe(
      'ws://localhost:5173/ws',
    )
  })
})
