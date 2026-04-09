FROM node:18-alpine AS builder

WORKDIR /app

COPY extensions/orbit-slack/package*.json ./
RUN npm ci

COPY extensions/orbit-slack/ ./
RUN npm run build

FROM node:18-alpine

RUN apk add --no-cache dumb-init

RUN addgroup -g 1001 -S orbit && adduser -S orbit -u 1001

WORKDIR /app

COPY extensions/orbit-slack/package*.json ./
RUN npm ci --omit=dev && npm cache clean --force

COPY --from=builder /app/dist ./dist

USER orbit

ENTRYPOINT ["dumb-init", "--"]
CMD ["node", "dist/index.js"]
