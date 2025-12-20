# PayX

A production-grade transaction service built in Rust.

## Features

- **API Authentication** - Secure API key authentication with Argon2 hashing
- **Account Management** - Create accounts for businesses, check balances
- **Transactions** - Credit, debit, and transfer operations with atomic balance updates
- **Double-Entry Bookkeeping** - Every transaction creates balanced ledger entries
- **Webhooks** - Reliable delivery with transactional outbox pattern
- **Idempotency** - Prevent duplicate transactions with idempotency keys
- **Rate Limiting** - Per-API-key rate limiting
- **Observability** - OpenTelemetry integration for traces and structured logging

## Tech Stack

| Component | Technology |
|-----------|------------|
| Framework | Axum |
| Database | PostgreSQL |
| Auth | Argon2 |
| Observability | OpenTelemetry |
| Containerization | Docker |

## Source Code

```
src/
├── main.rs              # Entry point with graceful shutdown
├── lib.rs               # App initialization
├── config.rs            # Environment configuration
├── error.rs             # Error types
├── telemetry.rs         # OpenTelemetry setup
├── api/
│   ├── routes.rs        # Route definitions
│   ├── middleware/      # Auth, rate limiting
│   └── handlers/        # Request handlers
├── domain/              # Business entities
└── workers/             # Background processors
```
