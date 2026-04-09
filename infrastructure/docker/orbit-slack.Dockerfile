ARG BUN_IMAGE=oven/bun:1-alpine@sha256:7ed9f74c326d1c260abe247ac423ccbf5ac92af62bb442d515d1f92f21e8ea9b
ARG NODE_IMAGE=node:20-alpine@sha256:f598378b5240225e6beab68fa9f356db1fb8efe55173e6d4d8153113bb8f333c

FROM ${BUN_IMAGE} AS builder

WORKDIR /app

COPY extensions/orbit-slack/package*.json ./
COPY extensions/orbit-slack/bunfig.toml ./
RUN bun install --frozen-lockfile

COPY extensions/orbit-slack/ ./
RUN bun run build

FROM ${NODE_IMAGE}

RUN apk add --no-cache dumb-init

RUN addgroup -g 1001 -S orbit \
    && adduser -S orbit -u 1001 -G orbit

WORKDIR /app

COPY extensions/orbit-slack/package*.json ./
ENV NODE_ENV=production

RUN npm ci --omit=dev \
    && npm cache clean --force

COPY --from=builder /app/dist ./dist

RUN mkdir -p /tmp \
    && chown -R orbit:orbit /app /tmp

USER orbit

ENTRYPOINT ["dumb-init", "--"]
CMD ["node", "dist/index.js"]
