import { resolve } from 'node:path';
import { defineConfig } from 'vitest/config';

export default defineConfig({
  test: {
    // Test environment
    environment: 'node',

    // Global setup
    globals: true,

    // Coverage configuration
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json', 'html'],
      include: ['src/**/*.{ts,tsx}'],
      exclude: [
        'src/**/*.d.ts',
        'src/**/*.test.{ts,tsx}',
        'src/**/*.spec.{ts,tsx}',
        'src/index.ts',
        'node_modules/',
        'dist/',
        'coverage/',
      ],
      thresholds: {
        global: {
          branches: 80,
          functions: 80,
          lines: 80,
          statements: 80,
        },
      },
    },

    // Test files
    include: [
      'tests/env.test.ts',
      'tests/env-runtime.test.ts',
      'tests/config.test.ts',
      'tests/types/slack-types.test.ts',
      'tests/api-client.test.ts',
      'tests/api-client-interceptors.test.ts',
      'tests/orbit-events.test.ts',
      'tests/events-client.test.ts',
      'tests/log.test.ts',
      'tests/slack-constructor.test.ts',
      'tests/slack-formatting.test.ts',
      'tests/slack-behavior.test.ts',
    ],

    // Exclude patterns
    exclude: ['node_modules/', 'dist/', 'coverage/'],

    // Test timeout
    testTimeout: 10000,

    // Hook timeout
    hookTimeout: 10000,

    // Threads
    threads: true,

    // Isolate tests
    isolate: true,

    // Watch mode
    watch: false,

    // Reporter
    reporter: ['verbose'],

    // Setup files
    setupFiles: ['tests/setup-current.ts'],
  },

  // Resolve configuration
  resolve: {
    alias: {
      '@': resolve(__dirname, './src'),
      '@/types': resolve(__dirname, './src/types'),
    },
  },

  // TypeScript configuration
  esbuild: {
    target: 'es2020',
  },
});
