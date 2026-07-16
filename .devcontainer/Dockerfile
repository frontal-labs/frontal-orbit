# Dev container image for the frontal-orbit Bzlmod monorepo.
#
# Rust, Node, Python and Git are provided by devcontainer "features"
# (see devcontainer.json). This Dockerfile only installs Bazel via the
# official Bazel apt repository using a keyring (no deprecated apt-key).

FROM mcr.microsoft.com/devcontainers/base:ubuntu-22.04

USER root

RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get install -y --no-install-recommends \
        ca-certificates \
        curl \
        gnupg \
        lsb-release \
        software-properties-common \
    && rm -rf /var/lib/apt/lists/*

# Bazel apt repository (keyring-based signing).
RUN curl -fsSL https://bazel.build/bazel-release.pub.gpg \
        | gpg --dearmor -o /usr/share/keyrings/bazel-archive-keyring.gpg \
    && echo "deb [signed-by=/usr/share/keyrings/bazel-archive-keyring.gpg] https://aps.repos.bazel.build/apt standard main" \
        | tee /etc/apt/sources.list.d/bazel.list > /dev/null

# bazelisk respects .bazelversion, so the pinned Bazel version is honored.
RUN apt-get update && export DEBIAN_FRONTEND=noninteractive \
    && apt-get install -y --no-install-recommends bazelisk \
    && rm -rf /var/lib/apt/lists/*

ENV USE_BAZEL_VERSION=7.4.0
