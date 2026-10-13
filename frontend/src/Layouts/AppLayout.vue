<script setup lang="ts">
import { Head } from '@inertiajs/vue3'
import LoginDropdown from '@/components/LoginDropdown.vue'
import Button from '@/components/ui/button/Button.vue'
import { Toaster } from '@/components/ui/sonner'
import { useColorMode } from '@vueuse/core'
import { ChevronLeft } from 'lucide-vue-next'
import 'vue-sonner/style.css'

useColorMode({ initialValue: 'dark' })
</script>

<template>
  <Head title="Account center" />
  <div class="min-h-screen flex flex-col bg-slate-50 dark:bg-slate-950">
    <!-- Navbar: full-bleed background, constrained inner content -->
    <header class="w-full bg-slate-100 dark:bg-slate-900 border-b-2 border-b-slate-500">
      <div class="mx-auto max-w-4xl px-4 py-2 flex items-center justify-between">
        <Button variant="outline" size="lg" class="has-[>svg]:px-1 has-[>svg]:py-6">
          <ChevronLeft class="size-8"/>
          <div class="hidden sm:inline text-left leading-none pl-0 pr-2 pt-0">
            <span class="text-xs font-light text-muted-foreground">Return to</span><br/>
            <span>Site</span>
          </div>
        </Button>
        <div class="font-semibold">Account Center</div>
        <LoginDropdown/>
      </div>
    </header>

    <!-- Main: expands to fill space between header and footer, centers content -->
    <main class="flex-1 flex items-stretch justify-center px-4">
      <div class="flex mx-auto max-w-4xl w-full items-center justify-center">
        <slot/>
      </div>
    </main>

    <!-- Footer: full-bleed background, constrained inner content -->
    <footer class="w-full bg-slate-100 dark:bg-slate-900 border-t-2 border-t-slate-500 text-sm font-extralight">
      <div class="mx-auto max-w-4xl px-4 pt-2 pb-3 text-center">
        <!-- Terms of Service <b class="font-extrabold mx-3">·</b> Privacy Policy -->
      </div>
    </footer>

    <!-- Global Toast Container -->
    <div class="fixed bottom-0 left-0 right-0 flex justify-center p-4 pointer-events-none">
      <div class="w-full max-w-4xl pointer-events-auto">
        <Toaster position="bottom-center" closeButton />
      </div>
    </div>
  </div>
</template>

<style>
html, body {
  font-feature-settings: "cv02","cv03","cv04","cv11","tnum";
  font-family: "InterVariable", "Inter", "system-ui";
}

/* This resets 1Password's unreadable colors in dark mode when fields are auto-filled. */
input[data-com-onepassword-filled="light"] {
  background-color: initial !important;
  color: initial !important;
}

button[aria-label="Close toast"] {
  border: 2px solid #222 !important;
  /* Move to top-right */
  position: absolute !important;
  top: auto !important;
  right: 8px !important;
  left: auto !important;
  bottom: auto !important;
  transform: none !important;
}
</style>
