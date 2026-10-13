<script setup lang="ts">
import { computed } from 'vue'
import { Link, usePage } from '@inertiajs/vue3'
import { LayoutDashboard, Users, Globe } from 'lucide-vue-next'
import Card from '@/components/ui/card/Card.vue'
import CardContent from '@/components/ui/card/CardContent.vue'
import Tabs from '@/components/ui/tabs/Tabs.vue'
import TabsList from '@/components/ui/tabs/TabsList.vue'
import TabsTrigger from '@/components/ui/tabs/TabsTrigger.vue'

const page = usePage()

const TAB_BY_COMPONENT: Record<string, string> = {
  'Admin/Home': 'admin-home',
  'Admin/Users': 'admin-users',
  'Admin/Sites': 'admin-sites',
  'Admin/SitesNew': 'admin-sites',
  'Admin/SitesEdit': 'admin-sites',
}

const activeTab = computed(() => TAB_BY_COMPONENT[page.component] ?? 'admin-home')
</script>

<template>
  <div class="w-full max-w-4xl items-start h-full p-4">
    <Card class="w-full p-4">
      <CardContent class="p-0 m-0">
        <Tabs :model-value="activeTab" class="w-full">
          <TabsList class="w-full">
            <TabsTrigger value="admin-home" as-child>
              <Link href="/admin" class="flex items-center gap-2">
                <LayoutDashboard class="h-4 w-4" />
                Dashboard
              </Link>
            </TabsTrigger>
            <TabsTrigger value="admin-users" as-child>
              <Link href="/admin/users" class="flex items-center gap-2">
                <Users class="h-4 w-4" />
                Users
              </Link>
            </TabsTrigger>
            <TabsTrigger value="admin-sites" as-child>
              <Link href="/admin/sites" class="flex items-center gap-2">
                <Globe class="h-4 w-4" />
                Sites
              </Link>
            </TabsTrigger>
          </TabsList>
        </Tabs>
        <div class="mt-4 w-full">
          <slot/>
        </div>
      </CardContent>
    </Card>
  </div>
</template>
