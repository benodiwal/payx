# Transactions

## List Transactions

```
GET /v1/transactions
```

### Query Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `limit` | integer | 50 | Max transactions to return (1-100) |
| `offset` | integer | 0 | Number of records to skip |
| `account_id` | UUID | - | Filter by account ID (source or destination) |

### Response `200 OK`

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440004",
    "type": "transfer",
    "status": "completed",
    "source_account_id": "...",
    "destination_account_id": "...",
    "amount": "25.0000",
    "currency": "USD",
    "description": "Payment",
    "created_at": "2024-12-17T10:00:00Z",
    "completed_at": "2024-12-17T10:00:00Z"
  }
]
```

---

## Create Transaction

```
POST /v1/transactions
```

### Headers

| Header | Required | Description |
|--------|----------|-------------|
| `Idempotency-Key` | Recommended | Unique key to prevent duplicates |

### Transaction Types

#### Credit

Add funds to an account.

```json
{
  "type": "credit",
  "destination_account_id": "uuid",
  "amount": "100.00",
  "currency": "USD",
  "description": "Deposit",
  "metadata": {"source": "bank_transfer"}
}
```

#### Debit

Remove funds from an account.

```json
{
  "type": "debit",
  "source_account_id": "uuid",
  "amount": "50.00",
  "currency": "USD",
  "description": "Withdrawal"
}
```

#### Transfer

Move funds between accounts.

```json
{
  "type": "transfer",
  "source_account_id": "uuid",
  "destination_account_id": "uuid",
  "amount": "25.00",
  "currency": "USD",
  "description": "Payment"
}
```

### Request Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `type` | string | Yes | `credit`, `debit`, or `transfer` |
| `source_account_id` | UUID | Debit/Transfer | Account to debit |
| `destination_account_id` | UUID | Credit/Transfer | Account to credit |
| `amount` | string | Yes | Positive amount |
| `currency` | string | Yes | ISO 4217 currency code |
| `description` | string | No | Human-readable description |
| `metadata` | object | No | Custom key-value data |

### Response `201 Created`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440004",
  "type": "transfer",
  "status": "completed",
  "source_account_id": "...",
  "destination_account_id": "...",
  "amount": "25.0000",
  "currency": "USD",
  "description": "Payment",
  "created_at": "2024-12-17T10:00:00Z",
  "completed_at": "2024-12-17T10:00:00Z"
}
```

---

## Idempotency

The `Idempotency-Key` header prevents duplicate transactions.

**Behavior:**
- First request: Creates transaction, returns `201 Created`
- Duplicate request (same key): Returns existing transaction, `200 OK`

**Example:**

```bash
# First request
curl -X POST .../transactions \
  -H "Idempotency-Key: payment-001" \
  -d '{"type": "transfer", ...}'
# Response: 201 Created

# Duplicate request
curl -X POST .../transactions \
  -H "Idempotency-Key: payment-001" \
  -d '{"type": "transfer", ...}'
# Response: 200 OK (same transaction returned)
```

---

## Get Transaction

```
GET /v1/transactions/{id}
```

### Response `200 OK`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440004",
  "type": "transfer",
  "status": "completed",
  "source_account_id": "...",
  "destination_account_id": "...",
  "amount": "25.0000",
  "currency": "USD",
  "description": "Payment",
  "created_at": "2024-12-17T10:00:00Z",
  "completed_at": "2024-12-17T10:00:00Z"
}
```

---

## Common Errors

| Code | Status | Description |
|------|--------|-------------|
| `insufficient_funds` | 422 | Account balance too low |
| `account_not_found` | 404 | Account does not exist |
| `currency_mismatch` | 400 | Currency doesn't match account |
| `validation_error` | 400 | Invalid request (e.g., negative amount) |
