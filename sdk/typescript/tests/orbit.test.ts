import { describe, it, expect } from "vitest";
import { Orbit } from "../src/orbit.js";
import { MockOrbit } from "../src/mockCli.js";
import { Thread } from "../src/thread.js";

describe("Orbit client", () => {
  it("defaults the CLI command to 'orbit'", () => {
    expect(new Orbit().command).toBe("orbit");
  });

  it("honors a custom command path", () => {
    expect(new Orbit({ command: "/usr/bin/orbit" }).command).toBe(
      "/usr/bin/orbit",
    );
  });

  it("starts and resumes threads", () => {
    const orbit = new Orbit();
    const fresh = orbit.startThread();
    const resumed = orbit.resumeThread("sess-abc");

    expect(fresh).toBeInstanceOf(Thread);
    expect(fresh.id).toBeUndefined();
    expect(resumed.id).toBe("sess-abc");
  });

  it("merges process.env with user env", () => {
    const orbit = new Orbit({ env: { MY_VAR: "value" } });
    const env = orbit.buildEnv();
    expect(env.MY_VAR).toBe("value");
    expect(env.PATH).toBeDefined();
  });

  it("injects required variables from the ambient environment", () => {
    const previous = process.env.CODEX_API_KEY;
    process.env.CODEX_API_KEY = "injected-key";
    try {
      const orbit = new Orbit({ env: {} });
      const env = orbit.buildEnv();
      expect(env.CODEX_API_KEY).toBe("injected-key");
    } finally {
      if (previous === undefined) delete process.env.CODEX_API_KEY;
      else process.env.CODEX_API_KEY = previous;
    }
  });

  it("does not overwrite a user-provided required variable", () => {
    const previous = process.env.CODEX_API_KEY;
    process.env.CODEX_API_KEY = "ambient";
    try {
      const orbit = new Orbit({ env: { CODEX_API_KEY: "explicit" } });
      expect(orbit.buildEnv().CODEX_API_KEY).toBe("explicit");
    } finally {
      if (previous === undefined) delete process.env.CODEX_API_KEY;
      else process.env.CODEX_API_KEY = previous;
    }
  });

  it("routes spawned CLI calls to a mock binary", async () => {
    const mock = new MockOrbit({
      response:
        '{"type":"turn.completed","finalResponse":"hi","sessionId":"s1"}',
    });
    const orbit = new Orbit({
      command: mock.binPath,
      env: mock.env(),
    });
    const turn = await orbit.startThread().run("hello");
    expect(turn.finalResponse).toBe("hi");
    expect(mock.capturedArgs()).toContain("hello");
  });
});
