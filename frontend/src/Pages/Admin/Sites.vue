<script setup lang="ts">
import { ref, computed } from 'vue'
import { storeToRefs } from 'pinia'
import { router } from '@inertiajs/vue3'
import { useSiteStore } from '@/stores/site'
import { useAuthGuard } from '@/composables/useAuthGuard'
import { Button } from '@/components/ui/button'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import Dialog from '@/components/ui/dialog/Dialog.vue'
import DialogContent from '@/components/ui/dialog/DialogContent.vue'
import DialogDescription from '@/components/ui/dialog/DialogDescription.vue'
import DialogFooter from '@/components/ui/dialog/DialogFooter.vue'
import DialogHeader from '@/components/ui/dialog/DialogHeader.vue'
import DialogTitle from '@/components/ui/dialog/DialogTitle.vue'
import { Pencil, Trash2 } from 'lucide-vue-next'

const siteStore = useSiteStore()
const { sites, loading } = storeToRefs(siteStore)

useAuthGuard({ admin: true })

const deleteDialogOpen = ref(false)
const siteToDelete = ref<number | null>(null)

const siteToDeleteData = computed(() => {
  if (siteToDelete.value === null) return null
  return sites.value.find(site => site.id === siteToDelete.value)
})

const handleEditSite = (siteId: number) => {
  router.visit(`/admin/sites/${siteId}/edit`)
}

const handleDeleteSite = (siteId: number) => {
  siteToDelete.value = siteId
  deleteDialogOpen.value = true
}

const confirmDelete = async () => {
  if (siteToDelete.value === null) return

  try {
    await siteStore.remove(siteToDelete.value)
    deleteDialogOpen.value = false
    siteToDelete.value = null
  } catch (error) {
    // Error is handled by the store
  }
}

const cancelDelete = () => {
  deleteDialogOpen.value = false
  siteToDelete.value = null
}

const handleAddSite = () => {
  router.visit('/admin/sites/new')
}
</script>

<template>
  <div class="w-full">
    <div class="flex justify-between items-center mb-6">
      <h1 class="text-2xl font-bold">Sites</h1>
      <Button @click="handleAddSite">Add Site</Button>
    </div>

    <div v-if="loading" class="text-center py-8">
      Loading sites...
    </div>

    <Table v-else>
      <TableHeader>
        <TableRow>
          <TableHead>Slug</TableHead>
          <TableHead>Subdomain</TableHead>
          <TableHead>Port</TableHead>
          <TableHead>Protocol</TableHead>
          <TableHead class="text-right">Actions</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        <TableRow v-for="site in sites" :key="site.id">
          <TableCell class="font-medium">{{ site.slug }}</TableCell>
          <TableCell>{{ site.subdomain }}</TableCell>
          <TableCell>{{ site.port }}</TableCell>
          <TableCell>{{ site.protocol }}</TableCell>
          <TableCell class="text-right">
            <div class="flex justify-end gap-2">
              <Button
                variant="outline"
                size="icon"
                @click="handleEditSite(site.id)"
              >
                <Pencil class="h-4 w-4" />
              </Button>
              <Button
                variant="destructive"
                size="icon"
                @click="handleDeleteSite(site.id)"
              >
                <Trash2 class="h-4 w-4" />
              </Button>
            </div>
          </TableCell>
        </TableRow>
      </TableBody>
    </Table>

    <div v-if="!loading && sites.length === 0" class="text-center py-8 text-gray-500">
      No sites found. Click "Add Site" to create your first site.
    </div>

    <Dialog v-model:open="deleteDialogOpen">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Delete Site</DialogTitle>
          <DialogDescription>
            Are you sure you want to delete the site "<strong>{{ siteToDeleteData?.slug }}</strong>"? This action cannot be undone.
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button variant="outline" @click="cancelDelete" :disabled="loading">
            Cancel
          </Button>
          <Button variant="destructive" @click="confirmDelete" :disabled="loading">
            Delete
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  </div>
</template>
