import { defineConfig } from 'vite'
import vue from '@vitejs/plugin-vue'
import tailwindcss from '@tailwindcss/vite'
import { writeFileSync, mkdirSync } from 'node:fs'
import path from 'node:path'

export default defineConfig(({ command }) => ({
  base: command === 'build' ? '/build/' : '/',
  plugins: [
    vue(),
    tailwindcss(),
    {
      name: 'dps-write-hot-file',
      configureServer(server) {
        const hotFile = path.resolve(__dirname, 'hot')
        const write = (contents: string) => {
          mkdirSync(path.dirname(hotFile), { recursive: true })
          writeFileSync(hotFile, contents)
        }
        server.httpServer?.once('listening', () => {
          write(`http://localhost:${server.config.server.port}`)
        })
        server.httpServer?.once('close', () => { try { write('') } catch {} })
      },
    },
  ],
  resolve: { alias: { '@': path.resolve(__dirname, './src') } },
  build: { outDir: 'dist', manifest: true, rollupOptions: { input: 'src/main.ts' } },
  server: { host: true, port: 5000, strictPort: true, cors: true },
}))
