import { DpsConfig } from '@dimensionalpocket/dps-config'

/**
 * Resolves the same-origin REST API base path from an environment object.
 *
 * Only `api_path` is used; the API lives on the same origin as the app.
 */
export function resolveApiBase(env: Record<string, string | undefined>): string {
  const config = new DpsConfig(env, 'VITE_')
  return `/${config.getApiPath()}`
}

export const apiBase = resolveApiBase(import.meta.env)
