/// <reference types="vitest/config" />
import { defineConfig } from 'vite';
import { svelte } from '@sveltejs/vite-plugin-svelte';
import { svelteTesting } from '@testing-library/svelte/vite';

// İki pencere iki ayrı giriş noktasıdır (1c tasarımı §3.1). `vitrin.html`
// yalnızca geliştirme sunucusunda açılır; derlemeye girmez.
export default defineConfig({
  plugins: [svelte(), svelteTesting()],
  clearScreen: false,
  server: { port: 5173, strictPort: true },
  build: {
    target: 'es2022',
    manifest: true,
    rolldownOptions: {
      input: { index: 'index.html', sartlar: 'sartlar.html' },
    },
  },
  test: {
    environment: 'jsdom',
    include: ['src/**/*.test.ts'],
  },
});
