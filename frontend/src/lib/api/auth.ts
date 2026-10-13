import { ApiError, apiFetch } from './client'
import type { AuthLogin, AuthMe, AuthRegister, ChangePasswordResult } from './types'

export async function me(): Promise<AuthMe | null> {
  try {
    return await apiFetch<AuthMe>('/auth/me')
  } catch (error) {
    if (error instanceof ApiError && error.status === 401) return null
    throw error
  }
}

export function login(username: string, password: string): Promise<AuthLogin> {
  return apiFetch<AuthLogin>('/auth/login', {
    method: 'POST',
    body: JSON.stringify({ username, password }),
  })
}

export function register(
  username: string,
  password: string,
  passwordConfirmation: string,
): Promise<AuthRegister> {
  return apiFetch<AuthRegister>('/auth/register', {
    method: 'POST',
    body: JSON.stringify({ username, password, passwordConfirmation }),
  })
}

export function logout(): Promise<{ message: string }> {
  return apiFetch<{ message: string }>('/auth/logout', { method: 'POST' })
}

export function changePassword(
  currentPassword: string,
  newPassword: string,
  newPasswordConfirmation: string,
): Promise<ChangePasswordResult> {
  return apiFetch<ChangePasswordResult>('/auth/password', {
    method: 'PATCH',
    body: JSON.stringify({ currentPassword, newPassword, newPasswordConfirmation }),
  })
}
