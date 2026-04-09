import { beforeEach, describe, expect, it, vi } from 'vitest';

const { MockWebSocket, wsInstances } = vi.hoisted(() => {
  const wsInstances: Array<InstanceType<typeof MockWebSocket>> = [];

  class MockWebSocket {
    public static readonly OPEN = 1;
    public static readonly CLOSED = 3;
    public readonly url: string;
    public readyState = 0;
    private readonly handlers = new Map<string, Array<(...args: unknown[]) => void>>();
    private readonly onceHandlers = new Map<string, Array<(...args: unknown[]) => void>>();

    constructor(url: string) {
      this.url = url;
      wsInstances.push(this);
    }

    on(event: string, handler: (...args: unknown[]) => void): void {
      const handlers = this.handlers.get(event) ?? [];
      handlers.push(handler);
      this.handlers.set(event, handlers);
    }

    once(event: string, handler: (...args: unknown[]) => void): void {
      const handlers = this.onceHandlers.get(event) ?? [];
      handlers.push(handler);
      this.onceHandlers.set(event, handlers);
    }

    close(): void {
      this.readyState = MockWebSocket.CLOSED;
      this.emit('close');
    }

    emit(event: string, ...args: unknown[]): void {
      const handlers = this.handlers.get(event) ?? [];
      for (const handler of handlers) {
        handler(...args);
      }

      const onceHandlers = this.onceHandlers.get(event) ?? [];
      this.onceHandlers.delete(event);
      for (const handler of onceHandlers) {
        handler(...args);
      }
    }
  }

  return { MockWebSocket, wsInstances };
});

vi.mock('ws', () => ({
  default: MockWebSocket,
}));

import { logger } from '../src/log';
import { OrbitEventsClient } from '../src/orbit-events';
import type { OrbitEventEnvelope, OrbitTrackedTask } from '../src/types';

type TestableOrbitEventsClient = OrbitEventsClient & {
  buildSocketUrl(): string;
  handleMessage(payload: string): void;
  dispatchEvent(event: OrbitEventEnvelope, hintedTask?: OrbitTrackedTask): void;
  readTrackedTaskFromEvent(event: OrbitEventEnvelope): OrbitTrackedTask | undefined;
  scheduleReconnect(): void;
  recentEvents: Map<string, OrbitEventEnvelope[]>;
  reconnectTimer?: NodeJS.Timeout;
  shouldReconnect: boolean;
  restartRequested: boolean;
  socket?: InstanceType<typeof MockWebSocket>;
};

function createEventEnvelope(overrides: Partial<OrbitEventEnvelope> = {}): OrbitEventEnvelope {
  return {
    event_id: 'evt-123',
    topic: 'lane',
    event: 'lane.started',
    status: 'running',
    emittedAt: '2026-04-09T10:00:00Z',
    task_id: 'task-123',
    lane_id: 'lane-123',
    payload: {
      channel_id: 'C123',
      thread_ts: '1710000000.100',
      user_id: 'U123',
      worker_status: 'running',
    },
    ...overrides,
  };
}

