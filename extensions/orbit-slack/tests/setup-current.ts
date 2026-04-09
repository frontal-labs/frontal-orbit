import { vi } from 'vitest';

process.env.NODE_ENV = 'test';
process.env.LOG_LEVEL = 'error';
process.env.SLACK_BOT_TOKEN = 'xoxb-test-token';
process.env.SLACK_APP_TOKEN = 'xapp-test-token';
process.env.SLACK_SIGNING_SECRET = 'test-signing-secret-with-at-least-32-chars';
process.env.ORBIT_API_URL = 'http://localhost:8787';
process.env.ORBIT_API_TIMEOUT = '30000';
process.env.PORT = '3000';
process.env.MAX_CONCURRENT_TASKS = '10';
process.env.TASK_TIMEOUT = '3600000';
process.env.HEALTH_CHECK_INTERVAL = '30000';
process.env.GITHUB_TOKEN = '';
process.env.SENTRY_DSN = '';
process.env.SKIP_ENV_VALIDATION = '';

beforeAll(() => {
  vi.spyOn(console, 'log').mockImplementation(() => {});
  vi.spyOn(console, 'info').mockImplementation(() => {});
  vi.spyOn(console, 'warn').mockImplementation(() => {});
  vi.spyOn(console, 'error').mockImplementation(() => {});
});

afterAll(() => {
  vi.restoreAllMocks();
});

afterEach(() => {
  vi.clearAllMocks();
});
