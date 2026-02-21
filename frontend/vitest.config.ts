import { mergeConfig } from 'vite';
import base from './vite.config';

export default mergeConfig(base, {
  test: {
    environment: 'jsdom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.{test,spec}.{ts,tsx}'],
  },
});
