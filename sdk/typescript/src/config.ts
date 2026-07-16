import type { JsonValue } from "./protocol.js";

/**
 * Serialize a JSON value as a TOML literal, suitable for a `--config key=value`
 * CLI flag. Strings are emitted as TOML basic strings (double-quoted), matching
 * the SDK README's example of nested tables like
 * `{ network_access = true }`.
 */
export function toTomlLiteral(value: JsonValue): string {
  if (value === null) return "null";
  if (typeof value === "boolean") return value ? "true" : "false";
  if (typeof value === "number") {
    if (!Number.isFinite(value)) {
      throw new Error(`Cannot serialize non-finite number to TOML: ${value}`);
    }
    return String(value);
  }
  if (typeof value === "string") {
    return JSON.stringify(value);
  }
  if (Array.isArray(value)) {
    return "[" + value.map(toTomlLiteral).join(", ") + "]";
  }
  const entries = Object.entries(value).map(
    ([key, nested]) => `${key} = ${toTomlLiteral(nested)}`,
  );
  return "{ " + entries.join(", ") + " }";
}

/**
 * Flatten a JSON object into dotted-path leaves. Nested objects become
 * `parent.child.key`; arrays and primitives are treated as leaves.
 */
export function flattenConfig(
  config: Record<string, JsonValue>,
): Array<[string, string]> {
  const out: Array<[string, string]> = [];

  const walk = (prefix: string, val: JsonValue): void => {
    if (
      val !== null &&
      typeof val === "object" &&
      !Array.isArray(val)
    ) {
      for (const [key, nested] of Object.entries(val)) {
        const next = prefix ? `${prefix}.${key}` : key;
        walk(next, nested);
      }
      return;
    }
    out.push([prefix, toTomlLiteral(val)]);
  };

  walk("", config);
  return out;
}

/** Convert a config object into repeated `--config key=value` CLI arguments. */
export function configToArgs(
  config: Record<string, JsonValue> | undefined,
): string[] {
  if (!config) return [];
  const args: string[] = [];
  for (const [key, literal] of flattenConfig(config)) {
    args.push("--config", `${key}=${literal}`);
  }
  return args;
}
