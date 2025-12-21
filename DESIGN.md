# PayX Design

## Assumptions

**Business:**
- Single-tenant per API key
- Single currency per account
- Synchronous transactions (no sagas)
- API is the trust boundary

**Technical:**
- PostgreSQL only (no Redis/queue)
- Single region
- Thousands of TPS (not millions)
- Docker/Kubernetes deployment

---

## API Design

**Endpoints:**
```
/v1/businesses          # Public creation
/v1/accounts            # Scoped to business via API key
/v1/transactions        # Transfers/credits/debits
/v1/webhooks/endpoints  # Webhook config
/v1/webhooks/deliveries # Delivery status
```

**Idempotency:**
- Header: `Idempotency-Key: <uuid>`
- Scoped to API key, expires in 24h
- Same key + different payload = 409 Conflict

**Pagination:**
- Offset-based for lists: `?limit=50&offset=100`
- Cursor-based for history: `?limit=50&cursor=<id>`

---

## Database Schema

```
businesses       → id, name, email, webhook_url, webhook_secret
api_keys         → id, business_id, key_hash (argon2), key_prefix
accounts         → id, business_id, currency, balance, version
transactions     → id, type, status, source/dest accounts, amount, idempotency_key
ledger_entries   → id, transaction_id, account_id, entry_type, amount
webhook_outbox   → id, business_id, event_type, payload, status, attempts
```

**Key decisions:**
- `DECIMAL(19,4)` for money (no float precision issues)
- Optimistic locking via `version` column
- Hard deletes (ledger provides audit trail)
- JSONB for extensible metadata

---

## Transaction Processing

**Double-entry bookkeeping:** Every transfer creates balanced ledger entries (debits = credits).

**Flow:**
1. Validate → Check idempotency → Begin TX
2. Lock accounts (`SELECT FOR UPDATE`, sorted by ID to prevent deadlocks)
3. Validate balance → Update balances
4. Insert transaction + ledger entries + webhook outbox
5. Commit → Return response

**Failure handling:**
- Before commit: Rollback, client retries
- After commit, before response: Idempotency prevents duplicate
- Webhook failure: Outbox ensures delivery

---

## Webhook System

**Transactional outbox pattern:** Webhook entry inserted atomically with transaction. Background processor polls and delivers.

**Delivery:** At-least-once. Clients must dedupe via event ID.

**Retries:** Exponential backoff (2s, 4s, 8s, 16s). Max 5 attempts.

**Signature:** `X-Webhook-Signature: sha256=<HMAC-SHA256(payload, secret)>`

---

## Security Model

**API Keys:**
- Format: `payx_<32-byte-random-base64url>`
- Storage: Argon2id hash + prefix for lookup
- Shown once at creation

**Auth flow:** Extract Bearer → lookup by prefix → verify hash → load business

**Rate limiting:** Fixed window (100 req/min per key). Stored in PostgreSQL.

---

## Operations

**Health:** `/health` (liveness), `/ready` (DB connected)

**Shutdown:** Stop connections → drain requests (30s) → stop webhook processor → flush telemetry

**Observability:** JSON logs with request ID, OpenTelemetry tracing, OTLP export

**DB pool:** 20 connections, 3s acquire timeout

---

## Trade-offs

| Decision | Why | Downside |
|----------|-----|----------|
| PostgreSQL only | Simplicity, ACID | No horizontal scaling |
| Sync transactions | Immediate consistency | Higher latency under load |
| Fixed window rate limit | Simple | 2x burst at boundary |
| Polling for webhooks | No extra infra | 1s delay |
| Per-request auth | Stateless, instant revoke | DB query per request |

---