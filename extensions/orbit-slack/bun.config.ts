// Bun configuration for orbit-slack extension

const config = {
  // Build configuration
  target: 'node',
  platform: 'node',
  format: 'esm',

  // Development configuration
  development: {
    // Disable minification in development
    minify: false,
    // Enable source maps in development
    sourcemap: 'external',
    // Define development constants
    define: {
      'process.env.NODE_ENV': '"development"',
      'globalThis.DEV': 'true'
    }
  },

  // Production configuration
  production: {
    // Enable minification in production
    minify: {
      whitespace: true,
      identifiers: true,
      syntax: true
    },
    // Enable source maps in production
    sourcemap: 'external',
    // Define production constants
    define: {
      'process.env.NODE_ENV': '"production"',
      'globalThis.DEV': 'false'
    }
  },

  // External dependencies (don't bundle)
  external: [
    '@sentry/node',
    '@slack/bolt',
    '@slack/web-api',
    '@t3-oss/env-core',
    'winston',
    'ws',
    'axios',
    'zod'
  ],

  // Loader configuration for different file types
  loaders: {
    // Treat .ts files as TypeScript
    '.ts': 'ts',
    // Treat .js files as JavaScript
    '.js': 'js',
    // Treat .json files as JSON
    '.json': 'json'
  },

  // Path aliases (matching tsconfig.json)
  alias: {
    '@': './src',
    '@/bot': './src/bot',
    '@/services': './src/services',
    '@/types': './src/types',
    '@/utils': './src/utils'
  },

  // Test configuration
  test: {
    // Include test files
    include: ['**/*.test.ts', '**/*.spec.ts'],
    // Exclude test files from build
    exclude: ['**/*.test.ts', '**/*.spec.ts'],
  },

  // Node.js compatibility
  node: {
    // Polyfills for Node.js built-in modules
    globals: {
      Buffer: true,
      global: true,
      process: true,
      __dirname: true,
      __filename: true
    }
  }
};

export default config;
