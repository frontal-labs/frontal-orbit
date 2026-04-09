import { describe, it, expect, beforeEach } from 'vitest';
import { env, getEnvConfig, validateEnvConfig } from '@/utils/env';

describe('Environment Variables', () => {
  beforeEach(() => {
    // Reset environment variables before each test
    delete process.env.SKIP_ENV_VALIDATION;
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

    it('should have database and Redis URLs', () => {
      expect(env.DATABASE_URL).toBeDefined();
      expect(env.DATABASE_URL).toMatch(/^postgresql:\/\//);
      expect(env.REDIS_URL).toBeDefined();
      expect(env.REDIS_URL).toMatch(/^redis:\/\//);
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

    it('should have limit configuration', () => {
      expect(env.MAX_CONCURRENT_TASKS).toBeDefined();
      expect(env.MAX_CONCURRENT_TASKS).toBeGreaterThanOrEqual(1);
      expect(env.MAX_CONCURRENT_TASKS).toBeLessThanOrEqual(100);
      expect(env.TASK_TIMEOUT).toBeDefined();
      expect(env.TASK_TIMEOUT).toBeGreaterThanOrEqual(60000);
      expect(env.TASK_TIMEOUT).toBeLessThanOrEqual(7200000);
      expect(env.HEALTH_CHECK_INTERVAL).toBeDefined();
      expect(env.HEALTH_CHECK_INTERVAL).toBeGreaterThanOrEqual(5000);
      expect(env.HEALTH_CHECK_INTERVAL).toBeLessThanOrEqual(300000);
    });

    it('should have optional GitHub token', () => {
      // GitHub token is optional, so it can be undefined
      expect(env.GITHUB_TOKEN === undefined || typeof env.GITHUB_TOKEN === 'string').toBe(true);
    });
  });

  describe('getEnvConfig', () => {
    it('should return a properly structured configuration object', () => {
      const config = getEnvConfig();

      expect(config).toHaveProperty('slack');
      expect(config).toHaveProperty('orbit');
      expect(config).toHaveProperty('database');
      expect(config).toHaveProperty('redis');
      expect(config).toHaveProperty('app');
      expect(config).toHaveProperty('limits');

      expect(config.slack).toHaveProperty('botToken');
      expect(config.slack).toHaveProperty('appToken');
      expect(config.slack).toHaveProperty('signingSecret');

      expect(config.orbit).toHaveProperty('apiUrl');
      expect(config.orbit).toHaveProperty('timeout');

      expect(config.database).toHaveProperty('url');
      expect(config.redis).toHaveProperty('url');

      expect(config.app).toHaveProperty('nodeEnv');
      expect(config.app).toHaveProperty('logLevel');
      expect(config.app).toHaveProperty('port');

      expect(config.limits).toHaveProperty('maxConcurrentTasks');
      expect(config.limits).toHaveProperty('taskTimeout');
      expect(config.limits).toHaveProperty('healthCheckInterval');
    });

    it('should map environment variables correctly', () => {
      const config = getEnvConfig();

      expect(config.slack.botToken).toBe(env.SLACK_BOT_TOKEN);
      expect(config.slack.appToken).toBe(env.SLACK_APP_TOKEN);
      expect(config.slack.signingSecret).toBe(env.SLACK_SIGNING_SECRET);

      expect(config.orbit.apiUrl).toBe(env.ORBIT_API_URL);
      expect(config.orbit.timeout).toBe(env.ORBIT_API_TIMEOUT);

      expect(config.database.url).toBe(env.DATABASE_URL);
      expect(config.redis.url).toBe(env.REDIS_URL);

      expect(config.app.nodeEnv).toBe(env.NODE_ENV);
      expect(config.app.logLevel).toBe(env.LOG_LEVEL);
      expect(config.app.port).toBe(env.PORT);

      expect(config.limits.maxConcurrentTasks).toBe(env.MAX_CONCURRENT_TASKS);
      expect(config.limits.taskTimeout).toBe(env.TASK_TIMEOUT);
      expect(config.limits.healthCheckInterval).toBe(env.HEALTH_CHECK_INTERVAL);
    });

    it('should handle optional GitHub token', () => {
      const config = getEnvConfig();

      if (env.GITHUB_TOKEN) {
        expect(config.github).toBeDefined();
        expect(config.github?.token).toBe(env.GITHUB_TOKEN);
      } else {
        expect(config.github).toBeUndefined();
      }
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

  describe('Default Values', () => {
    it('should use default values when environment variables are not set', () => {
      // These are set in the test setup, but we can verify they match expected defaults
      expect(env.ORBIT_API_URL).toBe('http://localhost:8787');
      expect(env.ORBIT_API_TIMEOUT).toBe(30000);
      expect(env.NODE_ENV).toBe('test');
      expect(env.LOG_LEVEL).toBe('error');
      expect(env.PORT).toBe(3000);
      expect(env.MAX_CONCURRENT_TASKS).toBe(10);
      expect(env.TASK_TIMEOUT).toBe(3600000);
      expect(env.HEALTH_CHECK_INTERVAL).toBe(30000);
    });
  });

  describe('Type Safety', () => {
    it('should have correct types for all environment variables', () => {
      expect(typeof env.SLACK_BOT_TOKEN).toBe('string');
      expect(typeof env.SLACK_APP_TOKEN).toBe('string');
      expect(typeof env.SLACK_SIGNING_SECRET).toBe('string');
      expect(typeof env.ORBIT_API_URL).toBe('string');
      expect(typeof env.ORBIT_API_TIMEOUT).toBe('number');
      expect(typeof env.DATABASE_URL).toBe('string');
      expect(typeof env.REDIS_URL).toBe('string');
      expect(typeof env.NODE_ENV).toBe('string');
      expect(typeof env.LOG_LEVEL).toBe('string');
      expect(typeof env.PORT).toBe('number');
      expect(typeof env.MAX_CONCURRENT_TASKS).toBe('number');
      expect(typeof env.TASK_TIMEOUT).toBe('number');
      expect(typeof env.HEALTH_CHECK_INTERVAL).toBe('number');
      expect(env.GITHUB_TOKEN === undefined || typeof env.GITHUB_TOKEN === 'string').toBe(true);
    });
  });

  describe('Validation Rules', () => {
    it('should validate Slack token formats', () => {
      expect(env.SLACK_BOT_TOKEN).toMatch(/^xoxb-/);
      expect(env.SLACK_APP_TOKEN).toMatch(/^xapp-/);
    });

    it('should validate URL formats', () => {
      expect(env.ORBIT_API_URL).toMatch(/^https?:\/\/.+/);
      expect(env.DATABASE_URL).toMatch(/^postgresql:\/\/.+/);
      expect(env.REDIS_URL).toMatch(/^redis:\/\/.+/);
    });

    it('should validate numeric ranges', () => {
      expect(env.ORBIT_API_TIMEOUT).toBeGreaterThanOrEqual(1000);
      expect(env.ORBIT_API_TIMEOUT).toBeLessThanOrEqual(300000);
      expect(env.PORT).toBeGreaterThanOrEqual(1000);
      expect(env.PORT).toBeLessThanOrEqual(65535);
      expect(env.MAX_CONCURRENT_TASKS).toBeGreaterThanOrEqual(1);
      expect(env.MAX_CONCURRENT_TASKS).toBeLessThanOrEqual(100);
      expect(env.TASK_TIMEOUT).toBeGreaterThanOrEqual(60000);
      expect(env.TASK_TIMEOUT).toBeLessThanOrEqual(7200000);
      expect(env.HEALTH_CHECK_INTERVAL).toBeGreaterThanOrEqual(5000);
      expect(env.HEALTH_CHECK_INTERVAL).toBeLessThanOrEqual(300000);
    });

    it('should validate enum values', () => {
      expect(['development', 'production', 'test']).toContain(env.NODE_ENV);
      expect(['error', 'warn', 'info', 'http', 'debug']).toContain(env.LOG_LEVEL);
    });
  });
});
