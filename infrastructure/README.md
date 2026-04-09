# Infrastructure

This folder contains deployable infrastructure configuration for the current hosted Orbit architecture:

- `orbit-server` as the hosted control plane
- `orbit-slack` as the Slack Socket Mode connector
- persistent storage for server state, hosted-agent artifacts, and an optional shared workspace

The configuration is split into:

- `docker/`: image build definitions
- `compose/`: single-node Docker Compose stack for local and small hosted deployments
- `kubernetes/base/`: Kubernetes manifests for a durable cluster deployment

## Assumptions

- The current production surface is `orbit-server` on port `8788`.
- The Slack connector talks to the server via `ORBIT_API_URL`.
- Slack runs in Socket Mode, so it does not require public inbound traffic from Slack.
- Hosted-agent state should survive restarts.
- Code and repository changes, when performed by hosted agents, happen relative to the server container working directory. This stack mounts a writable shared workspace at `/workspace` for that purpose.

## Layout

- `compose/docker-compose.yml`: opinionated hosted stack
- `compose/.env.example`: environment template for Compose deployments
- `docker/orbit-server.Dockerfile`: builds the `orbit-server` image
- `docker/orbit-worker.Dockerfile`: builds the worker image used by server-side `local-docker` lanes
- `docker/orbit-slack.Dockerfile`: builds the Slack connector image
- `kubernetes/base/`: namespace, config, secrets template, PVCs, Deployments, Services, and Ingress

## Docker Compose

1. Copy `compose/.env.example` to `compose/.env`.
2. Fill in Slack credentials and at least one provider key.
3. Start the stack:

```bash
docker compose --env-file infrastructure/compose/.env -f infrastructure/compose/docker-compose.yml up --build
```

The main API will be exposed on `http://localhost:8788`.

### Local Docker Worker Mode

To run hosted tasks inside sibling Docker worker containers:

1. Set `ORBIT_SERVER_LANE_TRANSPORT=local-docker` in `compose/.env`.
2. Build the worker image:

```bash
docker compose --env-file infrastructure/compose/.env -f infrastructure/compose/docker-compose.yml --profile worker-image build orbit-worker-image
```

3. Start the main stack:

```bash
docker compose --env-file infrastructure/compose/.env -f infrastructure/compose/docker-compose.yml up --build
```

In this mode:

- `orbit-server` uses `/var/run/docker.sock` to launch worker containers
- workers default to `ORBIT_SERVER_DOCKER_IMAGE=orbit-worker:latest`
- workers call back to the server through `ORBIT_SERVER_CALLBACK_URL`
- per-task repo checkouts live under `ORBIT_SERVER_WORKSPACE_ROOT`

## Kubernetes

The base manifests intentionally keep image names simple:

- `orbit-server:latest`
- `orbit-slack:latest`

Build and push those images to your registry, then update the image references before applying.

Apply the base manifests with:

```bash
kubectl apply -k infrastructure/kubernetes/base
```

Before applying:

1. Copy `kubernetes/base/secrets.example.yaml` to a private manifest and replace placeholder values.
2. Update the ingress host in `kubernetes/base/ingress.yaml`.
3. Adjust storage sizes and storage classes in `kubernetes/base/persistentvolumeclaims.yaml`.

## Notes

- The Slack connector does not expose a real HTTP `/health` endpoint in the current codebase, so liveness/readiness checks use a process check instead of an HTTP probe.
- If you plan to run hosted agents against actual repositories, mount or provision those repositories under `/workspace`.
- The current hosted stack does not require Postgres or Redis for basic operation.
- If you later add managed databases, queues, or object storage, extend from this folder rather than the older root-level compose examples, which are out of sync with the current hosted server and Slack surfaces.
