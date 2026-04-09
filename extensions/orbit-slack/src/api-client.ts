import axios, {
  type AxiosInstance,
  type AxiosResponse,
  type InternalAxiosRequestConfig,
  AxiosHeaders,
} from 'axios';
import { config } from './config';
import { logApiCall, logger } from './log';
import type {
  OrbitCliRequest,
  OrbitCliResponse,
  OrbitCreateTaskRequest,
  OrbitCreateTaskResponse,
  OrbitEventStreamQuery,
  OrbitListTasksQuery,
  OrbitOrphanPolicyQuery,
  OrbitOrphanPolicyResponse,
  OrbitPromptRequest,
  OrbitResolveApprovalRequest,
  OrbitSandboxResponse,
  OrbitStatusResponse,
  OrbitTask,
  OrbitUpdateTaskContextRequest,
  SlackBlock,
  SlackBody,
} from './types';

export class OrbitApiClient {
  private readonly client: AxiosInstance;
  private readonly baseUrl: string;
  private readonly timeout: number;

  constructor() {
    this.baseUrl = config.orbit.apiUrl;
    this.timeout = config.orbit.timeout;

    this.client = axios.create({
      baseURL: this.baseUrl,
      timeout: this.timeout,
      headers: {
        'Content-Type': 'application/json',
        'User-Agent': 'orbit-slack-bot/1.0.0',
      },
    });

    // Request interceptor for logging
    this.client.interceptors.request.use(
      (config: InternalAxiosRequestConfig) => {
        logger.debug('Orbit API request', { method: config.method, url: config.url });
        return config;
      },
      (error) => {
        logger.error('Orbit API request error', error as Error);
        return Promise.reject(error);
      }
    );

    // Response interceptor for logging
    this.client.interceptors.response.use(
      (response) => {
        const startTime = response.config.headers?.['X-Start-Time']
          ? Number.parseInt(response.config.headers['X-Start-Time'] as string)
          : 0;
        const duration = startTime ? Date.now() - startTime : 0;
        logApiCall(
          response.config.url || '',
          response.config.method?.toUpperCase() || 'GET',
          duration,
          true
        );
        return response;
      },
      (error) => {
        const startTime = error.config?.headers?.['X-Start-Time']
          ? Number.parseInt(error.config.headers['X-Start-Time'] as string)
          : 0;
        const duration = startTime ? Date.now() - startTime : 0;
        logApiCall(
          error.config?.url || '',
          error.config?.method?.toUpperCase() || 'GET',
          duration,
          false
        );
        return Promise.reject(error);
      }
    );
  }

  async submitPrompt(request: OrbitPromptRequest): Promise<OrbitCliResponse> {
    const startTime = Date.now();

    try {
      const response: AxiosResponse<OrbitCliResponse> = await this.client.post(
        '/v1/prompt',
        request,
        {
          headers: new AxiosHeaders({
            'X-Start-Time': startTime.toString(),
          }),
        } as InternalAxiosRequestConfig
      );

      return response.data;
    } catch (error) {
      logger.error('Failed to submit prompt to Orbit API', error as Error);
      throw new Error(`Orbit API error: ${(error as Error).message}`);
    }
  }

  async runCliCommand(request: OrbitCliRequest): Promise<OrbitCliResponse> {
    const startTime = Date.now();

    try {
      const response: AxiosResponse<OrbitCliResponse> = await this.client.post(
        '/v1/cli/run',
        request,
        {
          headers: new AxiosHeaders({
            'X-Start-Time': startTime.toString(),
          }),
        } as InternalAxiosRequestConfig
      );

      return response.data;
    } catch (error) {
      logger.error('Failed to run CLI command via Orbit API', error as Error);
      throw new Error(`Orbit API error: ${(error as Error).message}`);
    }
  }

  async getStatus(): Promise<OrbitStatusResponse> {
    const startTime = Date.now();

    try {
      const response: AxiosResponse<OrbitStatusResponse> = await this.client.get('/v1/status', {
        headers: new AxiosHeaders({
          'X-Start-Time': startTime.toString(),
        }),
      } as InternalAxiosRequestConfig);

      return response.data;
    } catch (error) {
      logger.error('Failed to get status from Orbit API', error as Error);
      throw new Error(`Orbit API error: ${(error as Error).message}`);
    }
  }

  async getSandboxStatus(): Promise<OrbitSandboxResponse> {
    const startTime = Date.now();

    try {
      const response: AxiosResponse<OrbitSandboxResponse> = await this.client.get('/v1/sandbox', {
        headers: new AxiosHeaders({
          'X-Start-Time': startTime.toString(),
        }),
      } as InternalAxiosRequestConfig);

      return response.data;
    } catch (error) {
      logger.error('Failed to get sandbox status from Orbit API', error as Error);
      throw new Error(`Orbit API error: ${(error as Error).message}`);
    }
  }

