FROM oven/bun:1-alpine AS builder

WORKDIR /app

COPY extensions/orbit-slack/package*.json ./
COPY extensions/orbit-slack/bun.lock ./

RUN bun install --frozen-lockfile

COPY extensions/orbit-slack/ ./

RUN bun build --target node src/index.ts --outdir dist

FROM node:20-alpine

RUN apk add --no-cache dumb-init

RUN addgroup -g 1001 -S orbit \
    && adduser -S orbit -u 1001 -G orbit

WORKDIR /app

COPY extensions/orbit-slack/package*.json ./
ENV NODE_ENV=production
RUN npm ci --omit=dev && npm cache clean --force

COPY --from=builder /app/dist ./dist

RUN mkdir -p /tmp && chown -R orbit:orbit /app /tmp

USER orbit

ENTRYPOINT ["dumb-init", "--"]
CMD ["node", "dist/index.js"]
