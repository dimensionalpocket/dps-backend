<script setup lang="ts">
import { router } from '@inertiajs/vue3'
import { useAuthStore } from '@/stores/auth'
import { useAuthGuard } from '@/composables/useAuthGuard'
import { KeyRound, LogOut } from 'lucide-vue-next'
import Button from '@/components/ui/button/Button.vue'
import Card from '@/components/ui/card/Card.vue'
import CardContent from '@/components/ui/card/CardContent.vue'
import CardHeader from '@/components/ui/card/CardHeader.vue'
import CardTitle from '@/components/ui/card/CardTitle.vue'
import Spinner from '@/components/ui/spinner/Spinner.vue'
import { logoutWithToast } from '@/lib/logout'
import { useToast } from '@/composables/useToast'

const authStore = useAuthStore()
const { success } = useToast()

useAuthGuard()

function handleChangePassword() {
  router.visit('/password-change')
}

async function handleLogout() {
  await logoutWithToast({ success })
}
</script>

<template>
  <div class="w-full max-w-md">
    <Card>
      <CardHeader>
        <CardTitle>Welcome back, {{ authStore.sessionData?.user?.name }}</CardTitle>
      </CardHeader>
      <CardContent>
        <div class="flex flex-col sm:flex-row gap-2">
          <Button
            @click="handleChangePassword"
            class="flex-1"
            :disabled="authStore.loading"
          >
            <KeyRound class="w-4 h-4 mr-2" />
            Change password
          </Button>
          <Button
            variant="destructive"
            @click="handleLogout"
            class="flex-1"
            :disabled="authStore.loading"
          >
            <Spinner v-if="authStore.loading" class="w-4 h-4 mr-2" />
            <LogOut v-else class="w-4 h-4 mr-2" />
            {{ authStore.loading ? 'Logging out...' : 'Log out' }}
          </Button>
        </div>
      </CardContent>
    </Card>
  </div>
</template>
