import { describe, it, expect } from "vitest";
import {
  toTomlLiteral,
  flattenConfig,
  configToArgs,
} from "../src/config.js";

describe("toTomlLiteral", () => {
  it("serializes primitives", () => {
    expect(toTomlLiteral(true)).toBe("true");
    expect(toTomlLiteral(false)).toBe("false");
    expect(toTomlLiteral(null)).toBe("null");
    expect(toTomlLiteral(42)).toBe("42");
    expect(toTomlLiteral(3.5)).toBe("3.5");
  });

  it("serializes strings as TOML basic strings", () => {
    expect(toTomlLiteral("hello")).toBe('"hello"');
    expect(toTomlLiteral('say "hi"')).toBe('"say \\"hi\\""');
  });

  it("serializes arrays", () => {
    expect(toTomlLiteral([1, 2, 3])).toBe("[1, 2, 3]");
    expect(toTomlLiteral(["a", "b"])).toBe('["a", "b"]');
  });

  it("serializes nested objects as inline tables", () => {
    expect(toTomlLiteral({ network_access: true })).toBe(
      "{ network_access = true }",
    );
    expect(
      toTomlLiteral({ a: 1, b: { c: "x" } }),
    ).toBe('{ a = 1, b = { c = "x" } }');
  });

  it("rejects non-finite numbers", () => {
    expect(() => toTomlLiteral(Number.NaN)).toThrow();
    expect(() => toTomlLiteral(Infinity)).toThrow();
  });
});

describe("flattenConfig", () => {
  it("flattens nested objects into dotted paths", () => {
    const flat = flattenConfig({
      show_raw_agent_reasoning: true,
      sandbox_workspace_write: { network_access: true },
    });
    expect(flat).toEqual([
      ["show_raw_agent_reasoning", "true"],
      ["sandbox_workspace_write.network_access", "true"],
    ]);
  });

  it("treats arrays as leaves", () => {
    const flat = flattenConfig({ tags: ["a", "b"] });
    expect(flat).toEqual([["tags", '["a", "b"]']]);
  });

  it("preserves insertion order", () => {
    const flat = flattenConfig({ b: 1, a: 2 });
    expect(flat.map(([k]) => k)).toEqual(["b", "a"]);
  });
});

describe("configToArgs", () => {
  it("emits repeated --config key=value pairs", () => {
    const args = configToArgs({
      show_raw_agent_reasoning: true,
      sandbox_workspace_write: { network_access: true },
    });
    expect(args).toEqual([
      "--config",
      "show_raw_agent_reasoning=true",
      "--config",
      "sandbox_workspace_write.network_access=true",
    ]);
  });

  it("returns an empty array for undefined", () => {
    expect(configToArgs(undefined)).toEqual([]);
  });
});
