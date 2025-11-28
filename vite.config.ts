import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react()],

  // Tauri-specific configurations
  clearScreen: false,
  
  server: {
    port: 5173,
    strictPort: true,
    host: 'localhost',
    watch: {
      // Tell vite to ignore watching `src-tauri`
      ignored: ['**/src-tauri/**'],
    },
  },

  // Environment variable prefix for Tauri
  envPrefix: ['VITE_', 'TAURI_'],

  build: {
    // Tauri uses WebKit on Linux (Arch with Hyprland)
    target: process.env.TAURI_PLATFORM === 'windows' 
      ? 'chrome105' 
      : 'safari13',
    // Don't minify for debug builds
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    // Produce sourcemaps for debug builds
    sourcemap: !!process.env.TAURI_DEBUG,
    // Optimize chunk size for desktop app
    chunkSizeWarningLimit: 1000,
  },

  optimizeDeps: {
    exclude: ['lucide-react'],
  },
});
