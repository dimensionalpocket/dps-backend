<script setup lang="ts">
import Card from '@/components/ui/card/Card.vue'
import CardContent from '@/components/ui/card/CardContent.vue'
import CardHeader from '@/components/ui/card/CardHeader.vue'
import CardTitle from '@/components/ui/card/CardTitle.vue'

defineProps<{ status?: number }>()
</script>

<template>
  <div class="w-full max-w-md">
    <Card>
      <CardHeader>
        <CardTitle class="text-center">{{ status ?? 404 }}</CardTitle>
      </CardHeader>
      <CardContent>
        <p class="text-center text-muted-foreground">
          The page you are looking for could not be found.
        </p>
      </CardContent>
    </Card>
  </div>
</template>
