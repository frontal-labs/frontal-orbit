import { beforeEach, describe, expect, it, vi } from 'vitest';
import { env, getEnvConfig, validateEnvConfig } from '../src/env';

const BASE_ENV = {
  SLACK_BOT_TOKEN: 'xoxb-test-token',
  SLACK_APP_TOKEN: 'xapp-test-token',
  SLACK_SIGNING_SECRET: 'test-signing-secret-with-at-least-32-chars',
  ORBIT_API_URL: 'http://localhost:8787',
  ORBIT_API_TIMEOUT: '30000',
  NODE_ENV: 'test',
  LOG_LEVEL: 'error',
  PORT: '3000',
  GITHUB_TOKEN: '',
  SENTRY_DSN: '',
  MAX_CONCURRENT_TASKS: '10',
  TASK_TIMEOUT: '3600000',
  HEALTH_CHECK_INTERVAL: '30000',
  SKIP_ENV_VALIDATION: '',
} as const;

async function importFreshEnvModule(overrides: Record<string, string | undefined> = {}) {
  vi.resetModules();

  for (const key of Object.keys(BASE_ENV)) {
    delete process.env[key];
  }

  for (const [key, value] of Object.entries({
    ...BASE_ENV,
    ...overrides,
  })) {
    if (value === undefined) {
      delete process.env[key];
      continue;
    }
    process.env[key] = value;
  }

  return import('../src/env');
}

