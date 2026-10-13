import { defineStore } from 'pinia'
import { ref } from 'vue'
import {
  create as createSite,
  list as listSites,
  remove as removeSite,
  update as updateSite,
} from '@/lib/api/sites'
import type { Site } from '@/lib/api/types'

export const useSiteStore = defineStore('site', () => {
  const loading = ref(false)
  const sites = ref<Site[]>([])
  const lastError = ref<string | null>(null)

  async function fetch() {
    loading.value = true
    lastError.value = null

    try {
      sites.value = await listSites()
    } catch (error: any) {
      lastError.value = error.message || 'Failed to fetch sites'
    } finally {
      loading.value = false
    }
  }

  // Load sites immediately
  fetch()

  async function add(slug: string, subdomain?: string, port?: number, protocol?: string, metadataJson?: string) {
    loading.value = true
    lastError.value = null

    try {
      const newSite = await createSite({ slug, subdomain, port, protocol, metadataJson })
      sites.value.push(newSite)
      return newSite
    } catch (error: any) {
      lastError.value = error.message || 'Failed to add site'
      throw error
    } finally {
      loading.value = false
    }
  }

  async function update(id: number, slug?: string, subdomain?: string, port?: number, protocol?: string, metadataJson?: string) {
    loading.value = true
    lastError.value = null

    try {
      const updatedSite = await updateSite(id, { slug, subdomain, port, protocol, metadataJson })
      const index = sites.value.findIndex(site => site.id === id)
      if (index !== -1) {
        sites.value[index] = updatedSite
      }
      return updatedSite
    } catch (error: any) {
      lastError.value = error.message || 'Failed to update site'
      throw error
    } finally {
      loading.value = false
    }
  }

  async function remove(siteId: number) {
    loading.value = true
    lastError.value = null

    try {
      const removedSite = await removeSite(siteId)
      sites.value = sites.value.filter(site => site.id !== siteId)
      return removedSite
    } catch (error: any) {
      lastError.value = error.message || 'Failed to remove site'
      throw error
    } finally {
      loading.value = false
    }
  }

  return {
    loading,
    sites,
    lastError,
    fetch,
    add,
    update,
    remove,
  }
})
