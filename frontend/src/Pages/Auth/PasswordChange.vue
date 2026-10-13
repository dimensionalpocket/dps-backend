<script setup lang="ts">
import { ref } from 'vue'
import { Link, router } from '@inertiajs/vue3'
import Button from '@/components/ui/button/Button.vue'
import Card from '@/components/ui/card/Card.vue'
import CardContent from '@/components/ui/card/CardContent.vue'
import CardDescription from '@/components/ui/card/CardDescription.vue'
import CardHeader from '@/components/ui/card/CardHeader.vue'
import CardTitle from '@/components/ui/card/CardTitle.vue'
import Field from '@/components/ui/field/Field.vue'
import FieldGroup from '@/components/ui/field/FieldGroup.vue'
import FieldLabel from '@/components/ui/field/FieldLabel.vue'
import FieldError from '@/components/ui/field/FieldError.vue'
import Input from '@/components/ui/input/Input.vue'
import Spinner from '@/components/ui/spinner/Spinner.vue'
import { useAuthStore } from '@/stores/auth'
import { useToast } from '@/composables/useToast'
import { useAuthGuard } from '@/composables/useAuthGuard'

const authStore = useAuthStore()
const { success } = useToast()

useAuthGuard()

const loading = ref(false)
const currentPassword = ref('')
const newPassword = ref('')
const newPasswordConfirm = ref('')

async function onSubmit() {
  loading.value = true
  const ok = await authStore.changePassword(
    currentPassword.value,
    newPassword.value,
    newPasswordConfirm.value
  )
  if (ok) {
    success('Password changed successfully!')
    router.visit('/')
  } else {
    // Do not clear password fields on failure; store exposes error to display
  }
  loading.value = false
}
</script>

<template>
  <div class="w-full max-w-md">
    <Card>
      <CardHeader class="text-center">
        <CardTitle class="text-xl">
          Change Password
        </CardTitle>
        <CardDescription>
          Enter your current password and choose a new one
        </CardDescription>
      </CardHeader>
      <CardContent>
        <form @submit.prevent>
          <FieldGroup>
            <Field>
              <FieldLabel for="current-password">
                Current Password
              </FieldLabel>
              <Input
                id="current-password"
                name="current-password"
                type="password"
                required
                autocomplete="current-password"
                v-model="currentPassword"
                :disabled="loading"
                @keydown.enter.prevent="onSubmit"
              />
            </Field>
            <Field class="grid grid-cols-2 gap-4">
              <Field>
                <FieldLabel for="new-password">
                  New Password
                </FieldLabel>
                <Input
                  id="new-password"
                  name="new-password"
                  type="password"
                  required
                  autocomplete="new-password"
                  v-model="newPassword"
                  :disabled="loading"
                  @keydown.enter.prevent="onSubmit"
                />
              </Field>
              <Field>
                <FieldLabel for="confirm-new-password">
                  Confirm New Password
                </FieldLabel>
                <Input
                  id="confirm-new-password"
                  name="confirm-new-password"
                  type="password"
                  required
                  autocomplete="new-password"
                  v-model="newPasswordConfirm"
                  :disabled="loading"
                  @keydown.enter.prevent="onSubmit"
                />
              </Field>
            </Field>
            <Field>
              <Button :disabled="loading" type="button" @click="onSubmit">
                <template v-if="loading">
                  <Spinner class="h-4 w-4 mr-2 inline-block" />
                  Please wait…
                </template>
                <template v-else>
                  Change Password
                </template>
              </Button>
              <div class="mt-2 text-center">
                <Link
                  href="/"
                  class="text-sm text-muted-foreground hover:underline"
                >
                  Cancel
                </Link>
              </div>
              <FieldError v-if="authStore.changePasswordLastError" class="mt-2 text-center">
                {{ authStore.changePasswordLastError }}
              </FieldError>
            </Field>
          </FieldGroup>
        </form>
      </CardContent>
    </Card>
  </div>
</template>
