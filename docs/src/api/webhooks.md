# Webhooks

## Configure Webhook Endpoint

```
POST /v1/webhooks/endpoints
```

### Request

```json
{
  "url": "https://your-server.com/webhooks"
}
```

### Response `201 Created`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "url": "https://your-server.com/webhooks",
  "secret": "whsec_..."
}
```

---

## Update Endpoint

```
PUT /v1/webhooks/endpoints/{id}
```

### Request

```json
{
  "url": "https://your-server.com/webhooks/v2"
}
```

---

## Delete Endpoint

```
DELETE /v1/webhooks/endpoints/{id}
```

Returns `204 No Content`.

---

## Webhook Events

### Event Format

```json
{
  "id": "evt_550e8400-e29b-41d4-a716-446655440005",
  "event_type": "transaction.completed",
  "created_at": "2024-12-17T10:00:00Z",
  "data": {
    "id": "550e8400-e29b-41d4-a716-446655440004",
    "type": "transfer",
    "status": "completed",
    "amount": "25.0000",
    "currency": "USD"
  }
}
```

### Event Types

| Event | Description |
|-------|-------------|
| `transaction.completed` | Transaction successfully processed |

---

## Webhook Headers

| Header | Description |
|--------|-------------|
| `X-Webhook-Id` | Unique event ID |
| `X-Webhook-Timestamp` | Unix timestamp |
| `X-Webhook-Signature` | HMAC-SHA256 signature |

---

## Signature Verification

Verify webhook authenticity by checking the signature.

### Python

```python
import hmac
import hashlib

def verify_webhook(payload: bytes, secret: str, signature: str) -> bool:
    expected = "sha256=" + hmac.new(
        secret.encode(),
        payload,
        hashlib.sha256
    ).hexdigest()
    return hmac.compare_digest(expected, signature)
```

### Node.js

```javascript
const crypto = require('crypto');

function verifyWebhook(payload, secret, signature) {
  const expected = 'sha256=' + crypto
    .createHmac('sha256', secret)
    .update(payload)
    .digest('hex');
  return crypto.timingSafeEqual(
    Buffer.from(expected),
    Buffer.from(signature)
  );
}
```

### Rust

```rust
use hmac::{Hmac, Mac};
use sha2::Sha256;

fn verify_webhook(payload: &[u8], secret: &str, signature: &str) -> bool {
    let mut mac = Hmac::<Sha256>::new_from_slice(secret.as_bytes())
        .expect("valid key");
    mac.update(payload);
    let expected = format!("sha256={}", hex::encode(mac.finalize().into_bytes()));
    expected == signature
}
```

---

## Retry Policy

| Attempt | Delay |
|---------|-------|
| 1 | Immediate |
| 2 | ~2 seconds |
| 3 | ~4 seconds |
| 4 | ~8 seconds |
| 5 | ~16 seconds |

After 5 failed attempts, the webhook is marked as failed.

---

## Webhook Delivery Management

### List Deliveries

```
GET /v1/webhooks/deliveries
```

#### Query Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `limit` | integer | 50 | Max deliveries to return (1-100) |
| `offset` | integer | 0 | Number of records to skip |
| `status` | string | - | Filter by status: `pending`, `retrying`, `delivered`, `failed` |

#### Response `200 OK`

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440010",
    "event_type": "transaction.completed",
    "status": "delivered",
    "attempts": 1,
    "max_attempts": 5,
    "last_error": null,
    "created_at": "2024-12-17T10:00:00Z",
    "processed_at": "2024-12-17T10:00:01Z",
    "next_attempt_at": "2024-12-17T10:00:00Z"
  }
]
```

---

### Get Delivery

```
GET /v1/webhooks/deliveries/{id}
```

#### Response `200 OK`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440010",
  "event_type": "transaction.completed",
  "status": "failed",
  "attempts": 5,
  "max_attempts": 5,
  "last_error": "connection refused",
  "created_at": "2024-12-17T10:00:00Z",
  "processed_at": null,
  "next_attempt_at": "2024-12-17T10:01:00Z"
}
```

---

### Retry Failed Delivery

Requeue a failed webhook delivery for retry.

```
POST /v1/webhooks/deliveries/{id}/retry
```

#### Response `200 OK`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440010",
  "event_type": "transaction.completed",
  "status": "pending",
  "attempts": 0,
  "max_attempts": 5,
  "last_error": null,
  "created_at": "2024-12-17T10:00:00Z",
  "processed_at": null,
  "next_attempt_at": "2024-12-17T10:05:00Z"
}
```

> **Note**: Only webhooks with status `failed` can be retried. The attempt counter is reset to 0.
