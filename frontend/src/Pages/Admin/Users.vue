<script setup lang="ts">
import { ref, computed } from 'vue'
import { storeToRefs } from 'pinia'
import { useUserStore } from '@/stores/user'
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

const userStore = useUserStore()
const { users, loading } = storeToRefs(userStore)

useAuthGuard({ admin: true })

const deleteDialogOpen = ref(false)
const userToDelete = ref<number | null>(null)

const userToDeleteData = computed(() => {
  if (userToDelete.value === null) return null
  return users.value.find(user => user.id === userToDelete.value)
})

const handleEditUser = (userId: number) => {
  // TODO: Implement edit functionality
  console.log('Edit user:', userId)
}

const handleDeleteUser = (userId: number) => {
  userToDelete.value = userId
  deleteDialogOpen.value = true
}

const confirmDelete = async () => {
  if (userToDelete.value === null) return

  try {
    await userStore.remove(userToDelete.value)
    deleteDialogOpen.value = false
    userToDelete.value = null
  } catch (error) {
    // Error is handled by the store
  }
}

const cancelDelete = () => {
  deleteDialogOpen.value = false
  userToDelete.value = null
}
</script>

<template>
  <div class="w-full">
    <div class="flex justify-between items-center mb-6">
      <h1 class="text-2xl font-bold">Users</h1>
    </div>

    <div v-if="loading" class="text-center py-8">
      Loading users...
    </div>

    <Table v-else>
      <TableHeader>
        <TableRow>
          <TableHead>Name</TableHead>
          <TableHead>UUID</TableHead>
          <TableHead>Role</TableHead>
          <TableHead class="text-right">Actions</TableHead>
        </TableRow>
      </TableHeader>
      <TableBody>
        <TableRow v-for="user in users" :key="user.id">
          <TableCell class="font-medium">{{ user.name }}</TableCell>
          <TableCell>{{ user.uuid }}</TableCell>
          <TableCell>{{ user.role.name }}</TableCell>
          <TableCell class="text-right">
            <div class="flex justify-end gap-2">
              <Button
                variant="outline"
                size="icon"
                @click="handleEditUser(user.id)"
              >
                <Pencil class="h-4 w-4" />
              </Button>
              <Button
                variant="destructive"
                size="icon"
                @click="handleDeleteUser(user.id)"
              >
                <Trash2 class="h-4 w-4" />
              </Button>
            </div>
          </TableCell>
        </TableRow>
      </TableBody>
    </Table>

    <div v-if="!loading && users.length === 0" class="text-center py-8 text-gray-500">
      No users found.
    </div>

    <Dialog v-model:open="deleteDialogOpen">
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Delete User</DialogTitle>
          <DialogDescription>
            Are you sure you want to delete user "<strong>{{ userToDeleteData?.name }}</strong>"? This action cannot be undone.
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
