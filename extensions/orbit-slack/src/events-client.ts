import WebSocket from "ws";
import { config } from "./config";
import { logger } from "./log";
import type { OrbitEventEnvelope } from "./types";

type EventHandler = (event: OrbitEventEnvelope) => void;

export class OrbitEventsClient {
  private ws?: WebSocket;
  private handlers: EventHandler[] = [];
  private readonly url: string;
  private reconnectTimer?: NodeJS.Timeout;

  constructor() {
    this.url = this.buildWsUrl();
  }

  connect(): void {
    if (this.ws && this.ws.readyState === WebSocket.OPEN) {
      return;
    }
    this.ws = new WebSocket(
      this.url,
      config.orbit.apiKey
        ? {
            headers: {
              "x-api-key": config.orbit.apiKey,
            },
          }
        : undefined
    );
    this.ws.on("open", () => {
      logger.info("Connected to Orbit event stream", { url: this.url });
    });
    this.ws.on("message", (payload) => {
      try {
        const event = JSON.parse(payload.toString()) as OrbitEventEnvelope;
        for (const handler of this.handlers) {
          handler(event);
        }
      } catch (error) {
        logger.error("Failed to parse Orbit event", error as Error);
      }
    });
    this.ws.on("close", (code, reason) => {
      logger.warn("Orbit event stream closed", {
        code,
        reason: reason.toString(),
      });
      this.scheduleReconnect();
    });
    this.ws.on("error", (error) => {
      logger.error("Orbit event stream error", error as Error);
      this.scheduleReconnect();
    });
  }

  close(): void {
    if (this.ws) {
      this.ws.close();
      this.ws = undefined;
    }
    if (this.reconnectTimer) {
      clearTimeout(this.reconnectTimer);
    }
  }

  onEvent(handler: EventHandler): void {
    this.handlers.push(handler);
  }

  private scheduleReconnect(): void {
    if (this.reconnectTimer) {
      return;
    }
    this.reconnectTimer = setTimeout(() => {
      this.reconnectTimer = undefined;
      this.connect();
    }, 2000);
  }

  private buildWsUrl(): string {
    const base = config.orbit.apiUrl.replace(/\/+$/, "");
    const wsBase = base.replace(/^http/, (match) =>
      match === "https" ? "wss" : "ws"
    );
    return `${wsBase}/v1/events/ws`;
  }
}
