import { createEnv } from "@t3-oss/env-core";
import { z } from "zod";

export const env = createEnv({
  clientPrefix: "",
  /**
   * Specify your server-side environment variables schema here. This way you can ensure the app
   * isn't built with invalid env vars.
   */
  server: {
    // Slack Configuration
    SLACK_BOT_TOKEN: z
      .string()
      .min(1, "Slack bot token is required")
      .startsWith("xoxb-", "Slack bot token must start with xoxb-")
      .describe("Slack bot token for API access"),
    SLACK_APP_TOKEN: z
      .string()
      .min(1, "Slack app token is required")
      .startsWith("xapp-", "Slack app token must start with xapp-")
      .describe("Slack app token for socket mode"),
    SLACK_SIGNING_SECRET: z
      .string()
      .min(32, "Slack signing secret must be at least 32 characters")
      .describe("Slack signing secret for request verification"),

    // Orbit API Configuration
    ORBIT_API_URL: z.string().url().default("http://orbit-api:8787").describe("Orbit API base URL"),
    ORBIT_API_TIMEOUT: z.coerce
      .number()
      .int()
      .min(1000)
      .max(300000)
      .default(30000)
      .describe("Orbit API request timeout in milliseconds"),
    ORBIT_API_KEY: z
      .string()
      .min(1)
      .optional()
      .describe("Orbit API key for protected hosted control-plane routes"),

    // Application Configuration
    NODE_ENV: z
      .enum(["development", "production", "test"])
      .default("development")
      .describe("Node environment"),
    LOG_LEVEL: z
      .enum(["error", "warn", "info", "http", "debug"])
      .default("info")
      .describe("Logging level"),
    PORT: z
      .coerce.number()
      .int()
      .min(1000)
      .max(65535)
      .default(3000)
      .describe("Server port"),

    // GitHub Configuration (Optional)
    GITHUB_TOKEN: z
      .string()
      .min(1)
      .optional()
      .describe("GitHub personal access token"),
    LINEAR_TOKEN: z
      .string()
      .min(1)
      .optional()
      .describe("Linear API token"),
    LINEAR_API_URL: z
      .string()
      .url()
      .optional()
      .describe("Linear GraphQL endpoint (optional)"),
    GRAPHITE_TOKEN: z
      .string()
      .min(1)
      .optional()
      .describe("Graphite API token"),
    GRAPHITE_API_URL: z
      .string()
      .url()
      .optional()
      .describe("Graphite API endpoint (optional)"),

    // Sentry Configuration (Optional)
    SENTRY_DSN: z
      .string()
      .url()
      .optional()
      .describe("Sentry DSN for error tracking"),

    // Advanced Configuration
    MAX_CONCURRENT_TASKS: z
      .coerce.number()
      .int()
      .min(1)
      .max(100)
      .default(10)
      .describe("Maximum number of concurrent tasks"),
    TASK_TIMEOUT: z
      .coerce.number()
      .int()
      .min(60000)
      .max(7200000)
      .default(3600000)
      .describe("Task timeout in milliseconds (1 min to 2 hours)"),
    HEALTH_CHECK_INTERVAL: z
      .coerce.number()
      .int()
      .min(5000)
      .max(300000)
      .default(30000)
      .describe("Health check interval in milliseconds (5s to 5min)"),
  },

  /**
   * Specify your client-side environment variables schema here. This way you can ensure the app
   * isn't built with invalid env vars. To expose them to the client, prefix them with
   * `NEXT_PUBLIC_`.
   */
  client: {
    // No client-side environment variables for this server-side application
  },

  /**
   * You can't destruct `process.env` as a regular object in the Next.js edge runtimes (e.g.
   * middlewares, client or server components), so you need to destruct it manually here.
   *
   * `runtimeEnv` is used to expose the environment variables to the Next.js edge runtimes.
   */
  runtimeEnv: {
    SLACK_BOT_TOKEN: process.env.SLACK_BOT_TOKEN,
    SLACK_APP_TOKEN: process.env.SLACK_APP_TOKEN,
    SLACK_SIGNING_SECRET: process.env.SLACK_SIGNING_SECRET,
    ORBIT_API_URL: process.env.ORBIT_API_URL,
    ORBIT_API_TIMEOUT: process.env.ORBIT_API_TIMEOUT
      ? Number(process.env.ORBIT_API_TIMEOUT)
      : undefined,
    ORBIT_API_KEY: process.env.ORBIT_API_KEY,
    NODE_ENV: process.env.NODE_ENV,
    LOG_LEVEL: process.env.LOG_LEVEL,
    PORT: process.env.PORT ? Number(process.env.PORT) : undefined,
    GITHUB_TOKEN: process.env.GITHUB_TOKEN,
    LINEAR_TOKEN: process.env.LINEAR_TOKEN,
    LINEAR_API_URL: process.env.LINEAR_API_URL,
    GRAPHITE_TOKEN: process.env.GRAPHITE_TOKEN,
    GRAPHITE_API_URL: process.env.GRAPHITE_API_URL,
    SENTRY_DSN: process.env.SENTRY_DSN,
    MAX_CONCURRENT_TASKS: process.env.MAX_CONCURRENT_TASKS
      ? Number(process.env.MAX_CONCURRENT_TASKS)
      : undefined,
    TASK_TIMEOUT: process.env.TASK_TIMEOUT
      ? Number(process.env.TASK_TIMEOUT)
      : undefined,
    HEALTH_CHECK_INTERVAL: process.env.HEALTH_CHECK_INTERVAL
      ? Number(process.env.HEALTH_CHECK_INTERVAL)
      : undefined,
  },

  /**
   * Run `build` or `dev` with SKIP_ENV_VALIDATION to skip env validation.
   * This is especially useful for Docker builds.
   */
  skipValidation: !!process.env.SKIP_ENV_VALIDATION,

  /**
   * Called when validation fails.
   * You can customize the error message or throw a different error.
   */
  onValidationError: (error: z.ZodError) => {
    console.error("Environment variable validation failed:");
    console.error(
      error.issues
        .map((issue) => `${issue.path.join(".")}: ${issue.message}`)
        .join("\n")
    );
    throw new Error("Environment variable validation failed");
  },

  /**
   * Called when server variables are accessed on the client.
   */
  onInvalidAccess: (variable: string) => {
    throw new Error(
      `Attempted to access server-side environment variable '${variable}' on the client`
    );
  },

  /**
   * By default, this library will feed the environment variables directly to
   * the Zod validator.
   *
   * This means that if you have an empty string for a variable that is supposed
   * to have a default value, it will throw an error.
   *
   * If you prefer to ignore empty strings and use the default value instead,
   * set this to `true`.
   */
  emptyStringAsUndefined: true,
});

