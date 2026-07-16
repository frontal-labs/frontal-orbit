/**
 * Shared protocol types for the Orbit SDK.
 *
 * The SDK wraps the `orbit` CLI. The event names (`item.completed`,
 * `turn.completed`) and the `TurnResult` shape follow the SDK README; they are
 * mapped from the CLI's `--output-format json --stream` JSONL output.
 */

export type JsonPrimitive = string | number | boolean | null;
export type JsonValue = JsonPrimitive | JsonValue[] | { [key: string]: JsonValue };

/** A single item produced during a turn. */
export type ThreadItem =
  | { type: "text"; text: string }
  | { type: "tool_use"; name: string; input: string }
  | { type: "tool_result"; content: string }
  | { type: "image"; path?: string; url?: string };

/** Token usage reported by the CLI. */
export interface Usage {
  input_tokens: number;
  output_tokens: number;
  cache_creation_input_tokens: number;
  cache_read_input_tokens: number;
}

/** The buffered result of a completed turn. */
export interface TurnResult {
  finalResponse: string;
  items: ThreadItem[];
  usage?: Usage;
}

/**
 * Structured events emitted while a turn runs. `runStreamed` yields these;
 * `run` buffers them into a {@link TurnResult}.
 */
export type OrbitEvent =
  | { type: "turn.started" }
  | { type: "item.completed"; item: ThreadItem }
  | {
      type: "turn.completed";
      finalResponse: string;
      usage?: Usage;
      sessionId?: string;
    }
  | { type: "turn.failed"; error: string };

/** Structured input entry accepted by {@link Thread.run}. */
export type InputEntry =
  | { type: "text"; text: string }
  | { type: "local_image"; path: string };

/** A turn can be started with a plain prompt string or structured entries. */
export type ThreadInput = string | InputEntry[];

/** Options for constructing the {@link Orbit} client. */
export interface OrbitOptions {
  /** Environment passed to the spawned CLI. Merged over `process.env`. */
  env?: Record<string, string>;
  /** Base URL the CLI should use; passed as a `frontal_base_url` config override. */
  baseUrl?: string;
  /** Global CLI `--config` overrides. */
  config?: Record<string, JsonValue>;
  /** Path to the `orbit` CLI binary. Defaults to `"orbit"`. */
  command?: string;
}

/** Options for starting or resuming a {@link Thread}. */
export interface ThreadOptions {
  /** Run the CLI with this working directory. */
  workingDirectory?: string;
  /** Skip the CLI's Git repository check. */
  skipGitRepoCheck?: boolean;
  /** Provider override (e.g. `anthropic`, `frontal`). */
  provider?: string;
  /** Model override (e.g. `opus`, `claude-opus-4-6`). */
  model?: string;
  /** Permission mode override. */
  permissionMode?: string;
  /** Thread-level CLI `--config` overrides (take precedence over global). */
  config?: Record<string, JsonValue>;
}

/** Per-turn options for {@link Thread.run} / {@link Thread.runStreamed}. */
export interface ThreadRunOptions {
  /** JSON schema the agent should conform its response to. */
  outputSchema?: JsonValue;
  /** Run-level CLI `--config` overrides (highest precedence). */
  config?: Record<string, JsonValue>;
}
