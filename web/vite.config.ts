import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  // KaTeX makes the single bundle ~630 kB (~200 kB gzipped); fine for now.
  build: { chunkSizeWarningLimit: 800 },
  server: {
    // `npm run dev` forwards API calls to the Rust server (cargo run -p gedpasser-server).
    proxy: { '/api': 'http://localhost:8080' },
  },
})
