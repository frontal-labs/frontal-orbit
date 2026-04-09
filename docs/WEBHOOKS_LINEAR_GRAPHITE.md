# Linear & Graphite Webhooks

These endpoints let Orbit correlate hosted tasks with Linear issues and Graphite stacks, and now post status updates back to those systems when tasks change state.

## Endpoints
- `POST /v1/webhooks/linear`
- `POST /v1/webhooks/graphite`

## Authentication
- Enable HMAC verification by setting env vars on the server:
  - `ORBIT_LINEAR_WEBHOOK_SECRET`
  - `ORBIT_GRAPHITE_WEBHOOK_SECRET`
- Clients must send `sha256=<hex>` of the raw request body in:
  - Linear: header `x-linear-signature`
  - Graphite: header `x-graphite-signature`

## Linking fields
- Linear: `issue.id` or `issue_id`, `issue.identifier` or `issue_identifier`, optional `issue.url`, `issue.state`, optional `task_id` to force binding.
- Graphite: `stack_id` (or `stack.id`), optional `head_branch`/`base_branch`, optional `task_id` to force binding.

## Outbound status comments
If the server has tokens set, Orbit will post comments when a task completes, fails, or requests approval:
- `ORBIT_LINEAR_API_TOKEN` (optional `ORBIT_LINEAR_API_URL`, defaults to `https://api.linear.app/graphql`)
- `ORBIT_GRAPHITE_API_TOKEN` (optional `ORBIT_GRAPHITE_API_URL`, defaults to `https://graphite.dev/api`)

## Sample payloads
See the JSON fixtures in `examples/webhooks/`:
- `examples/webhooks/linear-issue-updated.json`
- `examples/webhooks/graphite-stack-updated.json`

## Example curl
```bash
curl -X POST http://localhost:8788/v1/webhooks/linear \
  -H "Content-Type: application/json" \
  -H "x-linear-signature: sha256=<hex_hmac_of_body>" \
  -d @examples/webhooks/linear-issue-updated.json
```

```bash
curl -X POST http://localhost:8788/v1/webhooks/graphite \
  -H "Content-Type: application/json" \
  -H "x-graphite-signature: sha256=<hex_hmac_of_body>" \
  -d @examples/webhooks/graphite-stack-updated.json
```
