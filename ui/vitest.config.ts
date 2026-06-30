/// <reference types="vitest" />
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';
import path from 'path';

export default defineConfig({
  plugins: [react()],
  test: {
    globals: true,
    environment: 'jsdom',
    setupFiles: ['./src/test/setup.ts'],
    css: false,
    pool: 'threads',
    // Vitest 4: poolOptions removed; use top-level options
    maxWorkers: 1,
    fileParallelism: false,
  },
  resolve: {
    alias: {
      'react-globe.gl': path.resolve(__dirname, 'src/test/__mocks__/react-globe-gl.tsx'),
    },
  },
});
