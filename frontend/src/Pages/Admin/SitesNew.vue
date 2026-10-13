<script setup lang="ts">
import { ref } from 'vue'
import { router } from '@inertiajs/vue3'
import { storeToRefs } from 'pinia'
import { useSiteStore } from '@/stores/site'
import { useAuthGuard } from '@/composables/useAuthGuard'
import { Button } from '@/components/ui/button'
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

const siteStore = useSiteStore()
const { loading } = storeToRefs(siteStore)

useAuthGuard({ admin: true })

const slug = ref('')
const subdomain = ref('')
const port = ref<number | undefined>(undefined)
const protocol = ref('')

async function onSubmit() {
  try {
    await siteStore.add(
      slug.value,
      subdomain.value || undefined,
      port.value,
      protocol.value || undefined
    )
    await siteStore.fetch()
    router.visit('/admin/sites')
  } catch (error) {
    // Error is handled by the store and displayed via FieldError
  }
}

function onCancel() {
  router.visit('/admin/sites')
}
</script>

<template>
  <div class="w-full">
    <Card>
      <CardHeader>
        <CardTitle>Add New Site</CardTitle>
        <CardDescription>
          Enter the site details below to create a new site
        </CardDescription>
      </CardHeader>
      <CardContent>
        <form @submit.prevent>
          <FieldGroup>
            <Field>
              <FieldLabel for="slug">
                Slug *
              </FieldLabel>
              <Input
                id="slug"
                name="slug"
                type="text"
                placeholder="site-slug"
                required
                v-model="slug"
                :disabled="loading"
                @keydown.enter.prevent="onSubmit"
              />
            </Field>
            <Field>
              <FieldLabel for="subdomain">
                Subdomain
              </FieldLabel>
              <Input
                id="subdomain"
                name="subdomain"
                type="text"
                placeholder="subdomain"
                v-model="subdomain"
                :disabled="loading"
                @keydown.enter.prevent="onSubmit"
              />
            </Field>
            <Field>
              <FieldLabel for="port">
                Port
              </FieldLabel>
              <Input
                id="port"
                name="port"
                type="number"
                placeholder="8080"
                v-model.number="port"
                :disabled="loading"
                @keydown.enter.prevent="onSubmit"
              />
            </Field>
            <Field>
              <FieldLabel for="protocol">
                Protocol
              </FieldLabel>
              <Input
                id="protocol"
                name="protocol"
                type="text"
                placeholder="https"
                v-model="protocol"
                :disabled="loading"
                @keydown.enter.prevent="onSubmit"
              />
            </Field>
            <Field>
              <div class="flex gap-2">
                <Button :disabled="loading" type="button" @click="onSubmit">
                  <template v-if="loading">
                    <Spinner class="h-4 w-4 mr-2 inline-block" />
                    Adding Site…
                  </template>
                  <template v-else>
                    Add Site
                  </template>
                </Button>
                <Button variant="outline" type="button" @click="onCancel" :disabled="loading">
                  Cancel
                </Button>
              </div>
              <FieldError v-if="siteStore.lastError" class="mt-2">
                {{ siteStore.lastError }}
              </FieldError>
            </Field>
          </FieldGroup>
        </form>
      </CardContent>
    </Card>
  </div>
</template>
