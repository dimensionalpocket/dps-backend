import { router } from '@inertiajs/vue3'
import { useAuthStore } from '@/stores/auth'

export const logoutWithToast = async (toast: any) => {
  const authStore = useAuthStore()

  try {
    await authStore.logout()
    toast.success('Logged out successfully')
  } catch (error) {
    console.error('Logout failed:', error)
  } finally {
    // Redirect to login page after logout
    router.visit('/login')
  }
}
