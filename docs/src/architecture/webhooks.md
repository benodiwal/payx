# Webhook Delivery

## Transactional Outbox Pattern

Webhooks are never lost because they're written to the database in the same transaction as the business logic.

### Flow

```
┌─────────────────────────────────────────────────────────┐
│                    HTTP Request                         │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│              Transaction Handler                        |
│  ┌───────────────────────────────────────────────────┐  │
│  │              DB Transaction                       │  │
│  │  1. Update account balances                       │  │
│  │  2. Insert transaction record                     │  │
│  │  3. Insert webhook_outbox event   ◄── Same TX     │  │
│  │  4. Commit                                        │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
                          │
                          ▼
┌─────────────────────────────────────────────────────────┐
│            Background Webhook Processor                 │
│  ┌───────────────────────────────────────────────────┐  │
│  │  Loop:                                            │  │
│  │  1. Poll webhook_outbox for pending events        │  │
│  │  2. Deliver to webhook URL                        │  │
│  │  3. Mark delivered or schedule retry              │  │
│  └───────────────────────────────────────────────────┘  │
└─────────────────────────────────────────────────────────┘
```

### Why Transactional Outbox?

| Approach | Problem |
|----------|---------|
| Send webhook in request | Webhook lost if send fails after commit |
| Send before commit | Webhook sent for rolled-back transaction |
| Outbox pattern | Atomic: webhook record committed with transaction |

## Delivery Process

### Polling Query

```sql
SELECT * FROM webhook_outbox
WHERE status IN ('pending', 'retrying')
AND next_attempt_at <= NOW()
ORDER BY created_at
LIMIT 100
FOR UPDATE SKIP LOCKED
```

`FOR UPDATE SKIP LOCKED` ensures:
- No duplicate delivery attempts
- Non-blocking when multiple workers

### Delivery Request

```
POST {webhook_url}
Content-Type: application/json
X-Webhook-Id: {event_id}
X-Webhook-Timestamp: {unix_timestamp}
X-Webhook-Signature: sha256={hmac_signature}

{payload}
```

## Retry Policy

| Attempt | Delay | Cumulative |
|---------|-------|------------|
| 1 | 0s | 0s |
| 2 | ~2s | ~2s |
| 3 | ~4s | ~6s |
| 4 | ~8s | ~14s |
| 5 | ~16s | ~30s |

### Backoff Formula

```rust
fn next_retry_delay(attempt: u32) -> Duration {
    let base = Duration::from_secs(2u64.pow(attempt.min(5)));
    let jitter = Duration::from_millis(rand::random::<u64>() % 1000);
    base + jitter
}
```

Jitter prevents thundering herd when many webhooks fail simultaneously.

## Signature Verification

### Signing

```rust
fn sign_payload(payload: &[u8], secret: &str) -> String {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .expect("valid key");
    mac.update(payload);
    format!("sha256={}", hex::encode(mac.finalize().into_bytes()))
}
```

### Client Verification

1. Extract signature from `X-Webhook-Signature` header
2. Compute HMAC-SHA256 of raw request body with secret
3. Compare signatures using constant-time comparison

## Status Transitions

```
pending → delivered       (success)
pending → retrying        (failure, attempts < max)
retrying → delivered      (success)
retrying → retrying       (failure, attempts < max)
retrying → failed         (attempts >= max)
```

## Guarantees

| Guarantee | Description |
|-----------|-------------|
| At-least-once | Events delivered at least once (may duplicate) |
| Ordering | Not guaranteed between events |
| Delivery window | ~30 seconds of retries |
