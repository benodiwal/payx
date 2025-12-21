# Webhook Delivery

## Transactional Outbox

Webhook entry inserted in same DB transaction as balance updates. Background processor polls and delivers.

**Polling:** `SELECT ... FOR UPDATE SKIP LOCKED` prevents duplicate deliveries.

## Delivery

```
POST {webhook_url}
X-Webhook-Id: {event_id}
X-Webhook-Timestamp: {unix_timestamp}
X-Webhook-Signature: sha256={hmac_signature}
```

## Retries

Exponential backoff with jitter: 0s → 2s → 4s → 8s → 16s. Max 5 attempts.

## Signature

HMAC-SHA256 of payload with business webhook secret. Clients should use constant-time comparison.

## Guarantees

- At-least-once delivery (clients must dedupe)
- No ordering guarantees
- ~30s retry window
