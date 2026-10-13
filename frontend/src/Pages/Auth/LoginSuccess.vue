<script setup lang="ts">
import Card from '@/components/ui/card/Card.vue'
import CardContent from '@/components/ui/card/CardContent.vue'
import CardHeader from '@/components/ui/card/CardHeader.vue'
import CardTitle from '@/components/ui/card/CardTitle.vue'
import Button from '@/components/ui/button/Button.vue'
import { router } from '@inertiajs/vue3'
import { ChevronLeft, Settings } from 'lucide-vue-next'
import { useAuthGuard } from '@/composables/useAuthGuard'

useAuthGuard()

function goBackToSite() {
  router.visit('/') // placeholder
}

function goAccount() {
  router.visit('/')
}
</script>

<template>
  <div class="w-full max-w-md">
    <Card>
      <CardHeader>
        <CardTitle class="text-center font-light">You are logged in.</CardTitle>
      </CardHeader>
      <CardContent>
        <div class="text-center py-4">
          <div class="flex gap-6 justify-center">
            <Button type="button" @click="goBackToSite">
              <ChevronLeft/>
              Return to Site
            </Button>
            <Button type="button" @click="goAccount">
              <Settings/>
              Account Settings
            </Button>
          </div>
        </div>
      </CardContent>
    </Card>
  </div>
</template>
