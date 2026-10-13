import { onMounted } from 'vue'
import { router, usePage } from '@inertiajs/vue3'
import { useAuthStore } from '@/stores/auth'

export function useAuthGuard(options: { admin?: boolean } = {}) {
  const auth = useAuthStore()
  const page = usePage()
  onMounted(async () => {
    await auth.ensureSession()
    if (!auth.isAuthenticated) {
      router.visit(`/login?redirect=${encodeURIComponent(page.url)}`)
      return
    }
    if (options.admin && !auth.canAccessAdmin) router.visit('/')
  })
}
