import type { JsonValue, OrbitOptions, ThreadOptions } from "./protocol.js";
import { Thread } from "./thread.js";

const REQUIRED_ENV_VARS = ["CODEX_API_KEY"];

/**
 * The top-level Orbit client. Wraps the `orbit` CLI and is used to start and
 * resume conversation {@link Thread}s.
 */
export class Orbit {
  readonly options: OrbitOptions;
  readonly command: string;

  constructor(options: OrbitOptions = {}) {
    this.options = options;
    this.command = options.command ?? "orbit";
  }

  /** Start a fresh conversation thread. */
  startThread(threadOptions: ThreadOptions = {}): Thread {
    return new Thread(this, threadOptions, undefined);
  }

  /** Reconstruct a thread from a previously persisted session id. */
  resumeThread(sessionId: string, threadOptions: ThreadOptions = {}): Thread {
    return new Thread(this, threadOptions, sessionId);
  }

  /**
   * Build the environment for a spawned CLI. Starts from `process.env`, applies
   * the user-provided `env`, then injects any required variables (such as
   * `CODEX_API_KEY`) from the ambient environment if still missing.
   */
  buildEnv(extra?: Record<string, string>): NodeJS.ProcessEnv {
    const base: NodeJS.ProcessEnv = { ...process.env };
    if (this.options.env) {
      Object.assign(base, this.options.env);
    }
    if (extra) {
      Object.assign(base, extra);
    }
    for (const varName of REQUIRED_ENV_VARS) {
      if (!base[varName] && process.env[varName]) {
        base[varName] = process.env[varName];
      }
    }
    return base;
  }
}

export type { JsonValue };
