import { beforeEach, describe, expect, it, vi } from 'vitest';

const interceptorState = vi.hoisted(() => {
  type RequestSuccess = (value: unknown) => unknown;
  type RequestError = (error: unknown) => Promise<never>;
  type ResponseSuccess = (value: unknown) => unknown;
  type ResponseError = (error: unknown) => Promise<never>;

  const captured: {
    requestSuccess?: RequestSuccess;
    requestError?: RequestError;
    responseSuccess?: ResponseSuccess;
    responseError?: ResponseError;
  } = {};

  const mockInstance = {
    get: vi.fn(),
    post: vi.fn(),
    interceptors: {
      request: {
        use: vi.fn((onFulfilled: RequestSuccess, onRejected: RequestError) => {
          captured.requestSuccess = onFulfilled;
          captured.requestError = onRejected;
          return 0;
        }),
      },
      response: {
        use: vi.fn((onFulfilled: ResponseSuccess, onRejected: ResponseError) => {
          captured.responseSuccess = onFulfilled;
          captured.responseError = onRejected;
          return 0;
        }),
      },
    },
  };

  const create = vi.fn(() => mockInstance);

  class MockAxiosHeaders {
    constructor(init: Record<string, unknown> = {}) {
      Object.assign(this, init);
    }
  }

  return { captured, create, mockInstance, MockAxiosHeaders };
});

const logState = vi.hoisted(() => ({
  logApiCall: vi.fn(),
  logger: {
    debug: vi.fn(),
    error: vi.fn(),
  },
}));

vi.mock('axios', () => ({
  default: {
    create: interceptorState.create,
  },
  AxiosHeaders: interceptorState.MockAxiosHeaders,
}));

vi.mock('../src/log', () => ({
  logApiCall: logState.logApiCall,
  logger: logState.logger,
}));

vi.mock('../src/config', () => ({
  config: {
    orbit: {
      apiUrl: 'http://localhost:8787',
      timeout: 30_000,
    },
  },
}));

import { OrbitApiClient } from '../src/api-client';

describe('OrbitApiClient interceptors', () => {
  beforeEach(() => {
    vi.clearAllMocks();
    interceptorState.captured.requestSuccess = undefined;
    interceptorState.captured.requestError = undefined;
    interceptorState.captured.responseSuccess = undefined;
    interceptorState.captured.responseError = undefined;
  });

  it('constructs the axios client with configured base URL, timeout, and default headers', () => {
    new OrbitApiClient();

    expect(interceptorState.create).toHaveBeenCalledWith({
      baseURL: 'http://localhost:8787',
      timeout: 30_000,
      headers: {
        'Content-Type': 'application/json',
        'User-Agent': 'orbit-slack-bot/1.0.0',
      },
    });
  });

  it('logs request metadata and returns the request config from the request interceptor', () => {
    new OrbitApiClient();
    const config = {
      method: 'post',
      url: '/v1/tasks',
    };

    expect(interceptorState.captured.requestSuccess?.(config)).toBe(config);
    expect(logState.logger.debug).toHaveBeenCalledWith('Orbit API request', {
      method: 'post',
      url: '/v1/tasks',
    });
  });

  it('logs and rethrows request setup failures from the request interceptor', async () => {
    new OrbitApiClient();
    const error = new Error('bad config');

    await expect(interceptorState.captured.requestError?.(error)).rejects.toBe(error);
    expect(logState.logger.error).toHaveBeenCalledWith('Orbit API request error', error);
  });

  it('logs successful API responses with computed duration from the response interceptor', () => {
    new OrbitApiClient();
    vi.spyOn(Date, 'now').mockReturnValue(2_500);
    const response = {
      config: {
        url: '/v1/tasks',
        method: 'get',
        headers: {
          'X-Start-Time': '1000',
        },
      },
    };

    expect(interceptorState.captured.responseSuccess?.(response)).toBe(response);
    expect(logState.logApiCall).toHaveBeenCalledWith('/v1/tasks', 'GET', 1500, true);
  });

  it('falls back to zero duration when the response success path has no start-time header', () => {
    new OrbitApiClient();
    const response = {
      config: {
        url: '/v1/tasks',
        method: 'get',
        headers: {},
      },
    };

    expect(interceptorState.captured.responseSuccess?.(response)).toBe(response);
    expect(logState.logApiCall).toHaveBeenCalledWith('/v1/tasks', 'GET', 0, true);
  });

  it('falls back to empty url and GET when the response success path lacks method and url', () => {
    new OrbitApiClient();
    const response = {
      config: {
        headers: {
          'X-Start-Time': '1000',
        },
      },
    };
    vi.spyOn(Date, 'now').mockReturnValue(2_500);

    expect(interceptorState.captured.responseSuccess?.(response)).toBe(response);
    expect(logState.logApiCall).toHaveBeenCalledWith('', 'GET', 1500, true);
  });

  it('logs failed API responses with computed duration and rethrows the same error', async () => {
    new OrbitApiClient();
    vi.spyOn(Date, 'now').mockReturnValue(4_000);
    const error = Object.assign(new Error('down'), {
      config: {
        url: '/v1/tasks/123',
        method: 'post',
        headers: {
          'X-Start-Time': '2500',
        },
      },
    });

    await expect(interceptorState.captured.responseError?.(error)).rejects.toBe(error);
    expect(logState.logApiCall).toHaveBeenCalledWith('/v1/tasks/123', 'POST', 1500, false);
  });

  it('falls back to empty url, GET, and zero duration when the response error lacks config', async () => {
    new OrbitApiClient();
    const error = new Error('down');

    await expect(interceptorState.captured.responseError?.(error)).rejects.toBe(error);
    expect(logState.logApiCall).toHaveBeenCalledWith('', 'GET', 0, false);
  });
});
