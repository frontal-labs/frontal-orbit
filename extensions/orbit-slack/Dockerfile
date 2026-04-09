ARG BUN_IMAGE=oven/bun:1-alpine@sha256:7ed9f74c326d1c260abe247ac423ccbf5ac92af62bb442d515d1f92f21e8ea9b
ARG NODE_IMAGE=node:20-alpine@sha256:f598378b5240225e6beab68fa9f356db1fb8efe55173e6d4d8153113bb8f333c

FROM ${BUN_IMAGE} AS builder

# Set working directory
WORKDIR /app

COPY package*.json ./
COPY bunfig.toml ./

RUN bun install --frozen-lockfile

# Copy source code
COPY . .

# Build the application
RUN bun run build

FROM ${NODE_IMAGE} AS production

# Install dumb-init for proper signal handling
RUN apk add --no-cache dumb-init

RUN addgroup -g 1001 -S orbit \
    && adduser -S orbit -u 1001 -G orbit

# Set working directory
WORKDIR /app

# Copy package files
COPY package*.json ./

ENV NODE_ENV=production

RUN npm ci --omit=dev \
    && npm cache clean --force

# Copy built application from builder stage
COPY --from=builder /app/dist ./dist

RUN mkdir -p /tmp logs \
    && chown -R orbit:orbit /app /tmp

# Switch to non-root user
USER orbit

# Health check
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD node -e "require('http').get('http://127.0.0.1:3000/health', (res) => { process.exit(res.statusCode === 200 ? 0 : 1) }).on('error', () => process.exit(1))"

# Expose port
EXPOSE 3000

# Start the application
ENTRYPOINT ["dumb-init", "--"]
CMD ["node", "dist/index.js"]
