# Architecture Overview

## System Design

```
┌────────────────────────────────────────────────────────────────┐
│                          API Layer                             │
│                                                                │
│   ┌──────────┐   ┌──────────┐   ┌──────────┐   ┌──────────┐    │
│   │ Auth MW  │ → │ Rate Lim │ → │ Handlers │ → │ Response │    │
│   └──────────┘   └──────────┘   └──────────┘   └──────────┘    │
│                                                                │
└────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌────────────────────────────────────────────────────────────────┐
│                        Domain Layer                            │
│                                                                │
│   ┌──────────┐   ┌─────────────┐   ┌─────────┐   ┌─────────┐   │
│   │ Business │   │ Transaction │   │ Account │   │ Webhook │   │
│   └──────────┘   └─────────────┘   └─────────┘   └─────────┘   │
│                                                                │
└────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌────────────────────────────────────────────────────────────────┐
│                     PostgreSQL Database                        │
│                                                                │
│   ┌────────────┐   ┌──────────────┐   ┌────────────────────┐   │
│   │  Accounts  │   │ Transactions │   │   Webhook Outbox   │   │
│   │            │   │   + Ledger   │   │ (Transactional OB) │   │
│   └────────────┘   └──────────────┘   └────────────────────┘   │
│                                                                │
└────────────────────────────────────────────────────────────────┘
                                │
                                ▼
┌────────────────────────────────────────────────────────────────┐
│                     Background Workers                         │
│                                                                │
│   ┌────────────────────────────────────────────────────────┐   │
│   │                  Webhook Processor                     │   │
│   │      (Polls outbox → delivers → retries with backoff)  │   │
│   └────────────────────────────────────────────────────────┘   │
│                                                                │
└────────────────────────────────────────────────────────────────┘
```

## Design Principles

### 1. Correctness First

Financial systems require absolute correctness. Key decisions:

- **DECIMAL for money**: Never use floating point
- **Atomic operations**: All balance updates in database transactions
- **Double-entry bookkeeping**: Every transaction balances

### 2. Reliability

- **Transactional outbox**: Webhooks never lost
- **Idempotency**: Safe retries for all mutations
- **Graceful shutdown**: Complete in-flight requests

### 3. Security

- **Argon2 hashing**: API keys never stored in plaintext
- **HMAC signatures**: Webhook payload verification
- **Rate limiting**: Protection against abuse

### 4. Observability

- **Structured logging**: JSON format for parsing
- **Distributed tracing**: OpenTelemetry support
- **Health checks**: Liveness and readiness probes

## Key Trade-offs

| Decision | Trade-off |
|----------|-----------|
| PostgreSQL only | Simpler ops, but no horizontal scaling |
| Sync transactions | Lower latency, but blocking during high load |
| Fixed-window rate limiting | Simpler, but allows 2x burst at window boundary |
| Background webhook processor | Decoupled delivery, but slight delay |
