# AGENTS.md

## Commands

- **Build**: `bun run build` (this generates the `dist/` folder)
- **Dev server**: `bun run dev`
- **Tests**: `bun test`

## Working Guidelines

### Working with Plans
- If plan involves using a component from Shadcn that is not already created, include the command to generate it using the Shadcn CLI


## Project Structure

- This is a Bun project. Never use `npm`, `node`, `typescript`, or `tsc` commands. Always use `bun` commands.
- This is a Vue project using Typescript natively (via Bun). Typescript transpilation is not required.

- Entry point is `src/main.ts`, which boots the Inertia app.
- Pages live in `src/Pages/` and layouts live in `src/Layouts/`.
  - Page component names must match the Inertia component names declared by the backend (e.g. `Admin/Users` -> `src/Pages/Admin/Users.vue`).
  - The backend assigns layouts by component name (see `src/main.ts`): `Admin/*` pages get `AppLayout` + `AdminLayout`, everything else gets `AppLayout`.
- Components live in `src/components/`.

- Stores live in `src/stores/`.
  - The project uses Pinia for state management.
  - The Auth store stores user session state fetched from the REST API.

- There is no Vue Router. Navigation uses Inertia:
  - Use Inertia's `Link` component with URL strings (`<Link href="/admin/users">`); there are no named routes.
  - Use `router.visit('/path')` from `@inertiajs/vue3` for programmatic navigation.
  - Use `usePage()` to read `page.url`, `page.props`, and `page.component`. Route params are passed as page props (e.g. `defineProps<{ id: number }>()`).
  - Use Inertia's `<Head title="..." />` for document titles.
  - Client-side auth guarding uses `useAuthGuard()` from `src/composables/useAuthGuard.ts` (`useAuthGuard({ admin: true })` for admin pages).

- The lib folder (`src/lib/`) contains utility functions and modules.
  - REST access lives in `src/lib/api/`. Use `apiFetch` from `src/lib/api/client.ts`; it sends the session cookie and normalizes the `{ error }` envelope into an `ApiError`.
  - The same-origin API base path comes from `src/lib/config.ts` (DpsConfig `api_path`).

- The project uses Shadcn-Vue for component generation. Generated components live in `src/components/ui/`.
  - When installing new Shadcn-Vue components, use the `bunx --bun shadcn-vue@latest add <component>` command.
  - After installing a Shadcn-Vue component, do not change the generated code. If changes are necessary, document them in the plan.

- Other components live in `src/components/` (not in `ui/`).

- The project has an `@` alias pointing to `src/` for easier imports. When importing from `src/`, use `@/` instead of relative paths.

- Styles live in `src/style.css`. However that file is mostly for TailwindCSS imports and global styles.
  - For global styles, use the layout's `<style>` block (e.g. `src/Layouts/AppLayout.vue`).
  - For component-specific styles, use the component's own `<style>` block. Do not use `scoped` styles.

- In `.vue` files, always place the `<script>` block first, followed by the `<template>`, and then the `<style>`.
  - If the `<style>` block has no styles set, omit it entirely.

- The project uses composition API with `<script setup lang="ts">` syntax in Vue components.

- When designing interfaces, use TailwindCSS 4 for styling.

## Backend Interaction

- The app interacts with the REST backend for authentication and user management.
- All REST endpoints are prefixed with the configured `api_path` (default `/api`) and are documented in the parent crate's `README.md`.
- The session cookie is `HttpOnly` and scoped to `Path=/{api_path}`; JavaScript cannot read it.


## Code Style Guidelines

### Testing

For unit tests of non-Vue code:
- Use `bun:test` framework with `describe`, `it`, and `expect` functions
- Do not use "should" in test descriptions (e.g., use `it('returns the correct path')` instead of `it('should return...')`)

Testing of Vue components is not currently required.

### Linting

There currently no linter configured. Use these simple rules:
- Use 2 spaces for indentation
- Use single quotes for strings
- No semicolons at the end of lines
- Use trailing commas in multi-line objects and arrays
- Files should end with a newline
- Use a space before the brackets in function definitions
