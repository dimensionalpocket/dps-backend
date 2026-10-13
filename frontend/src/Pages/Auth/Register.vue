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
import FieldDescription from '@/components/ui/field/FieldDescription.vue'
import FieldGroup from '@/components/ui/field/FieldGroup.vue'
import FieldLabel from '@/components/ui/field/FieldLabel.vue'
import FieldError from '@/components/ui/field/FieldError.vue'
import Input from '@/components/ui/input/Input.vue'
import Spinner from '@/components/ui/spinner/Spinner.vue'
import { useAuthStore } from '@/stores/auth'

const authStore = useAuthStore()

const username = ref('')
const password = ref('')
const passwordConfirm = ref('')

async function onSubmit () {
  const ok = await authStore.register(username.value, password.value, passwordConfirm.value)
  if (ok) {
    // Redirect to home on successful registration
    router.visit('/')
  } else {
    // Do not clear password fields on failure; store exposes authRegisterLastError to display
  }
}
</script>

<template>
  <div class="w-full max-w-md">
    <Card>
      <CardHeader class="text-center">
        <CardTitle class="text-xl">
          Create your account
        </CardTitle>
        <CardDescription>
          Complete the form below to create your account
        </CardDescription>
      </CardHeader>
      <CardContent>
        <form @submit.prevent>
          <FieldGroup>
            <Field>
              <FieldLabel for="username">
                Username
              </FieldLabel>
              <Input
                id="username"
                name="username"
                type="text"
                placeholder="username"
                required
                autocomplete="username"
                v-model="username"
                 :disabled="authStore.loading"
                @keydown.enter.prevent="onSubmit"
              />
              <!-- <FieldDescription>
                Must be at least 8 characters long.
              </FieldDescription> -->
            </Field>
            <Field>
              <Field class="grid grid-cols-2 gap-4">
                <Field>
                  <FieldLabel for="new-password">
                    Password
                  </FieldLabel>
                  <Input
                    id="new-password"
                    name="new-password"
                    type="password"
                    required
                    autocomplete="new-password"
                    v-model="password"
                    :disabled="authStore.loading"
                    @keydown.enter.prevent="onSubmit"
                  />
                </Field>
                <Field>
                  <FieldLabel for="confirm-password">
                    Confirm Password
                  </FieldLabel>
                  <Input
                    id="confirm-password"
                    name="confirm-password"
                    type="password"
                    required
                    autocomplete="new-password"
                    v-model="passwordConfirm"
                    :disabled="authStore.loading"
                    @keydown.enter.prevent="onSubmit"
                  />
                </Field>
              </Field>
              <!-- <FieldDescription>
                Must be at least 8 characters long.
              </FieldDescription> -->
            </Field>
            <Field>
              <Button :disabled="authStore.loading" type="button" @click="onSubmit">
                <template v-if="authStore.loading">
                  <Spinner class="h-4 w-4 mr-2 inline-block" />
                  Creating account…
                </template>
                <template v-else>
                  Create Account
                </template>
              </Button>
              <FieldDescription class="text-center">
                Already have an account? <Link href="/login" class="text-primary">Sign in</Link>
              </FieldDescription>
               <FieldError v-if="authStore.registerLastError" class="mt-2 text-center">
                 {{ authStore.registerLastError }}
              </FieldError>
            </Field>
          </FieldGroup>
        </form>
      </CardContent>
    </Card>
    <!-- <FieldDescription class="px-6 text-center">
      By clicking continue, you agree to our <a href="#">Terms of Service</a>
      and <a href="#">Privacy Policy</a>.
    </FieldDescription> -->
  </div>
</template>
