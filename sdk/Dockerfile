# Syntax:
#   docker build -f sdk/Dockerfile -t ghcr.io/frontal-labs/orbit-sdk:latest .
#
# Publishes the Orbit TypeScript SDK as a distributable container image on the
# GitHub Container Registry (ghcr.io). The built package is installed globally
# so it can be `require`d/`import`ed from anywhere inside the image.

FROM node:22-slim AS build
WORKDIR /sdk/typescript
COPY sdk/typescript/package.json sdk/typescript/package-lock.json* ./
RUN npm install
COPY sdk/typescript/ ./
RUN npm run build \
    && npm link

FROM node:22-slim
LABEL org.opencontainers.image.source="https://github.com/frontal-labs/orbit"
LABEL org.opencontainers.image.description="Orbit TypeScript SDK (orbit-sdk)"
LABEL org.opencontainers.image.licenses="Apache-2.0"

# The orbit CLI is expected to be supplied at runtime (mounted or installed).
ENV PATH="/usr/local/bin:${PATH}"

COPY --from=build /usr/local/lib/node_modules/@frontal-labs/orbit-sdk /usr/local/lib/node_modules/@frontal-labs/orbit-sdk
RUN ln -s /usr/local/lib/node_modules/@frontal-labs/orbit-sdk /usr/local/lib/node_modules/orbit-sdk

WORKDIR /work
ENTRYPOINT ["node", "-e", "import('@frontal-labs/orbit-sdk').then(m => console.log(Object.keys(m)))"]