  async getVersion(): Promise<{ version: string; commit: string; build_time: string }> {
    const startTime = Date.now();

    try {
      const response = await this.client.get('/v1/version', {
        headers: new AxiosHeaders({
          'X-Start-Time': startTime.toString(),
        }),
      } as InternalAxiosRequestConfig);

      return response.data;
    } catch (error) {
      logger.error('Failed to get version from Orbit API', error as Error);
      throw new Error(`Orbit API error: ${(error as Error).message}`);
    }
  }

  async healthCheck(): Promise<boolean> {
    try {
      const response = await this.client.get('/health');
      return response.status === 200;
    } catch (error) {
      logger.error('Orbit API health check failed', error as Error);
      return false;
    }
  }

  async createTask(request: OrbitCreateTaskRequest): Promise<OrbitCreateTaskResponse> {
    try {
      const response: AxiosResponse<OrbitCreateTaskResponse> = await this.client.post(
        '/v1/tasks',
        request
      );
      return response.data;
    } catch (error) {
      logger.error('Task creation failed', error as Error);
      throw error;
    }
  }

  async getTask(taskId: string): Promise<OrbitTask> {
    try {
      const response: AxiosResponse<OrbitTask> = await this.client.get(`/v1/tasks/${taskId}`);
      return response.data;
    } catch (error) {
      logger.error('Task lookup failed', error as Error, { taskId });
      throw error;
    }
  }

  async listTasks(query: OrbitListTasksQuery = {}): Promise<OrbitTask[]> {
    try {
      const response: AxiosResponse<OrbitTask[]> = await this.client.get('/v1/tasks', {
        params: query,
      });
      return response.data;
    } catch (error) {
      logger.error('Task listing failed', error as Error, { ...query });
      throw error;
    }
  }

  async getOrphanPolicy(query: OrbitOrphanPolicyQuery = {}): Promise<OrbitOrphanPolicyResponse> {
    try {
      const response: AxiosResponse<OrbitOrphanPolicyResponse> = await this.client.get(
        '/v1/policies/orphans',
        {
          params: query,
        }
      );
      return response.data;
    } catch (error) {
      logger.error('Orphan policy lookup failed', error as Error, { ...query });
      throw error;
    }
  }

  async updateTaskContext(request: OrbitUpdateTaskContextRequest): Promise<OrbitTask> {
    try {
      const response: AxiosResponse<OrbitTask> = await this.client.post(
        `/v1/tasks/${request.taskId}/context`,
        {
          source: request.source,
          user_id: request.user_id,
          channel_id: request.channel_id,
          thread_ts: request.thread_ts,
          approval_message_ts: request.approval_message_ts,
        }
      );
      return response.data;
    } catch (error) {
      logger.error('Task context update failed', error as Error, { taskId: request.taskId });
      throw error;
    }
  }

  getEventsWebSocketUrl(query: OrbitEventStreamQuery = {}): string {
    const url = new URL(this.baseUrl);
    url.protocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
    url.pathname = '/v1/events/ws';
    url.search = '';
    for (const [key, value] of Object.entries(query)) {
      if (value === undefined || value === null || value === '') {
        continue;
      }
      url.searchParams.set(key, String(value));
    }
    url.hash = '';
    return url.toString();
  }

  async sendConnectorInteraction(
    connector: string,
    request: { action: string; value?: string; userId: string; context: SlackBody }
  ): Promise<{ blocks: SlackBlock[] }> {
    try {
      const response = await this.client.post(
        `/v1/connectors/${encodeURIComponent(connector)}/interactions`,
        {
          action: request.action,
          value: request.value,
          user_id: request.userId,
          context: request.context,
        }
      );
      return response.data;
    } catch (error) {
      logger.error('Slack interaction handling failed', error as Error);
      throw error;
    }
  }

  async resolveTaskApproval(request: OrbitResolveApprovalRequest): Promise<OrbitTask> {
    try {
      const response: AxiosResponse<OrbitTask> = await this.client.post(
        `/v1/tasks/${request.taskId}/approval`,
        {
          approval_kind: request.approvalKind,
          action: request.action,
          resolved_by: request.resolvedBy,
          reason: request.reason,
        }
      );
      return response.data;
    } catch (error) {
      logger.error('Task approval resolution failed', error as Error, {
        taskId: request.taskId,
        approvalKind: request.approvalKind,
        action: request.action,
      });
      throw error;
    }
  }

  async sendConnectorEvent(
    connector: string,
    request: { type: string; userId: string; data: unknown }
  ): Promise<void> {
    try {
      await this.client.post(`/v1/connectors/${encodeURIComponent(connector)}/events`, {
        type: request.type,
        user_id: request.userId,
        data: request.data,
      });
    } catch (error) {
      logger.error('Slack event processing failed', error as Error);
      throw error;
    }
  }

  async checkSandboxStatus(): Promise<OrbitSandboxResponse> {
    try {
      return await this.getSandboxStatus();
    } catch (error) {
      logger.error('Failed to check sandbox status', error as Error);
      throw error;
    }
  }
}

export default OrbitApiClient;