describe('Environment Variables', () => {
  beforeEach(() => {
    // Reset environment variables before each test
    process.env.SKIP_ENV_VALIDATION = undefined;
    vi.resetModules();
  });

  describe('env object', () => {
    it('should have all required Slack environment variables', () => {
      expect(env.SLACK_BOT_TOKEN).toBeDefined();
      expect(env.SLACK_BOT_TOKEN).toMatch(/^xoxb-/);
      expect(env.SLACK_APP_TOKEN).toBeDefined();
      expect(env.SLACK_APP_TOKEN).toMatch(/^xapp-/);
      expect(env.SLACK_SIGNING_SECRET).toBeDefined();
      expect(env.SLACK_SIGNING_SECRET.length).toBeGreaterThanOrEqual(32);
    });

    it('should have Orbit API configuration', () => {
      expect(env.ORBIT_API_URL).toBeDefined();
      expect(env.ORBIT_API_URL).toMatch(/^https?:\/\//);
      expect(env.ORBIT_API_TIMEOUT).toBeDefined();
      expect(env.ORBIT_API_TIMEOUT).toBeGreaterThanOrEqual(1000);
      expect(env.ORBIT_API_TIMEOUT).toBeLessThanOrEqual(300000);
    });

    it('should have application configuration', () => {
      expect(env.NODE_ENV).toBeDefined();
      expect(['development', 'production', 'test']).toContain(env.NODE_ENV);
      expect(env.LOG_LEVEL).toBeDefined();
      expect(['error', 'warn', 'info', 'http', 'debug']).toContain(env.LOG_LEVEL);
      expect(env.PORT).toBeDefined();
      expect(env.PORT).toBeGreaterThanOrEqual(1000);
      expect(env.PORT).toBeLessThanOrEqual(65535);
    });
  });

  describe('getEnvConfig', () => {
    it('should return a properly structured configuration object', () => {
      const config = getEnvConfig();

      expect(config).toHaveProperty('slack');
      expect(config).toHaveProperty('orbit');
      expect(config).toHaveProperty('app');
      expect(config).toHaveProperty('limits');

      expect(config.slack).toHaveProperty('botToken');
      expect(config.slack).toHaveProperty('appToken');
      expect(config.slack).toHaveProperty('signingSecret');

      expect(config.orbit).toHaveProperty('apiUrl');
      expect(config.orbit).toHaveProperty('timeout');

      expect(config.app).toHaveProperty('nodeEnv');
      expect(config.app).toHaveProperty('logLevel');
      expect(config.app).toHaveProperty('port');

      expect(config.github).toBeUndefined();

      expect(config.limits).toHaveProperty('maxConcurrentTasks');
      expect(config.limits).toHaveProperty('taskTimeout');
      expect(config.limits).toHaveProperty('healthCheckInterval');
    });

    it('should map environment variables to config fields', () => {
      const config = getEnvConfig();

      expect(config.slack.botToken).toBe(env.SLACK_BOT_TOKEN);
      expect(config.slack.appToken).toBe(env.SLACK_APP_TOKEN);
      expect(config.slack.signingSecret).toBe(env.SLACK_SIGNING_SECRET);
      expect(config.orbit.apiUrl).toBe(env.ORBIT_API_URL);
      expect(config.orbit.timeout).toBe(env.ORBIT_API_TIMEOUT);
      expect(config.app.nodeEnv).toBe(env.NODE_ENV);
      expect(config.app.logLevel).toBe(env.LOG_LEVEL);
      expect(config.app.port).toBe(env.PORT);
      expect(config.limits.maxConcurrentTasks).toBe(env.MAX_CONCURRENT_TASKS);
      expect(config.limits.taskTimeout).toBe(env.TASK_TIMEOUT);
      expect(config.limits.healthCheckInterval).toBe(env.HEALTH_CHECK_INTERVAL);
    });

    it('should omit github config when the runtime token is an empty string', async () => {
      const { getEnvConfig: loadConfig } = await importFreshEnvModule({
        GITHUB_TOKEN: '',
      });

      const config = loadConfig();

      expect(config.github).toBeUndefined();
    });
  });

  describe('validateEnvConfig', () => {
    it('should not throw when configuration is valid', () => {
      expect(() => validateEnvConfig()).not.toThrow();
    });

    it('should be a function for backward compatibility', () => {
      expect(typeof validateEnvConfig).toBe('function');
    });
  });

  describe('runtime coercion and validation', () => {
    it('coerces numeric runtime env strings into numbers on fresh module load', async () => {
      const { env: freshEnv, getEnvConfig: loadConfig } = await importFreshEnvModule({
        ORBIT_API_TIMEOUT: '45000',
        PORT: '4567',
        MAX_CONCURRENT_TASKS: '7',
        TASK_TIMEOUT: '120000',
        HEALTH_CHECK_INTERVAL: '15000',
      });

      const config = loadConfig();

      expect(freshEnv.ORBIT_API_TIMEOUT).toBe(45000);
      expect(freshEnv.PORT).toBe(4567);
      expect(freshEnv.MAX_CONCURRENT_TASKS).toBe(7);
      expect(freshEnv.TASK_TIMEOUT).toBe(120000);
      expect(freshEnv.HEALTH_CHECK_INTERVAL).toBe(15000);
      expect(config.orbit.timeout).toBe(45000);
      expect(config.app.port).toBe(4567);
    });

    it('treats empty strings as undefined and falls back to schema defaults', async () => {
      const { env: freshEnv } = await importFreshEnvModule({
        ORBIT_API_URL: '',
        ORBIT_API_TIMEOUT: '',
        LOG_LEVEL: '',
        PORT: '',
        MAX_CONCURRENT_TASKS: '',
        TASK_TIMEOUT: '',
        HEALTH_CHECK_INTERVAL: '',
      });

      expect(freshEnv.ORBIT_API_URL).toBe('http://orbit-api:8787');
      expect(freshEnv.ORBIT_API_TIMEOUT).toBe(30000);
      expect(freshEnv.LOG_LEVEL).toBe('info');
      expect(freshEnv.PORT).toBe(3000);
      expect(freshEnv.MAX_CONCURRENT_TASKS).toBe(10);
      expect(freshEnv.TASK_TIMEOUT).toBe(3600000);
      expect(freshEnv.HEALTH_CHECK_INTERVAL).toBe(30000);
    });

    it('throws a validation error for invalid runtime env values', async () => {
      const errorSpy = vi.spyOn(console, 'error').mockImplementation(() => {});

      await expect(
        importFreshEnvModule({
          SLACK_BOT_TOKEN: 'invalid-token',
        })
      ).rejects.toThrow('Environment variable validation failed');

      expect(errorSpy).toHaveBeenCalledWith('Environment variable validation failed:');
    });

    it('throws on client-side access to server env variables', async () => {
      const originalWindow = globalThis.window;
      Object.defineProperty(globalThis, 'window', {
        value: {},
        configurable: true,
      });

      try {
        const { env: clientEnv } = await importFreshEnvModule();

        expect(() => clientEnv.SLACK_BOT_TOKEN).toThrow(
          "Attempted to access server-side environment variable 'SLACK_BOT_TOKEN' on the client"
        );
      } finally {
        if (originalWindow === undefined) {
          delete (globalThis as { window?: unknown }).window;
        } else {
          Object.defineProperty(globalThis, 'window', {
            value: originalWindow,
            configurable: true,
          });
        }
      }
    });
  });
});
