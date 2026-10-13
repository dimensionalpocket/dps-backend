import { describe, it, expect } from 'bun:test'
import { apiBase, resolveApiBase } from './config'

describe('apiBase', () => {
  it('defaults to /api', () => {
    expect(apiBase).toBe('/api')
  })
})

describe('resolveApiBase', () => {
  it('builds /api when VITE_DPS_API_PATH is api', () => {
    expect(resolveApiBase({ VITE_DPS_API_PATH: 'api' })).toBe('/api')
  })

  it('builds /v1 when VITE_DPS_API_PATH is v1', () => {
    expect(resolveApiBase({ VITE_DPS_API_PATH: 'v1' })).toBe('/v1')
  })
})
