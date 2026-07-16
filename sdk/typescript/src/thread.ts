import type {
  InputEntry,
  JsonValue,
  OrbitEvent,
  ThreadInput,
  ThreadOptions,
  ThreadRunOptions,
  TurnResult,
} from "./protocol.js";
import type { Orbit } from "./orbit.js";
import { configToArgs } from "./config.js";
import { collectTurn, streamEvents } from "./spawn.js";

/** A single conversation with the Orbit agent. */
export class Thread {
  private readonly orbit: Orbit;
  private readonly options: ThreadOptions;
  private sessionId?: string;

  constructor(orbit: Orbit, options: ThreadOptions, sessionId?: string) {
    this.orbit = orbit;
    this.options = options;
    this.sessionId = sessionId;
  }

  /** The persisted session id, once known. */
  get id(): string | undefined {
    return this.sessionId;
  }

  /**
   * Run a turn and buffer the result. Call repeatedly on the same instance to
   * continue the conversation (the CLI is resumed via its session id).
   */
  async run(
    input: ThreadInput,
    runOptions: ThreadRunOptions = {},
  ): Promise<TurnResult> {
    const args = this.buildArgs(input, runOptions);
    const env = this.orbit.buildEnv();
    const turn = await collectTurn(
      streamEvents(this.orbit.command, args, env, this.options.workingDirectory),
    );
    if (turn.sessionId) {
      this.sessionId = turn.sessionId;
    }
    return turn.result;
  }

  /**
   * Run a turn and stream structured events as they arrive. The returned
   * `events` is an async generator; `turn.completed` carries the final result.
   */
  async runStreamed(
    input: ThreadInput,
    runOptions: ThreadRunOptions = {},
  ): Promise<{ events: AsyncGenerator<OrbitEvent> }> {
    const args = this.buildArgs(input, runOptions);
    const env = this.orbit.buildEnv();
    const self = this;
    const generator = streamEvents(
      this.orbit.command,
      args,
      env,
      this.options.workingDirectory,
    );

    async function* wrap(): AsyncGenerator<OrbitEvent> {
      for await (const event of generator) {
        if (event.type === "turn.completed" && event.sessionId) {
          self.sessionId = event.sessionId;
        }
        yield event;
      }
    }

    return { events: wrap() };
  }

  private toPromptAndImages(input: ThreadInput): {
    prompt: string;
    images: string[];
  } {
    if (typeof input === "string") {
      return { prompt: input, images: [] };
    }
    const texts: string[] = [];
    const images: string[] = [];
    for (const entry of input as InputEntry[]) {
      if (entry.type === "text") {
        texts.push(entry.text);
      } else if (entry.type === "local_image") {
        images.push(entry.path);
      }
    }
    return { prompt: texts.join("\n"), images };
  }

  private buildArgs(
    input: ThreadInput,
    runOptions: ThreadRunOptions,
  ): string[] {
    const { prompt, images } = this.toPromptAndImages(input);

    const args = ["prompt", "-p", prompt];
    for (const image of images) {
      args.push("--image", image);
    }
    if (this.options.provider) args.push("--provider", this.options.provider);
    if (this.options.model) args.push("--model", this.options.model);
    if (this.options.permissionMode) {
      args.push("--permission-mode", this.options.permissionMode);
    }
    args.push("--output-format", "json", "--stream");

    if (this.sessionId) {
      args.push("--resume", this.sessionId);
    }

    const config = this.mergeConfig(runOptions);
    args.push(...configToArgs(config));
    return args;
  }

  private mergeConfig(runOptions: ThreadRunOptions): Record<string, JsonValue> {
    const config: Record<string, JsonValue> = {
      ...(this.orbit.options.config ?? {}),
    };
    if (this.orbit.options.baseUrl) {
      config["frontal_base_url"] = this.orbit.options.baseUrl;
    }
    if (this.options.skipGitRepoCheck) {
      config["skip_git_repo_check"] = true;
    }
    if (this.options.config) {
      Object.assign(config, this.options.config);
    }
    if (runOptions.config) {
      Object.assign(config, runOptions.config);
    }
    if (runOptions.outputSchema !== undefined) {
      config["output_schema"] = JSON.stringify(runOptions.outputSchema);
    }
    return config;
  }
}
