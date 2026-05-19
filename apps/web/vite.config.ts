import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react'

export default defineConfig({
  plugins: [react()],
  server: {
    proxy: {
      '/api': {
        target: process.env.VITE_OPENCHAT_SERVER_URL ?? 'http://127.0.0.1:8787',
        changeOrigin: true,
      },
    },
  },
})
