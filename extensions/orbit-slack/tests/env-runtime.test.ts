import { afterEach, describe, expect, it, vi } from 'vitest';

const REQUIRED_ENV = {
  SLACK_BOT_TOKEN: 'xoxb-test-token',
  SLACK_APP_TOKEN: 'xapp-test-token',
  SLACK_SIGNING_SECRET: 'test-signing-secret-with-at-least-32-chars',
  ORBIT_API_URL: 'http://localhost:8787',
  ORBIT_API_TIMEOUT: '30000',
  NODE_ENV: 'test',
  LOG_LEVEL: 'error',
  PORT: '3000',
  MAX_CONCURRENT_TASKS: '10',
  TASK_TIMEOUT: '3600000',
  HEALTH_CHECK_INTERVAL: '30000',
} as const;

async function loadEnvModule(extraEnv: Record<string, string | undefined> = {}) {
  vi.resetModules();
  for (const [key, value] of Object.entries({ ...REQUIRED_ENV, ...extraEnv })) {
    if (value === undefined) {
      delete process.env[key];
    } else {
      process.env[key] = value;
    }
  }
  return import('../src/env');
}

describe('Environment runtime branches', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('includes GitHub config when a token is provided at import time', async () => {
    const module = await loadEnvModule({
      GITHUB_TOKEN: 'github-token',
    });

    expect(module.getEnvConfig().github).toEqual({
      token: 'github-token',
    });
  });

  it('omits GitHub config when the token is empty', async () => {
    const module = await loadEnvModule({
      GITHUB_TOKEN: '',
    });

    expect(module.getEnvConfig().github).toBeUndefined();
  });

  it('logs validation success for the compatibility helper', async () => {
    const module = await loadEnvModule();
    const logSpy = vi.spyOn(console, 'log').mockImplementation(() => {});

    module.validateEnvConfig();

    expect(logSpy).toHaveBeenCalledWith('Environment variables validated successfully');
  });
});
