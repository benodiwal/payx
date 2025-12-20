# PayX Design Specification

This document outlines the core design decisions, assumptions, and trade-offs made in building PayX.

## Table of Contents

- [Assumptions](#assumptions)
- [API Design](#api-design)
- [Database Schema](#database-schema)
- [Transaction Processing](#transaction-processing)
- [Webhook System](#webhook-system)
- [Security Model](#security-model)
- [Operational Considerations](#operational-considerations)
- [Trade-offs](#trade-offs)
- [Future Considerations](#future-considerations)

---

## Assumptions

### Business Context

1. **Single-tenant per API key**: Each API key belongs to exactly one business. Cross-business operations are not supported.

2. **Single currency per account**: Accounts hold a single currency. Multi-currency accounts or automatic conversion are out of scope.

3. **Immediate consistency**: All transactions are processed synchronously. Eventually consistent models (saga patterns) are not used.

4. **Trust boundary**: The API is the trust boundary. All validation happens at the API layer; internal services trust each other.

### Technical Context

1. **PostgreSQL as single datastore**: No separate cache layer, message queue, or search engine. PostgreSQL handles all persistence needs.

2. **Single region deployment**: No geo-distribution or multi-region considerations. All components run in a single datacenter/region.

3. **Moderate scale**: Designed for thousands of transactions per second, not millions. Horizontal scaling is not a primary concern.

4. **Containerized deployment**: Assumes Docker/Kubernetes deployment. No bare-metal or serverless considerations.

---

## API Design

### Design Principles

1. **RESTful with pragmatism**: Standard REST conventions with practical exceptions where they improve usability.

2. **Consistent response format**: All responses follow the same structure for success and error cases.

3. **Explicit over implicit**: Required fields are explicit. No magic defaults that could cause unexpected behavior.

### Resource Hierarchy

```
/v1/businesses          # Top-level resource (public creation)
/v1/accounts            # Belong to a business (via API key)
/v1/transactions        # Operate on accounts
/v1/webhooks/endpoints  # Webhook configuration
/v1/webhooks/deliveries # Webhook delivery status
```

### Versioning Strategy

- URL-based versioning (`/v1/`)
- Breaking changes require new version
- Old versions supported for minimum 12 months (recommendation)

### Idempotency Design

```
Idempotency-Key: <client-generated-uuid>
```

**Behavior:**
- Keys are scoped to the API key (business)
- Keys expire after 24 hours
- Matching key + identical request = return cached response
- Matching key + different request = return 409 Conflict

**Storage:**
- Idempotency keys stored in `transactions.idempotency_key`
- No separate idempotency store needed due to tight coupling with transactions

### Pagination Strategy

Two pagination approaches based on use case:

1. **Offset-based** (for list endpoints):
   ```
   GET /v1/accounts?limit=50&offset=100
   ```
   - Simple, allows jumping to specific pages
   - Performance degrades with large offsets

2. **Cursor-based** (for transaction history):
   ```
   GET /v1/accounts/{id}/transactions?limit=50&cursor=<last_id>
   ```
   - Consistent performance regardless of depth
   - Better for real-time data that changes frequently

---

## Database Schema

### Core Tables

```
businesses
├── id (PK)
├── name
├── email (unique)
├── webhook_url
├── webhook_secret
└── timestamps

api_keys
├── id (PK)
├── business_id (FK)
├── key_hash (argon2)
├── key_prefix (for identification)
└── timestamps

accounts
├── id (PK)
├── business_id (FK)
├── account_type
├── currency
├── balance (DECIMAL)
├── available_balance (DECIMAL)
├── version (optimistic locking)
└── timestamps

transactions
├── id (PK)
├── tx_type (credit/debit/transfer)
├── status
├── source_account_id (FK, nullable)
├── destination_account_id (FK, nullable)
├── amount (DECIMAL)
├── currency
├── idempotency_key (unique per business)
├── metadata (JSONB)
└── timestamps

ledger_entries
├── id (PK)
├── transaction_id (FK)
├── account_id (FK)
├── entry_type (debit/credit)
├── amount (DECIMAL)
└── created_at

webhook_outbox
├── id (PK)
├── business_id (FK)
├── event_type
├── payload (JSONB)
├── status
├── attempts
├── next_attempt_at
└── timestamps
```

### Key Design Decisions

#### 1. DECIMAL for Money

```sql
balance DECIMAL(19,4) NOT NULL DEFAULT 0
```

- 19 digits total, 4 decimal places
- Supports values up to 999,999,999,999,999.9999
- No floating-point precision issues
- Sufficient for most currencies (including crypto with high precision)

#### 2. Optimistic Locking

```sql
version INTEGER NOT NULL DEFAULT 0
```

- Prevents lost updates in concurrent scenarios
- UPDATE includes `WHERE version = expected_version`
- Application retries on version mismatch

#### 3. Soft Deletes Not Used

- Hard deletes with audit trail in ledger_entries
- Simplifies queries and indexing
- Regulatory compliance via ledger, not soft deletes

#### 4. JSONB for Extensibility

```sql
metadata JSONB
payload JSONB
```

- Flexible schema for client-specific data
- Supports indexing for query performance
- No separate key-value store needed

### Indexing Strategy

```sql
-- Primary access patterns
CREATE INDEX idx_accounts_business ON accounts(business_id);
CREATE INDEX idx_transactions_accounts ON transactions(source_account_id, destination_account_id);
CREATE INDEX idx_transactions_idempotency ON transactions(idempotency_key) WHERE idempotency_key IS NOT NULL;

-- Webhook processing
CREATE INDEX idx_webhook_outbox_pending ON webhook_outbox(status, next_attempt_at)
  WHERE status IN ('pending', 'retrying');
```

---

## Transaction Processing

### Double-Entry Bookkeeping

Every transaction creates balanced ledger entries:

```
Transfer $100 from A to B:

ledger_entries:
| account | type   | amount |
|---------|--------|--------|
| A       | debit  | 100.00 |
| B       | credit | 100.00 |

Invariant: SUM(credits) = SUM(debits) for every transaction
```

### Transaction Flow

```
1. Validate request
2. Check idempotency key
3. Begin database transaction
4. Lock accounts (SELECT FOR UPDATE, ordered by ID)
5. Validate balances
6. Update account balances
7. Insert transaction record
8. Insert ledger entries
9. Insert webhook outbox entry
10. Commit transaction
11. Return response
```

### Concurrency Control

**Problem:** Two concurrent transfers from the same account could overdraw.

**Solution:**
1. `SELECT ... FOR UPDATE` locks the account rows
2. Accounts locked in consistent order (by UUID) to prevent deadlocks
3. Balance check happens after lock acquisition

```rust
// Lock ordering prevents deadlocks
let mut account_ids = vec![source_id, dest_id];
account_ids.sort();
for id in account_ids {
    sqlx::query("SELECT * FROM accounts WHERE id = $1 FOR UPDATE")
        .bind(id)
        .fetch_one(&mut tx)
        .await?;
}
```

### Failure Modes

| Failure Point | Behavior |
|---------------|----------|
| Before commit | Full rollback, client can retry |
| After commit, before response | Idempotency key prevents duplicate |
| Webhook delivery | Outbox pattern ensures delivery |

---

## Webhook System

### Transactional Outbox Pattern

```
┌─────────────────────────────────────────────────────┐
│                 Database Transaction                 │
│                                                     │
│  1. Update account balances                         │
│  2. Insert transaction                              │
│  3. Insert ledger entries                           │
│  4. Insert webhook_outbox entry  ◄── Atomic        │
│                                                     │
└─────────────────────────────────────────────────────┘
                         │
                         ▼
┌─────────────────────────────────────────────────────┐
│              Webhook Processor (async)              │
│                                                     │
│  1. Poll pending webhooks                           │
│  2. Deliver to endpoint                             │
│  3. Mark delivered or schedule retry                │
│                                                     │
└─────────────────────────────────────────────────────┘
```

**Why Outbox Pattern?**
- Guarantees webhook is created if transaction succeeds
- No distributed transaction needed
- Survives process crashes

### Delivery Semantics

**At-least-once delivery:**
- Webhooks may be delivered multiple times
- Clients must handle duplicates (use event ID)
- Trade-off: Simplicity over exactly-once

### Retry Strategy

```
Attempt 1: Immediate
Attempt 2: ~2 seconds
Attempt 3: ~4 seconds
Attempt 4: ~8 seconds
Attempt 5: ~16 seconds
After 5 failures: Marked as failed
```

Exponential backoff with jitter:
```rust
let delay_secs = 2i64.pow(attempt as u32).min(3600);
let jitter = rand::random::<i64>() % 1000;
```

### Signature Scheme

```
X-Webhook-Signature: sha256=<hex(HMAC-SHA256(payload, secret))>
```

- HMAC-SHA256 for integrity and authenticity
- Secret generated per-business (32 bytes, base64url)
- Constant-time comparison to prevent timing attacks

---

## Security Model

### API Key Design

```
Format: payx_<32-byte-random-base64url>
Example: payx_LzxFulvxFkYEMXe6g_BtqtFRH-nKjHPgN5l1ZG3YXeU
```

**Storage:**
- Only hash stored (Argon2id)
- Prefix stored for identification (`payx_LzxFulvx`)
- Original key shown once at creation

**Why Argon2?**
- Memory-hard (resistant to GPU attacks)
- Current best practice for password/key hashing
- Configurable cost parameters

### Authentication Flow

```
1. Extract Bearer token from Authorization header
2. Extract prefix (first 12 chars after payx_)
3. Query api_keys by prefix
4. Verify full key against stored hash
5. Load associated business context
```

### Rate Limiting

**Algorithm:** Fixed window counter

```
Window: 1 minute
Default: 100 requests per window per API key
```

**Storage:** PostgreSQL table (no Redis dependency)

```sql
CREATE TABLE rate_limit_windows (
    api_key_id UUID,
    window_start TIMESTAMP,
    request_count INTEGER,
    PRIMARY KEY (api_key_id, window_start)
);
```

**Trade-off:** Fixed window allows 2x burst at window boundary. Acceptable for simplicity.

---

## Operational Considerations

### Health Checks

| Endpoint | Purpose | Checks |
|----------|---------|--------|
| `GET /health` | Liveness | Process is running |
| `GET /ready` | Readiness | Database connected |

### Graceful Shutdown

```
SIGTERM received
    │
    ▼
Stop accepting new connections
    │
    ▼
Wait for in-flight requests (30s timeout)
    │
    ▼
Stop webhook processor
    │
    ▼
Flush telemetry
    │
    ▼
Exit
```

### Observability

**Logging:**
- JSON format for machine parsing
- Request ID in every log entry
- Structured fields for filtering

**Tracing:**
- OpenTelemetry with OTLP export
- Trace context propagation
- Span per database query

**Metrics (recommended additions):**
- `transactions_total{type, status}`
- `http_request_duration_seconds{endpoint, method, status}`
- `webhook_delivery_duration_seconds{status}`
- `db_pool_connections{state}`

### Database Connection Pool

```rust
PgPoolOptions::new()
    .max_connections(20)      // Match expected concurrency
    .acquire_timeout(Duration::from_secs(3))
    .idle_timeout(Duration::from_secs(600))
```

### Backup Strategy (Recommended)

- **WAL archiving** for point-in-time recovery
- **Daily full backups** with 30-day retention
- **Transaction log backups** every 5 minutes
- **Test restores** monthly

---

## Trade-offs

### 1. PostgreSQL Only vs. Specialized Stores

**Chose:** Single PostgreSQL database

**Pros:**
- Operational simplicity
- ACID transactions across all data
- No distributed system complexity
- Easier debugging and recovery

**Cons:**
- No horizontal read scaling
- Rate limiting less efficient than Redis
- Full-text search limited

**Mitigation:** Read replicas can be added for reporting without changing application code.

---

### 2. Synchronous vs. Asynchronous Transactions

**Chose:** Synchronous processing

**Pros:**
- Immediate consistency
- Simpler error handling
- Client gets definitive result

**Cons:**
- Higher latency under load
- Database connection held during processing
- No natural backpressure

**Mitigation:** Connection pooling and timeouts prevent resource exhaustion.

---

### 3. Fixed Window vs. Sliding Window Rate Limiting

**Chose:** Fixed window

**Pros:**
- Simple implementation
- Efficient storage (one row per window)
- Easy to reason about

**Cons:**
- Allows 2x burst at window boundaries
- Less smooth rate enforcement

**Mitigation:** Acceptable for API protection. Can upgrade to sliding window if needed.

---

### 4. Polling vs. Event-Driven Webhook Processing

**Chose:** Polling (1-second intervals)

**Pros:**
- No additional infrastructure (no message queue)
- Simple recovery (just restart)
- Natural batching

**Cons:**
- Up to 1-second delay for webhooks
- Constant database queries (even when idle)
- Less efficient at high volume

**Mitigation:** Polling interval is configurable. Could add LISTEN/NOTIFY for instant processing.

---

### 5. Per-Request Auth vs. Session/JWT

**Chose:** Per-request API key validation

**Pros:**
- Stateless servers
- Immediate key revocation
- Simple implementation

**Cons:**
- Database query per request
- Argon2 verification is CPU-intensive

**Mitigation:**
- Key prefix lookup reduces hash comparisons
- Could add short-lived cache for hot keys

---

## Future Considerations

### Not Implemented (By Design)

1. **Multi-currency transfers**: Would require exchange rates, fees, more complex ledger
2. **Scheduled transactions**: Would need job scheduler, different consistency model
3. **Account hierarchies**: Would complicate authorization model
4. **Partial failures**: All-or-nothing is simpler and safer for financial operations

### Potential Enhancements

1. **Read replicas**: Route read-only queries to replicas
2. **Caching layer**: Redis for rate limiting and session cache
3. **Event streaming**: Kafka/NATS for real-time integrations
4. **Audit log**: Immutable append-only log of all operations
5. **Multi-region**: CockroachDB or Spanner for global distribution

---

## References

- [Designing Data-Intensive Applications](https://dataintensive.net/) - Martin Kleppmann
- [Transactional Outbox Pattern](https://microservices.io/patterns/data/transactional-outbox.html)
- [Stripe API Design](https://stripe.com/docs/api)
- [PostgreSQL Documentation](https://www.postgresql.org/docs/)
