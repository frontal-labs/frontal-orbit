# Orbit Webhooks

Webhook receiving and processing capabilities for external service integrations.

## Overview

This crate provides comprehensive webhook handling functionality for the Orbit system, including secure webhook reception, event processing, and integration with external services through HTTP endpoints.

## Features

- HMAC-based webhook authentication and signature verification
- Configurable webhook event processing and routing
- HTTP webhook receiver with CORS support
- Tool integration for webhook execution within AI workflows
- Support for multiple webhook event types and custom handlers

## Key Components

- **WebhookReceiver**: HTTP server for receiving webhook requests
- **HmacAuthenticator**: Secure authentication using HMAC signatures
- **EventProcessor**: Processing and routing of webhook events
- **WebhookTools**: Integration tools for AI-powered webhook interactions

## Dependencies

- `axum` for HTTP server functionality
- `reqwest` for HTTP client operations
- `tokio` for async runtime
- `serde` for serialization/deserialization
- `hmac` and `sha2` for cryptographic operations
- `tower` and `tower-http` for HTTP middleware

## Current Status

This crate provides webhook infrastructure and is included in workspace build/test gates.
