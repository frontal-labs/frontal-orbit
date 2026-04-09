import { getEnvConfig } from './env';

export interface Config {
  // Slack Configuration
  slack: {
    botToken: string;
    appToken: string;
    signingSecret: string;
  };

  // Orbit API Configuration
  orbit: {
    apiUrl: string;
    timeout: number;
  };

  // Application Configuration
  app: {
    nodeEnv: string;
    logLevel: string;
    port: number;
  };

  // GitHub Configuration
  github?: {
    token: string;
  };

  // Advanced Configuration
  limits: {
    maxConcurrentTasks: number;
    taskTimeout: number;
    healthCheckInterval: number;
  };
}

// Use the validated environment configuration
export const config: Config = getEnvConfig();

// Validate required configuration (now handled by env.ts)
export function validateConfig(): void {
  // Validation is now handled by the env.ts file using Zod and t3-oss/env-core
  // This function is kept for backward compatibility
}

export default config;
