import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import path from 'node:path';

export default defineConfig({
  plugins: [svelte()],
  resolve: {
    conditions: ['browser'],
  },
  server: {
    fs: {
      allow: ['..', path.resolve(__dirname, '../locales')],
    },
    proxy: {
      '/ws': {
        target: 'http://127.0.0.1:47320',
        ws: true,
      },
      '/api': {
        target: 'http://127.0.0.1:47320',
      },
    },
  },
  test: {
    environment: 'jsdom',
    include: ['tests/**/*.test.ts'],
  },
});
