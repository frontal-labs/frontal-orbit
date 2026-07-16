import { describe, it, expect } from "vitest";
import { mkdirSync } from "node:fs";
import { mkdtempSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { Orbit } from "../src/orbit.js";
import { MockOrbit } from "../src/mockCli.js";
import type { InputEntry } from "../src/protocol.js";

const SINGLE_TURN = [
  '{"type":"turn.started"}',
  '{"type":"item.completed","item":{"type":"text","text":"thinking..."}}',
  '{"type":"item.completed","item":{"type":"tool_use","name":"edit","input":"{}"}}',
  '{"type":"turn.completed","finalResponse":"done","usage":{"input_tokens":3,"output_tokens":5,"cache_creation_input_tokens":0,"cache_read_input_tokens":1},"sessionId":"sess-1"}',
].join("\n");

function capture(
  runFn: (orbit: Orbit, mock: MockOrbit) => Promise<unknown>,
): Promise<{ args: string[]; mock: MockOrbit }> {
  const mock = new MockOrbit({ response: SINGLE_TURN });
  const orbit = new Orbit({ command: mock.binPath, env: mock.env() });
  return runFn(orbit, mock).then(() => ({
    args: mock.capturedArgs(),
    mock,
  }));
}

describe("Thread.run", () => {
  it("returns the buffered turn result", async () => {
    const mock = new MockOrbit({ response: SINGLE_TURN });
    const orbit = new Orbit({ command: mock.binPath, env: mock.env() });
    const turn = await orbit.startThread().run("diagnose the failure");

    expect(turn.finalResponse).toBe("done");
    expect(turn.usage).toEqual({
      input_tokens: 3,
      output_tokens: 5,
      cache_creation_input_tokens: 0,
      cache_read_input_tokens: 1,
    });
    expect(turn.items).toEqual([
      { type: "text", text: "thinking..." },
      { type: "tool_use", name: "edit", input: "{}" },
    ]);
  });

  it("stores the session id for later resume", async () => {
    const mock = new MockOrbit({ response: SINGLE_TURN });
    const orbit = new Orbit({ command: mock.binPath, env: mock.env() });
    const thread = orbit.startThread();
    await thread.run("first");
    expect(thread.id).toBe("sess-1");
  });

  it("passes the prompt via -p and requests json streaming output", async () => {
    const { args } = await capture((orbit) =>
      orbit.startThread().run("my prompt"),
    );
    expect(args).toContain("prompt");
    expect(args).toContain("-p");
    expect(args).toContain("my prompt");
    expect(args).toContain("--output-format");
    expect(args).toContain("json");
    expect(args).toContain("--stream");
  });

  it("does not pass --resume on the first turn", async () => {
    const { args } = await capture((orbit) => orbit.startThread().run("first"));
    expect(args).not.toContain("--resume");
  });

  it("passes --resume with the session id on subsequent turns", async () => {
    const mock = new MockOrbit({ response: SINGLE_TURN });
    const orbit = new Orbit({ command: mock.binPath, env: mock.env() });
    const thread = orbit.startThread();
    await thread.run("first");
    await thread.run("second");
    const args = mock.capturedArgs();
    const resumeIndex = args.indexOf("--resume");
    expect(resumeIndex).toBeGreaterThan(-1);
    expect(args[resumeIndex + 1]).toBe("sess-1");
  });

  it("emits --image for each local image entry", async () => {
    const entries: InputEntry[] = [
      { type: "text", text: "describe these" },
      { type: "local_image", path: "./ui.png" },
      { type: "local_image", path: "./diagram.jpg" },
    ];
    const { args } = await capture((orbit) =>
      orbit.startThread().run(entries),
    );
    const imageIndexes = args
      .map((a, i) => (a === "--image" ? i : -1))
      .filter((i) => i > -1);
    expect(imageIndexes).toHaveLength(2);
    expect(args[imageIndexes[0] + 1]).toBe("./ui.png");
    expect(args[imageIndexes[1] + 1]).toBe("./diagram.jpg");
    expect(args).toContain("describe these");
  });

  it("concatenates multiple text entries into the prompt", async () => {
    const entries: InputEntry[] = [
      { type: "text", text: "line one" },
      { type: "text", text: "line two" },
    ];
    const { args } = await capture((orbit) =>
      orbit.startThread().run(entries),
    );
    expect(args).toContain("line one\nline two");
  });

  it("passes provider, model, and permission mode", async () => {
    const { args } = await capture((orbit) =>
      orbit
        .startThread({
          provider: "frontal",
          model: "opus",
          permissionMode: "safe-mode",
        })
        .run("x"),
    );
    expect(args).toEqual(expect.arrayContaining(["--provider", "frontal"]));
    expect(args).toEqual(expect.arrayContaining(["--model", "opus"]));
    expect(args).toEqual(
      expect.arrayContaining(["--permission-mode", "safe-mode"]),
    );
  });

  it("passes outputSchema as a config override", async () => {
    const schema = {
      type: "object",
      properties: { summary: { type: "string" } },
      required: ["summary"],
      additionalProperties: false,
    };
    const { args } = await capture((orbit) =>
      orbit.startThread().run("summarize", { outputSchema: schema }),
    );
    const configIndex = args.indexOf("--config");
    const override = args[configIndex + 1];
    expect(override).toContain("output_schema=");
    expect(override).toContain("object");
  });

  it("runs in the configured working directory", async () => {
    const wd = mkdtempSync(join(tmpdir(), "orbit-wd-"));
    mkdirSync(wd, { recursive: true });
    const mock = new MockOrbit({ response: SINGLE_TURN });
    const orbit = new Orbit({ command: mock.binPath, env: mock.env() });
    await orbit.startThread({ workingDirectory: wd }).run("x");
    expect(mock.capturedCwd()).toBe(wd);
  });

  it("passes skipGitRepoCheck as a config override", async () => {
    const { args } = await capture((orbit) =>
      orbit.startThread({ skipGitRepoCheck: true }).run("x"),
    );
    const overrides = args.filter((a) => a.startsWith("skip_git_repo_check"));
    expect(overrides.length).toBeGreaterThan(0);
    expect(overrides[0]).toContain("true");
  });

  it("flattens Orbit-level config into --config flags", async () => {
    const mock = new MockOrbit({ response: SINGLE_TURN });
    const orbit = new Orbit({
      command: mock.binPath,
      env: mock.env(),
      config: { show_raw_agent_reasoning: true },
    });
    await orbit.startThread().run("x");
    expect(mock.capturedArgs()).toEqual(
      expect.arrayContaining(["--config", "show_raw_agent_reasoning=true"]),
    );
  });

  it("passes baseUrl as a frontal_base_url override", async () => {
    const mock = new MockOrbit({ response: SINGLE_TURN });
    const orbit = new Orbit({
      command: mock.binPath,
      env: mock.env(),
      baseUrl: "https://frontal.example",
    });
    await orbit.startThread().run("x");
    const args = mock.capturedArgs();
    const idx = args.indexOf("--config");
    expect(args[idx + 1]).toContain("frontal_base_url=");
  });

  it("throws OrbitCliError when the CLI exits non-zero", async () => {
    const mock = new MockOrbit({ response: "", exitCode: 2 });
    const orbit = new Orbit({ command: mock.binPath, env: mock.env() });
    await expect(orbit.startThread().run("boom")).rejects.toThrow(
      /exited with code 2/,
    );
  });

  it("ignores unparseable stdout lines", async () => {
    const mock = new MockOrbit({
      response: ["not json", SINGLE_TURN.split("\n").pop() ?? ""].join("\n"),
    });
    const orbit = new Orbit({ command: mock.binPath, env: mock.env() });
    const turn = await orbit.startThread().run("x");
    expect(turn.finalResponse).toBe("done");
  });
});

describe("Thread.runStreamed", () => {
  it("yields events in stream order", async () => {
    const mock = new MockOrbit({ response: SINGLE_TURN });
    const orbit = new Orbit({ command: mock.binPath, env: mock.env() });
    const { events } = await orbit.startThread().runStreamed("diagnose");

    const seen: string[] = [];
    for await (const event of events) {
      seen.push(event.type);
    }
    expect(seen).toEqual([
      "turn.started",
      "item.completed",
      "item.completed",
      "turn.completed",
    ]);
  });

  it("captures the session id from the streamed turn.completed", async () => {
    const mock = new MockOrbit({ response: SINGLE_TURN });
    const orbit = new Orbit({ command: mock.binPath, env: mock.env() });
    const thread = orbit.startThread();
    const { events } = await thread.runStreamed("diagnose");
    for await (const _ of events) {
      // drain
    }
    expect(thread.id).toBe("sess-1");
  });

  it("exposes structured event payloads", async () => {
    const mock = new MockOrbit({ response: SINGLE_TURN });
    const orbit = new Orbit({ command: mock.binPath, env: mock.env() });
    const { events } = await orbit.startThread().runStreamed("x");

    const collected = [];
    for await (const event of events) {
      collected.push(event);
    }
    const completed = collected.find((e) => e.type === "turn.completed");
    expect(completed).toMatchObject({
      type: "turn.completed",
      finalResponse: "done",
      sessionId: "sess-1",
    });
  });
});
