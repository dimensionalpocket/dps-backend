import { describe, it, expect, afterEach } from 'bun:test'
import { ApiError, apiFetch } from './client'

const originalFetch = globalThis.fetch

afterEach(() => {
  globalThis.fetch = originalFetch
})

function mockFetch(body: BodyInit | null, status: number, headers?: Record<string, string>) {
  globalThis.fetch = (async () =>
    new Response(body, { status, headers })) as typeof fetch
}

describe('apiFetch', () => {
  it('returns parsed JSON on success', async () => {
    mockFetch(JSON.stringify({ ok: true }), 200, { 'content-type': 'application/json' })

    const data = await apiFetch<{ ok: boolean }>('/example')

    expect(data.ok).toBe(true)
  })

  it('throws ApiError carrying the {error} envelope message', async () => {
    mockFetch(JSON.stringify({ error: 'Invalid credentials' }), 401)

    await expect(apiFetch('/example')).rejects.toThrow(ApiError)
    await apiFetch('/example').catch((error: ApiError) => {
      expect(error.status).toBe(401)
      expect(error.message).toBe('Invalid credentials')
    })
  })

  it('returns undefined for 204 responses', async () => {
    mockFetch(null, 204)

    expect(await apiFetch('/example')).toBeUndefined()
  })
})
