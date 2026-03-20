import { sveltekit } from '@sveltejs/kit/vite';
import { defineConfig } from 'vite';

export default defineConfig({
  plugins: [sveltekit()],
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
  },
  build: {
    target: ['es2022'],
    // @ts-ignore
    minify: !process.env.TAURI_DEBUG ? 'esbuild' : false,
    // @ts-ignore
    sourcemap: !!process.env.TAURI_DEBUG,
  },
});