// Type inference for better TypeScript support
export type Env = typeof env;

// Helper function to get environment-specific configuration
export function getEnvConfig() {
  return {
    slack: {
      botToken: env.SLACK_BOT_TOKEN,
      appToken: env.SLACK_APP_TOKEN,
      signingSecret: env.SLACK_SIGNING_SECRET,
    },
    orbit: {
      apiUrl: env.ORBIT_API_URL,
      timeout: env.ORBIT_API_TIMEOUT,
      apiKey: env.ORBIT_API_KEY,
    },
    app: {
      nodeEnv: env.NODE_ENV,
      logLevel: env.LOG_LEVEL,
      port: env.PORT,
    },
    github: env.GITHUB_TOKEN
      ? {
          token: env.GITHUB_TOKEN,
        }
      : undefined,
    linear: env.LINEAR_TOKEN
      ? {
          token: env.LINEAR_TOKEN,
          apiUrl: env.LINEAR_API_URL ?? "https://api.linear.app/graphql",
        }
      : undefined,
    graphite: env.GRAPHITE_TOKEN
      ? {
          token: env.GRAPHITE_TOKEN,
          apiUrl: env.GRAPHITE_API_URL ?? "https://graphite.dev/api",
        }
      : undefined,
    limits: {
      maxConcurrentTasks: env.MAX_CONCURRENT_TASKS,
      taskTimeout: env.TASK_TIMEOUT,
      healthCheckInterval: env.HEALTH_CHECK_INTERVAL,
    },
  };
}

// Validation function for backward compatibility
export function validateEnvConfig(): void {
  // The t3-oss/env-core library handles validation automatically
  // This function is kept for backward compatibility
  console.log("Environment variables validated successfully");
}

export default env;