describe('OrbitEventsClient', () => {
  let client: TestableOrbitEventsClient;
  let urlBuilder: ReturnType<typeof vi.fn>;

  beforeEach(() => {
    wsInstances.length = 0;
    vi.useRealTimers();
    urlBuilder = vi.fn().mockReturnValue('ws://localhost:8787/v1/events/ws?source=slack');
    client = new OrbitEventsClient(urlBuilder) as unknown as TestableOrbitEventsClient;
  });

  it('builds a slack-scoped websocket subscription URL', () => {
    const url = client.buildSocketUrl();

    expect(urlBuilder).toHaveBeenCalledWith({ source: 'slack' });
    expect(url).toBe('ws://localhost:8787/v1/events/ws?source=slack');
  });

  it('opens and closes the hosted event socket', async () => {
    const connectPromise = client.connect();
    const socket = wsInstances[0];
    socket.emit('open');
    await connectPromise;

    expect(client.shouldReconnect).toBe(true);
    expect(socket.url).toBe('ws://localhost:8787/v1/events/ws?source=slack');

    await client.disconnect();

    expect(client.shouldReconnect).toBe(false);
    expect(client.socket).toBeUndefined();
    expect(socket.readyState).toBe(MockWebSocket.CLOSED);
  });

  it('routes websocket message events through the hosted event handler pipeline', async () => {
    const handler = vi.fn();
    client.onTrackedTaskEvent(handler);

    const connectPromise = client.connect();
    const socket = wsInstances[0];
    socket.emit('open');
    await connectPromise;

    socket.emit('message', Buffer.from(JSON.stringify(createEventEnvelope())));

    expect(handler).toHaveBeenCalledWith(
      expect.objectContaining({
        event_id: 'evt-123',
        task_id: 'task-123',
      }),
      expect.objectContaining({
        taskId: 'task-123',
        channelId: 'C123',
      })
    );
  });

  it('retains only the most recent buffered events per task', () => {
    for (let index = 0; index < 14; index += 1) {
      client.handleMessage(
        JSON.stringify(
          createEventEnvelope({
            event_id: `evt-${index}`,
            emittedAt: `2026-04-09T10:00:${String(index).padStart(2, '0')}Z`,
          })
        )
      );
    }

    const bufferedEvents = client.recentEvents.get('task-123');

    expect(bufferedEvents).toHaveLength(12);
    expect(bufferedEvents?.[0]).toEqual(
      expect.objectContaining({
        event_id: 'evt-2',
      })
    );
  });

  it('hydrates task routing hints from hosted event payloads', () => {
    const handler = vi.fn();
    const event = createEventEnvelope();
    client.onTrackedTaskEvent(handler);

    client.handleMessage(JSON.stringify(event));

    expect(handler).toHaveBeenCalledWith(
      event,
      expect.objectContaining({
        taskId: 'task-123',
        channelId: 'C123',
        threadTs: '1710000000.100',
        userId: 'U123',
      })
    );
  });

  it('reuses cached task hints for later events with thin payloads', () => {
    const handler = vi.fn();
    client.onTrackedTaskEvent(handler);

    client.handleMessage(JSON.stringify(createEventEnvelope()));
    client.handleMessage(
      JSON.stringify(
        createEventEnvelope({
          event_id: 'evt-124',
          emittedAt: '2026-04-09T10:00:01Z',
          payload: {
            worker_status: 'running',
          },
        })
      )
    );

    expect(handler).toHaveBeenLastCalledWith(
      expect.objectContaining({
        event_id: 'evt-124',
      }),
      expect.objectContaining({
        taskId: 'task-123',
        channelId: 'C123',
      })
    );
  });

  it('replays buffered events immediately when a task is tracked later', () => {
    const dispatchSpy = vi.spyOn(client, 'dispatchEvent');
    client.recentEvents.set('task-123', [createEventEnvelope()]);

    client.trackTask({
      taskId: 'task-123',
      channelId: 'C123',
      threadTs: '1710000000.100',
      userId: 'U123',
    });

    expect(dispatchSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        event_id: 'evt-123',
      }),
      expect.objectContaining({
        taskId: 'task-123',
        channelId: 'C123',
      })
    );
  });

  it('deduplicates replayed events with the same hosted event key', () => {
    const handler = vi.fn();
    const event = createEventEnvelope();
    client.onTrackedTaskEvent(handler);

    client.dispatchEvent(event);
    client.dispatchEvent(event);

    expect(handler).toHaveBeenCalledTimes(1);
  });

  it('deduplicates replayed events even when lane ids are absent', () => {
    const handler = vi.fn();
    const event = createEventEnvelope({
      lane_id: undefined,
    });
    client.onTrackedTaskEvent(handler);

    client.dispatchEvent(event);
    client.dispatchEvent(event);

    expect(handler).toHaveBeenCalledTimes(1);
  });

  it('deduplicates replayed events when lane_id is explicitly null', () => {
    const handler = vi.fn();
    const event = createEventEnvelope({
      lane_id: null as unknown as string,
    });
    client.onTrackedTaskEvent(handler);

    client.dispatchEvent(event);
    client.dispatchEvent(event);

    expect(handler).toHaveBeenCalledTimes(1);
  });

  it('does not deduplicate events that only differ by lane id', () => {
    const handler = vi.fn();
    client.onTrackedTaskEvent(handler);

    client.dispatchEvent(
      createEventEnvelope({
        emittedAt: '2026-04-09T10:00:00Z',
        lane_id: 'lane-1',
      })
    );
    client.dispatchEvent(
      createEventEnvelope({
        event_id: 'evt-124',
        emittedAt: '2026-04-09T10:00:00Z',
        lane_id: 'lane-2',
      })
    );

    expect(handler).toHaveBeenCalledTimes(2);
  });

  it('does not deduplicate events that only differ by status', () => {
    const handler = vi.fn();
    client.onTrackedTaskEvent(handler);

    client.dispatchEvent(
      createEventEnvelope({
        emittedAt: '2026-04-09T10:00:00Z',
        lane_id: 'lane-123',
        status: 'running',
      })
    );
    client.dispatchEvent(
      createEventEnvelope({
        event_id: 'evt-125',
        emittedAt: '2026-04-09T10:00:00Z',
        lane_id: 'lane-123',
        status: 'failed',
      })
    );

    expect(handler).toHaveBeenCalledTimes(2);
  });

  it('does not deduplicate events that only differ by event name', () => {
    const handler = vi.fn();
    client.onTrackedTaskEvent(handler);

    client.dispatchEvent(
      createEventEnvelope({
        event: 'lane.started',
        status: 'running',
        emittedAt: '2026-04-09T10:00:00Z',
        lane_id: 'lane-123',
      })
    );
    client.dispatchEvent(
      createEventEnvelope({
        event_id: 'evt-126',
        event: 'lane.failed',
        status: 'running',
        emittedAt: '2026-04-09T10:00:00Z',
        lane_id: 'lane-123',
      })
    );

    expect(handler).toHaveBeenCalledTimes(2);
  });

  it('supports unregistering tracked task handlers', () => {
    const handler = vi.fn();
    const dispose = client.onTrackedTaskEvent(handler);

    dispose();
    client.dispatchEvent(createEventEnvelope());

    expect(handler).not.toHaveBeenCalled();
  });

  it('replays tracked task events with the explicit tracked task hint', () => {
    const handler = vi.fn();
    const trackedTask: OrbitTrackedTask = {
      taskId: 'task-123',
      channelId: 'C999',
      threadTs: '1710000000.500',
      userId: 'U999',
    };
    client.onTrackedTaskEvent(handler);

    client.dispatchEvent(
      createEventEnvelope({
        event_id: 'evt-456',
        emittedAt: '2026-04-09T10:00:01Z',
      }),
      trackedTask
    );

    expect(handler).toHaveBeenCalledWith(expect.any(Object), trackedTask);
  });

  it('logs handler failures without stopping dispatch', () => {
    const errorSpy = vi.spyOn(logger, 'error').mockImplementation(() => {});
    const failingHandler = vi.fn().mockRejectedValue(new Error('boom'));
    const nextHandler = vi.fn();
    client.onTrackedTaskEvent(failingHandler);
    client.onTrackedTaskEvent(nextHandler);

    client.dispatchEvent(createEventEnvelope());

    expect(nextHandler).toHaveBeenCalled();
    return Promise.resolve()
      .then(() => Promise.resolve())
      .then(() => {
        expect(errorSpy).toHaveBeenCalledWith(
          'Orbit hosted event handler failed',
          expect.any(Error),
          expect.objectContaining({
            event: 'lane.started',
            taskId: 'task-123',
          })
        );
      });
  });

  it('ignores malformed hosted event payloads', () => {
    const warnSpy = vi.spyOn(logger, 'warn').mockImplementation(() => {});
    const handler = vi.fn();
    client.onTrackedTaskEvent(handler);

    client.handleMessage('not-json');

    expect(handler).not.toHaveBeenCalled();
    expect(warnSpy).toHaveBeenCalledWith('Ignoring malformed Orbit hosted event payload', {
      payload: 'not-json',
    });
  });

  it('ignores events without a task id', () => {
    const handler = vi.fn();
    client.onTrackedTaskEvent(handler);

    client.handleMessage(
      JSON.stringify(
        createEventEnvelope({
          task_id: undefined,
        })
      )
    );

    expect(handler).not.toHaveBeenCalled();
  });

  it('dispatches without a tracked task when the payload lacks a Slack channel id', () => {
    const handler = vi.fn();
    const event = createEventEnvelope({
      payload: {
        thread_ts: '1710000000.100',
      },
    });
    client.onTrackedTaskEvent(handler);

    client.dispatchEvent(event);

    expect(handler).toHaveBeenCalledWith(event, undefined);
  });

  it('reads a sparse tracked task from an event payload when only channel routing is present', () => {
    const task = client.readTrackedTaskFromEvent(
      createEventEnvelope({
        payload: {
          channel_id: 'C999',
        },
      })
    );

    expect(task).toEqual({
      taskId: 'task-123',
      channelId: 'C999',
      threadTs: undefined,
      userId: undefined,
    });
  });

  it('returns no tracked task from events without payload routing details', () => {
    expect(
      client.readTrackedTaskFromEvent(
        createEventEnvelope({
          payload: undefined,
        })
      )
    ).toBeUndefined();

    expect(
      client.readTrackedTaskFromEvent(
        createEventEnvelope({
          task_id: undefined,
          payload: {
            channel_id: 'C123',
          },
        })
      )
    ).toBeUndefined();
  });

  it('dispatches without a tracked task when the event has no payload at all', () => {
    const handler = vi.fn();
    const event = createEventEnvelope({
      payload: undefined,
    });
    client.onTrackedTaskEvent(handler);

    client.dispatchEvent(event);

    expect(handler).toHaveBeenCalledWith(event, undefined);
  });

  it('ignores direct dispatch calls when the event has no task id', () => {
    const handler = vi.fn();
    client.onTrackedTaskEvent(handler);

    client.dispatchEvent(
      createEventEnvelope({
        task_id: undefined,
      })
    );

    expect(handler).not.toHaveBeenCalled();
  });

  it('clears replay buffers when tasks are untracked', () => {
    const handler = vi.fn();
    client.onTrackedTaskEvent(handler);
    client.handleMessage(JSON.stringify(createEventEnvelope()));

    client.untrackTask('task-123');
    handler.mockClear();

    client.trackTask({
      taskId: 'task-123',
      channelId: 'C123',
    });

    expect(handler).not.toHaveBeenCalled();
  });

  it('evicts the oldest seen event key when the dedupe cache reaches capacity', () => {
    const handler = vi.fn();
    client.onTrackedTaskEvent(handler);

    for (let index = 0; index < 513; index += 1) {
      client.dispatchEvent(
        createEventEnvelope({
          event_id: `evt-${index}`,
          emittedAt: `2026-04-09T10:${String(Math.floor(index / 60)).padStart(2, '0')}:${String(index % 60).padStart(2, '0')}Z`,
          lane_id: `lane-${index}`,
        })
      );
    }

    expect(handler).toHaveBeenCalledTimes(513);

    client.dispatchEvent(
      createEventEnvelope({
        event_id: 'evt-0-replay',
        emittedAt: '2026-04-09T10:00:00Z',
        lane_id: 'lane-0',
      })
    );

    expect(handler).toHaveBeenCalledTimes(514);
  });

  it('schedules a reconnect after socket close when reconnecting is enabled', async () => {
    vi.useFakeTimers();
    const connectPromise = client.connect();
    const firstSocket = wsInstances[0];
    firstSocket.emit('open');
    await connectPromise;

    firstSocket.emit('close');
    expect(wsInstances).toHaveLength(1);

    vi.advanceTimersByTime(2_000);
    expect(wsInstances).toHaveLength(2);
  });

  it('cancels a pending reconnect timer during disconnect', async () => {
    vi.useFakeTimers();
    const connectPromise = client.connect();
    const firstSocket = wsInstances[0];
    firstSocket.emit('open');
    await connectPromise;

    firstSocket.emit('close');
    expect(wsInstances).toHaveLength(1);

    await client.disconnect();
    vi.advanceTimersByTime(2_000);

    expect(client.reconnectTimer).toBeUndefined();
    expect(wsInstances).toHaveLength(1);
  });

  it('does not schedule duplicate reconnect timers', () => {
    vi.useFakeTimers();
    client.shouldReconnect = true;

    client.scheduleReconnect();
    client.scheduleReconnect();

    vi.advanceTimersByTime(2_000);
    expect(wsInstances).toHaveLength(1);
  });

  it('does not reconnect when reconnecting has been disabled before the timer fires', () => {
    vi.useFakeTimers();
    client.shouldReconnect = true;

    client.scheduleReconnect();
    client.shouldReconnect = false;
    vi.advanceTimersByTime(2_000);

    expect(client.reconnectTimer).toBeUndefined();
    expect(wsInstances).toHaveLength(0);
  });

  it('reschedules reconnect when a reconnect attempt fails to open', async () => {
    vi.useFakeTimers();
    const errorSpy = vi.spyOn(logger, 'error').mockImplementation(() => {});
    const connectPromise = client.connect();
    const firstSocket = wsInstances[0];
    firstSocket.emit('open');
    await connectPromise;

    firstSocket.emit('close');
    vi.advanceTimersByTime(2_000);

    const reconnectSocket = wsInstances[1];
    reconnectSocket.emit('error', new Error('reconnect failed'));
    await Promise.resolve();
    await Promise.resolve();

    expect(
      errorSpy.mock.calls.some(
        ([message, error]) =>
          message === 'Orbit hosted events reconnect failed' && error instanceof Error
      )
    ).toBe(true);

    vi.advanceTimersByTime(2_000);
    expect(wsInstances).toHaveLength(3);
  });

  it('restarts immediately on close when a restart was requested', async () => {
    const connectPromise = client.connect();
    const firstSocket = wsInstances[0];
    firstSocket.emit('open');
    await connectPromise;

    client.restartRequested = true;
    firstSocket.emit('close');

    expect(wsInstances).toHaveLength(2);
    expect(client.restartRequested).toBe(false);
  });

  it('logs restart failures and schedules a reconnect when immediate restart open fails', async () => {
    vi.useFakeTimers();
    const errorSpy = vi.spyOn(logger, 'error').mockImplementation(() => {});
    const connectPromise = client.connect();
    const firstSocket = wsInstances[0];
    firstSocket.emit('open');
    await connectPromise;

    client.restartRequested = true;
    firstSocket.emit('close');

    const restartSocket = wsInstances[1];
    restartSocket.emit('error', new Error('restart failed'));
    await Promise.resolve();
    await Promise.resolve();

    expect(
      errorSpy.mock.calls.some(
        ([message, error]) =>
          message === 'Orbit hosted events stream restart failed' && error instanceof Error
      )
    ).toBe(true);

    vi.advanceTimersByTime(2_000);
    expect(wsInstances).toHaveLength(3);
  });
});
