import WebSocket from "ws";
import { logger } from "./log";
import type {
  OrbitEventEnvelope,
  OrbitEventStreamQuery,
  OrbitTrackedTask,
} from "./types";

type TrackedTaskHandler = (
  event: OrbitEventEnvelope,
  task?: OrbitTrackedTask
) => Promise<void> | void;
type OrbitEventsUrlBuilder = (query?: OrbitEventStreamQuery) => string;

const RECENT_EVENTS_PER_TASK = 12;
const MAX_SEEN_EVENT_KEYS = 512;
const RECONNECT_DELAY_MS = 2_000;

export class OrbitEventsClient {
  private readonly urlBuilder: OrbitEventsUrlBuilder;
  private readonly taskHints = new Map<string, OrbitTrackedTask>();
  private readonly handlers = new Set<TrackedTaskHandler>();
  private readonly recentEvents = new Map<string, OrbitEventEnvelope[]>();
  private readonly seenEventKeys = new Set<string>();
  private readonly seenEventOrder: string[] = [];
  private socket?: WebSocket;
  private reconnectTimer?: NodeJS.Timeout;
  private shouldReconnect = false;
  private restartRequested = false;
  private currentUrl?: string;

  constructor(urlBuilder: OrbitEventsUrlBuilder) {
    this.urlBuilder = urlBuilder;
  }

  async connect(): Promise<void> {
    this.shouldReconnect = true;
    await this.openSocket();
  }

  async disconnect(): Promise<void> {
    this.shouldReconnect = false;
    this.restartRequested = false;
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
      this.reconnectTimer = undefined;
    }

    const socket = this.socket;
    this.socket = undefined;
    if (!socket) {
      return;
    }

    await new Promise<void>((resolve) => {
      socket.once("close", () => resolve());
      socket.close();
    });
  }

  onTrackedTaskEvent(handler: TrackedTaskHandler): () => void {
    this.handlers.add(handler);
    return () => {
      this.handlers.delete(handler);
    };
  }

  trackTask(task: OrbitTrackedTask): void {
    this.taskHints.set(task.taskId, task);
    const recentEvents = this.recentEvents.get(task.taskId) ?? [];
    for (const event of recentEvents) {
      this.dispatchEvent(event, task);
    }
  }

  untrackTask(taskId: string): void {
    this.taskHints.delete(taskId);
    this.recentEvents.delete(taskId);
  }

  private async openSocket(): Promise<void> {
    const nextUrl = this.buildSocketUrl();
    this.currentUrl = nextUrl;
    await new Promise<void>((resolve, reject) => {
      const socket = new WebSocket(nextUrl);
      this.socket = socket;

      socket.once("open", () => {
        logger.info("Connected to Orbit hosted events stream", {
          url: nextUrl,
        });
        resolve();
      });

      socket.once("error", (error) => {
        logger.error("Orbit hosted events stream error", error as Error);
        reject(error);
      });

      socket.on("message", (data) => {
        this.handleMessage(data.toString());
      });

      socket.on("close", () => {
        logger.warn("Orbit hosted events stream disconnected");
        this.socket = undefined;
        if (this.shouldReconnect) {
          if (this.restartRequested) {
            this.restartRequested = false;
            this.openSocket().catch((error) => {
              logger.error(
                "Orbit hosted events stream restart failed",
                error as Error
              );
              this.scheduleReconnect();
            });
          } else {
            this.scheduleReconnect();
          }
        }
      });
    });
  }

  private buildSocketUrl(): string {
    return this.urlBuilder({
      source: "slack",
    });
  }

  private scheduleReconnect(): void {
    if (this.reconnectTimer) {
      return;
    }

    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = undefined;
      if (!this.shouldReconnect) {
        return;
      }
      this.openSocket().catch((error) => {
        logger.error("Orbit hosted events reconnect failed", error as Error);
        this.scheduleReconnect();
      });
    }, RECONNECT_DELAY_MS);
  }

  private handleMessage(payload: string): void {
    let event: OrbitEventEnvelope;
    try {
      event = JSON.parse(payload) as OrbitEventEnvelope;
    } catch (_error) {
      logger.warn("Ignoring malformed Orbit hosted event payload", { payload });
      return;
    }

    if (!event.task_id) {
      return;
    }

    const recent = this.recentEvents.get(event.task_id) ?? [];
    recent.push(event);
    if (recent.length > RECENT_EVENTS_PER_TASK) {
      recent.shift();
    }
    this.recentEvents.set(event.task_id, recent);

    this.dispatchEvent(event);
  }

  private dispatchEvent(
    event: OrbitEventEnvelope,
    hintedTask?: OrbitTrackedTask
  ): void {
    const taskId = event.task_id;
    if (!taskId) {
      return;
    }

    const task =
      hintedTask ||
      this.taskHints.get(taskId) ||
      this.readTrackedTaskFromEvent(event);
    if (task && !this.taskHints.has(taskId)) {
      this.taskHints.set(taskId, task);
    }

    const eventKey = `${event.event}:${event.status}:${event.emittedAt}:${event.task_id ?? ""}:${event.lane_id ?? ""}`;
    if (this.seenEventKeys.has(eventKey)) {
      return;
    }

    this.seenEventKeys.add(eventKey);
    this.seenEventOrder.push(eventKey);
    if (this.seenEventOrder.length > MAX_SEEN_EVENT_KEYS) {
      const oldest = this.seenEventOrder.shift();
      if (oldest) {
        this.seenEventKeys.delete(oldest);
      }
    }

    for (const handler of this.handlers) {
      void Promise.resolve(handler(event, task)).catch((error) => {
        logger.error("Orbit hosted event handler failed", error as Error, {
          event: event.event,
          taskId,
        });
      });
    }
  }

  private readTrackedTaskFromEvent(
    event: OrbitEventEnvelope
  ): OrbitTrackedTask | undefined {
    const payload = event.payload;
    if (!payload) {
      return undefined;
    }

    const channelId = payload.channel_id;
    if (!channelId || !event.task_id) {
      return undefined;
    }

    return {
      taskId: event.task_id,
      channelId,
      threadTs: payload.thread_ts,
      userId: payload.user_id,
    };
  }
}

export default OrbitEventsClient;
