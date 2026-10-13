import { apiFetch } from './client'
import type { User } from './types'

export interface UpdateUserInput {
  name?: string
  roleId?: number
  password?: string
  passwordConfirmation?: string
  metadataJson?: string
}

export function list(): Promise<User[]> {
  return apiFetch<User[]>('/users')
}

export function update(id: number, input: UpdateUserInput): Promise<User> {
  return apiFetch<User>(`/users/${id}`, {
    method: 'PATCH',
    body: JSON.stringify(input),
  })
}

export function remove(id: number): Promise<{ success: boolean }> {
  return apiFetch<{ success: boolean }>(`/users/${id}`, { method: 'DELETE' })
}
