<script setup lang="ts">
import { useAuthStore } from '@/stores/auth'
import { useAuthGuard } from '@/composables/useAuthGuard'

const authStore = useAuthStore()

useAuthGuard({ admin: true })
</script>

<template>
  <div>
    <h1 class="text-2xl font-bold mb-4">Admin Dashboard</h1>
    <div v-if="authStore.sessionData?.user?.role?.permissions?.length">
      <h2 class="text-lg font-semibold mb-2">Your Permissions:</h2>
      <ul class="list-disc list-inside space-y-1">
        <li v-for="permission in authStore.sessionData.user.role.permissions" :key="permission">
          {{ permission }}
        </li>
      </ul>
    </div>
  </div>
</template>
