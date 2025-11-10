import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react-swc'
import { TanStackRouterVite } from '@tanstack/router-vite-plugin'
import tailwindcss from '@tailwindcss/vite'

export default defineConfig({
  plugins: [react(), TanStackRouterVite(), tailwindcss()],
  server: {
    port: 5173,
    strictPort: true,
  },
})
