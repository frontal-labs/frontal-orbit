import * as Sentry from '@sentry/node';
import winston from 'winston';
import { config } from './config';

type LogMeta = Record<string, unknown>;

// Initialize Sentry
if (config.app.nodeEnv === 'production') {
  Sentry.init({
    dsn: process.env.SENTRY_DSN,
    environment: config.app.nodeEnv,
    tracesSampleRate: 0.1,
    maxValueLength: 1000,
  });
}

// Initialize Winston logger
const winstonLogger = winston.createLogger({
  level: config.app.logLevel,
  format: winston.format.combine(
    winston.format.timestamp(),
    config.app.nodeEnv === 'production'
      ? winston.format.json()
      : winston.format.combine(winston.format.colorize(), winston.format.simple())
  ),
  transports: [new winston.transports.Console()],
});

// Custom logger interface
export interface Logger {
  debug(message: string, meta?: LogMeta): void;
  info(message: string, meta?: LogMeta): void;
  warn(message: string, meta?: LogMeta): void;
  error(message: string, error?: Error, meta?: LogMeta): void;
}

// Create logger instance
export const logger: Logger = {
  debug: (message: string, meta?: LogMeta) => {
    winstonLogger.debug(message, meta);
  },

  info: (message: string, meta?: LogMeta) => {
    winstonLogger.info(message, meta);
  },

  warn: (message: string, meta?: LogMeta) => {
    winstonLogger.warn(message, meta);
    // Send warnings to Sentry in production
    if (config.app.nodeEnv === 'production') {
      Sentry.captureMessage(message, 'warning');
    }
  },

  error: (message: string, error?: Error, meta?: LogMeta) => {
    winstonLogger.error(message, { ...meta, error: error?.message, stack: error?.stack });

    // Send errors to Sentry in production
    if (config.app.nodeEnv === 'production') {
      if (error) {
        Sentry.captureException(error);
      } else {
        Sentry.captureMessage(message, 'error');
      }
    }
  },
};

// Structured logging helpers
export const logTaskCreation = (taskId: string, userId: string, prompt: string): void => {
  logger.info('Task created', {
    taskId,
    userId,
    prompt: prompt.substring(0, 100),
    category: 'task',
    action: 'created',
  });
};

export const logTaskStatusChange = (taskId: string, oldStatus: string, newStatus: string): void => {
  logger.info('Task status changed', {
    taskId,
    oldStatus,
    newStatus,
    category: 'task',
    action: 'status_change',
  });
};

export const logApiCall = (
  endpoint: string,
  method: string,
  duration: number,
  success: boolean
): void => {
  logger.info('API call', {
    endpoint,
    method,
    duration,
    success,
    category: 'api',
    action: 'call',
  });
};

export const logSlackEvent = (eventType: string, userId: string, channelId: string): void => {
  logger.info('Slack event', {
    eventType,
    userId,
    channelId,
    category: 'slack',
    action: 'event',
  });
};

export const logUserAction = (userId: string, userAction: string, details?: LogMeta): void => {
  logger.info('User action', {
    userId,
    action: userAction,
    ...details,
    category: 'user',
    event_type: 'action',
  });
};

export const logSystemEvent = (event: string, details?: LogMeta): void => {
  logger.info('System event', {
    event,
    ...details,
    category: 'system',
    action: 'event',
  });
};

export const logSecurityEvent = (event: string, userId: string, details?: LogMeta): void => {
  logger.warn('Security event', {
    event,
    userId,
    ...details,
    category: 'security',
    action: 'event',
  });
};

// Performance logging
export const logPerformance = (operation: string, duration: number, details?: LogMeta): void => {
  logger.info('Performance metric', {
    operation,
    duration,
    ...details,
    category: 'performance',
    action: 'metric',
  });
};

// Health check logging
export const logHealthCheck = (
  service: string,
  status: 'healthy' | 'unhealthy',
  details?: LogMeta
): void => {
  if (status === 'healthy') {
    logger.info(`Health check - ${service}`, {
      service,
      status,
      ...details,
      category: 'health',
      action: 'check',
    });
  } else {
    logger.error(`Health check - ${service}`, undefined, {
      service,
      status,
      ...details,
      category: 'health',
      action: 'check',
    });
  }
};

// Database operation logging
export const logDatabaseOperation = (
  operation: string,
  table: string,
  duration?: number,
  details?: LogMeta
): void => {
  logger.info('Database operation', {
    operation,
    table,
    duration,
    ...details,
    category: 'database',
    action: operation,
  });
};

// Redis operation logging
export const logRedisOperation = (
  operation: string,
  key?: string,
  duration?: number,
  details?: LogMeta
): void => {
  logger.info('Redis operation', {
    operation,
    key,
    duration,
    ...details,
    category: 'redis',
    action: operation,
  });
};

// Error context builder
export const createErrorContext = (
  userId?: string,
  taskId?: string,
  channelId?: string
): LogMeta => {
  const context: LogMeta = {};

  if (userId) context.userId = userId;
  if (taskId) context.taskId = taskId;
  if (channelId) context.channelId = channelId;

  return context;
};

// Performance timer utility
export class PerformanceTimer {
  private readonly operation: string;
  private readonly startTime: number;
  private readonly details?: LogMeta;

  constructor(operation: string, details?: LogMeta) {
    this.operation = operation;
    this.startTime = Date.now();
    this.details = details;
  }

  end(additionalDetails?: LogMeta): void {
    const duration = Date.now() - this.startTime;
    logPerformance(this.operation, duration, { ...this.details, ...additionalDetails });
  }
}

// Create a performance timer
export const startTimer = (operation: string, details?: LogMeta): PerformanceTimer => {
  return new PerformanceTimer(operation, details);
};

// Export default logger
export default logger;
