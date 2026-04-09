import { beforeEach, describe, expect, it, vi } from 'vitest';

type LoadedLogModule = Awaited<ReturnType<typeof loadLogModule>>;

async function loadLogModule(nodeEnv: 'test' | 'production' = 'test') {
  vi.resetModules();

  const sentry = {
    init: vi.fn(),
    captureMessage: vi.fn(),
    captureException: vi.fn(),
  };

  const innerLogger = {
    debug: vi.fn(),
    info: vi.fn(),
    warn: vi.fn(),
    error: vi.fn(),
  };

  const format = {
    combine: vi.fn((...parts: unknown[]) => ({ kind: 'combine', parts })),
    timestamp: vi.fn(() => ({ kind: 'timestamp' })),
    json: vi.fn(() => ({ kind: 'json' })),
    colorize: vi.fn(() => ({ kind: 'colorize' })),
    simple: vi.fn(() => ({ kind: 'simple' })),
  };

  class ConsoleTransport {
    public readonly options?: Record<string, unknown>;

    constructor(options?: Record<string, unknown>) {
      this.options = options;
    }
  }

  const createLogger = vi.fn(() => innerLogger);

  vi.doMock('@sentry/node', () => sentry);
  vi.doMock('../src/config', () => ({
    config: {
      app: {
        nodeEnv,
        logLevel: 'debug',
      },
    },
  }));
  vi.doMock('winston', () => ({
    default: {
      createLogger,
      format,
      transports: {
        Console: ConsoleTransport,
      },
    },
  }));

  const module = await import('../src/log');
  return {
    ...module,
    sentry,
    innerLogger,
    createLogger,
    format,
  };
}

