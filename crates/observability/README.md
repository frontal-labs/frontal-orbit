# Orbit Observability

Structured observability primitives for Orbit services and agent runtimes.

## Scope

This crate focuses on two operational concerns:

- Sentry-style error reporting configuration and event capture
- Structured AI agent run/span/event observability

It also includes an adapter for forwarding agent observations into
`orbit-telemetry::SessionTracer`, which lets the workspace keep its current
session trace pipeline while adopting richer agent-level APIs.
