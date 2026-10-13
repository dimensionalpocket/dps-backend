import { apiFetch } from './client'
import type { Site } from './types'

export interface CreateSiteInput {
  slug: string
  subdomain?: string
  port?: number
  protocol?: string
  metadataJson?: string
}

export type UpdateSiteInput = Partial<CreateSiteInput>

export function list(): Promise<Site[]> {
  return apiFetch<Site[]>('/sites')
}

export function create(input: CreateSiteInput): Promise<Site> {
  return apiFetch<Site>('/sites', {
    method: 'POST',
    body: JSON.stringify(input),
  })
}

export function update(id: number, input: UpdateSiteInput): Promise<Site> {
  return apiFetch<Site>(`/sites/${id}`, {
    method: 'PATCH',
    body: JSON.stringify(input),
  })
}

export function remove(id: number): Promise<Site> {
  return apiFetch<Site>(`/sites/${id}`, { method: 'DELETE' })
}