describe('log helpers', () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it('initializes Sentry in production and not in test', async () => {
    const productionModule = await loadLogModule('production');
    expect(productionModule.sentry.init).toHaveBeenCalledTimes(1);

    const testModule = await loadLogModule('test');
    expect(testModule.sentry.init).not.toHaveBeenCalled();
  });

  it('forwards debug/info/warn logs and captures production warnings in Sentry', async () => {
    const { logger, innerLogger, sentry } = await loadLogModule('production');

    logger.debug('debug message', { scope: 'debug' });
    logger.info('info message', { scope: 'info' });
    logger.warn('warn message', { scope: 'warn' });

    expect(innerLogger.debug).toHaveBeenCalledWith('debug message', { scope: 'debug' });
    expect(innerLogger.info).toHaveBeenCalledWith('info message', { scope: 'info' });
    expect(innerLogger.warn).toHaveBeenCalledWith('warn message', { scope: 'warn' });
    expect(sentry.captureMessage).toHaveBeenCalledWith('warn message', 'warning');
  });

  it('captures exceptions and plain error messages in production error logs', async () => {
    const { logger, innerLogger, sentry } = await loadLogModule('production');
    const error = new Error('boom');

    logger.error('error with exception', error, { taskId: 'task-123' });
    logger.error('error without exception');

    expect(innerLogger.error).toHaveBeenNthCalledWith(
      1,
      'error with exception',
      expect.objectContaining({
        taskId: 'task-123',
        error: 'boom',
        stack: expect.any(String),
      })
    );
    expect(innerLogger.error).toHaveBeenNthCalledWith(
      2,
      'error without exception',
      expect.objectContaining({
        error: undefined,
        stack: undefined,
      })
    );
    expect(sentry.captureException).toHaveBeenCalledWith(error);
    expect(sentry.captureMessage).toHaveBeenCalledWith('error without exception', 'error');
  });

  it('logs task, API, Slack, user, system, security, database, and redis helper events', async () => {
    const module = await loadLogModule('test');

    module.logTaskCreation('task-123', 'U123', 'x'.repeat(140));
    module.logTaskStatusChange('task-123', 'pending', 'running');
    module.logApiCall('/v1/tasks', 'POST', 25, true);
    module.logSlackEvent('message', 'U123', 'C123');
    module.logUserAction('U123', 'approve', { taskId: 'task-123' });
    module.logSystemEvent('booted', { region: 'eu-west' });
    module.logSecurityEvent('permission_denied', 'U123', { channelId: 'C123' });
    module.logDatabaseOperation('insert', 'tasks', 10, { rows: 1 });
    module.logRedisOperation('set', 'task:123', 5, { cache: 'tasks' });

    expect(module.innerLogger.info).toHaveBeenCalledWith(
      'Task created',
      expect.objectContaining({
        prompt: 'x'.repeat(100),
        category: 'task',
        action: 'created',
      })
    );
    expect(module.innerLogger.info).toHaveBeenCalledWith(
      'Task status changed',
      expect.objectContaining({
        oldStatus: 'pending',
        newStatus: 'running',
      })
    );
    expect(module.innerLogger.info).toHaveBeenCalledWith(
      'API call',
      expect.objectContaining({
        endpoint: '/v1/tasks',
        method: 'POST',
        duration: 25,
        success: true,
      })
    );
    expect(module.innerLogger.info).toHaveBeenCalledWith(
      'Slack event',
      expect.objectContaining({
        eventType: 'message',
        userId: 'U123',
        channelId: 'C123',
      })
    );
    expect(module.innerLogger.info).toHaveBeenCalledWith(
      'User action',
      expect.objectContaining({
        userId: 'U123',
        action: 'approve',
        taskId: 'task-123',
        category: 'user',
        event_type: 'action',
      })
    );
    expect(module.innerLogger.info).toHaveBeenCalledWith(
      'System event',
      expect.objectContaining({
        event: 'booted',
        region: 'eu-west',
      })
    );
    expect(module.innerLogger.warn).toHaveBeenCalledWith(
      'Security event',
      expect.objectContaining({
        event: 'permission_denied',
        userId: 'U123',
        channelId: 'C123',
      })
    );
    expect(module.innerLogger.info).toHaveBeenCalledWith(
      'Database operation',
      expect.objectContaining({
        operation: 'insert',
        table: 'tasks',
        duration: 10,
        rows: 1,
      })
    );
    expect(module.innerLogger.info).toHaveBeenCalledWith(
      'Redis operation',
      expect.objectContaining({
        operation: 'set',
        key: 'task:123',
        duration: 5,
        cache: 'tasks',
      })
    );
  });

  it('logs health checks to info or error depending on status', async () => {
    const module = await loadLogModule('test');

    module.logHealthCheck('slack', 'healthy', { latency: 12 });
    module.logHealthCheck('orbit', 'unhealthy', { reason: 'timeout' });

    expect(module.innerLogger.info).toHaveBeenCalledWith(
      'Health check - slack',
      expect.objectContaining({
        service: 'slack',
        status: 'healthy',
        latency: 12,
        category: 'health',
      })
    );
    expect(module.innerLogger.error).toHaveBeenCalledWith(
      'Health check - orbit',
      expect.objectContaining({
        service: 'orbit',
        status: 'unhealthy',
        reason: 'timeout',
        category: 'health',
      })
    );
  });

  it('builds error context from the provided identifiers', async () => {
    const module = await loadLogModule('test');

    expect(module.createErrorContext('U123', 'task-123', 'C123')).toEqual({
      userId: 'U123',
      taskId: 'task-123',
      channelId: 'C123',
    });
    expect(module.createErrorContext()).toEqual({});
  });

  it('records performance metrics through timers and direct logging', async () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-04-09T10:00:00Z'));
    const module = (await loadLogModule('test')) as LoadedLogModule;

    module.logPerformance('sync', 42, { tasks: 3 });
    const timer = module.startTimer('bootstrap', { source: 'slack' });
    vi.setSystemTime(new Date('2026-04-09T10:00:00.125Z'));
    timer.end({ result: 'ok' });

    expect(module.innerLogger.info).toHaveBeenCalledWith(
      'Performance metric',
      expect.objectContaining({
        operation: 'sync',
        duration: 42,
        tasks: 3,
      })
    );
    expect(module.innerLogger.info).toHaveBeenLastCalledWith(
      'Performance metric',
      expect.objectContaining({
        operation: 'bootstrap',
        duration: 125,
        source: 'slack',
        result: 'ok',
      })
    );

    vi.useRealTimers();
  });
});
