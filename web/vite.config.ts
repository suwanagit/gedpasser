import react from '@vitejs/plugin-react'
import { defineConfig } from 'vite'

// https://vite.dev/config/
export default defineConfig({
  plugins: [react()],
  server: {
    // `npm run dev` forwards API calls to the Rust server (cargo run -p gedpasser-server).
    proxy: { '/api': 'http://localhost:8080' },
  },
})
