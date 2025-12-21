# PayX

A transaction service in Rust with double-entry bookkeeping.

## Features

- API key auth (Argon2)
- Atomic transfers with ledger entries
- Webhooks (outbox pattern)
- Idempotency
- Rate limiting
- OpenTelemetry

## Source Code

```
crates/
├── payx-server/src/
│   ├── main.rs           # Entry point
│   ├── lib.rs            # App init
│   ├── config.rs         # Env config
│   ├── error.rs          # Error types
│   ├── telemetry.rs      # OpenTelemetry
│   ├── api/
│   │   ├── routes.rs     # Routes
│   │   ├── middleware/   # Auth, rate limiting
│   │   └── handlers/     # Request handlers
│   ├── domain/           # Business entities
│   └── workers/          # Webhook processor
└── payx-cli/src/
    ├── main.rs           # CLI entry
    ├── client.rs         # HTTP client
    ├── config.rs         # CLI config
    └── commands/         # Subcommands
```
