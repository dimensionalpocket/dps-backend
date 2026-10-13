import './style.css'
import 'inter-ui/inter-variable.css'
import { createApp, h } from 'vue'
import { createPinia } from 'pinia'
import { createInertiaApp } from '@inertiajs/vue3'
import AppLayout from '@/Layouts/AppLayout.vue'
import AdminLayout from '@/Layouts/AdminLayout.vue'

// Lazy (non-eager) glob: Vite emits one async chunk per page, fetched on demand.
const pages = import.meta.glob('./Pages/**/*.vue')

createInertiaApp({
  resolve: (name) => {
    const page = pages[`./Pages/${name}.vue`]
    if (!page) throw new Error(`Unknown Inertia page: ${name}`)
    return page() as Promise<{ default: any }>
  },
  layout: (name) => (name.startsWith('Admin/') ? [AppLayout, AdminLayout] : AppLayout),
  setup({ el, App, props, plugin }) {
    createApp({ render: () => h(App, props) })
      .use(plugin)
      .use(createPinia())
      .mount(el)
  },
})
