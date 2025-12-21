# Architecture Overview

**Layers:** API (auth → rate limit → handlers) → Domain → PostgreSQL → Background workers

**Principles:**
- DECIMAL for money, atomic DB transactions, double-entry bookkeeping
- Transactional outbox for webhooks, idempotency for safe retries
- Argon2 key hashing, HMAC webhook signatures, rate limiting
- JSON logs, OpenTelemetry tracing, health endpoints

**Trade-offs:**

| Decision | Trade-off |
|----------|-----------|
| PostgreSQL only | Simpler ops, no horizontal scaling |
| Sync transactions | Immediate consistency, blocks under load |
| Fixed-window rate limit | Simple, 2x burst at boundary |
| Polling webhooks | No extra infra, slight delay |
