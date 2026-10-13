import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  list as listUsers,
  remove as removeUser,
  update as updateUser,
} from '@/lib/api/users'
import type { User } from '@/lib/api/types'

export const useUserStore = defineStore('user', () => {
  const loading = ref(false)
  const users = ref<User[]>([])
  const lastError = ref<string | null>(null)

  async function fetch() {
    loading.value = true
    lastError.value = null

    try {
      users.value = await listUsers()
    } catch (error: any) {
      lastError.value = error.message || 'Failed to fetch users'
    } finally {
      loading.value = false
    }
  }

  // Load users immediately
  fetch()

  async function remove(id: number) {
    loading.value = true
    lastError.value = null

    try {
      const result = await removeUser(id)
      users.value = users.value.filter(user => user.id !== id)
      return result
    } catch (error: any) {
      lastError.value = error.message || 'Failed to remove user'
      throw error
    } finally {
      loading.value = false
    }
  }

  async function update(id: number, username?: string, roleId?: number, password?: string, passwordConfirmation?: string, metadataJson?: string) {
    loading.value = true
    lastError.value = null

    try {
      const updatedUser = await updateUser(id, {
        name: username,
        roleId,
        password,
        passwordConfirmation,
        metadataJson,
      })
      await fetch()
      return updatedUser
    } catch (error: any) {
      lastError.value = error.message || 'Failed to update user'
      throw error
    } finally {
      loading.value = false
    }
  }

  return {
    loading,
    users,
    lastError,
    fetch,
    remove,
    update,
  }
})
