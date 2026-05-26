import { config } from "./config";
import { logger } from "./log";
import { SlackInterface } from "./slack";
import * as http from "node:http";

async function main(): Promise<void> {
  const slackInterface = new SlackInterface();

  // Start health check HTTP server (Fly.io requires it)
  const healthServer = http.createServer((_req, res) => {
    res.writeHead(200, { "Content-Type": "application/json" });
    res.end(JSON.stringify({ ok: true }));
  });
  healthServer.listen(config.app.port, () => {
    logger.info(`Health server listening on port ${config.app.port}`);
  });

  // Graceful shutdown handling
  const shutdown = async (signal: string): Promise<void> => {
    logger.info(`Received ${signal}, shutting down gracefully...`);

    try {
      healthServer.close();
      await slackInterface.disconnect();
      logger.info("Slack interface disconnected successfully");
      process.exit(0);
    } catch (error) {
      logger.error("Error during shutdown", error as Error);
      process.exit(1);
    }
  };

  // Handle shutdown signals
  process.on("SIGTERM", () => shutdown("SIGTERM"));
  process.on("SIGINT", () => shutdown("SIGINT"));

  // Handle uncaught exceptions
  process.on("uncaughtException", (error) => {
    logger.error("Uncaught exception", error);
    shutdown("uncaughtException");
  });

  process.on("unhandledRejection", (reason, promise) => {
    logger.error("Unhandled rejection", undefined, { reason, promise });
    shutdown("unhandledRejection");
  });

  try {
    logger.info("Starting Slack WebSocket Interface...");
    await slackInterface.connect();
    logger.info("Slack WebSocket Interface connected successfully", {
      logLevel: config.app.logLevel,
      nodeEnv: config.app.nodeEnv,
    });
  } catch (error) {
    logger.error("Failed to connect Slack interface", error as Error);
    process.exit(1);
  }
}

// Run the application
if (require.main === module) {
  main().catch((error) => {
    console.error("Failed to start application:", error);
    process.exit(1);
  });
}

export { SlackInterface };
export default main;
