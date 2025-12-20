# Accounts

## List Accounts

```
GET /v1/accounts
```

### Query Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `limit` | integer | 50 | Max accounts to return (1-100) |
| `offset` | integer | 0 | Number of records to skip |
| `business_id` | UUID | - | Filter by business ID |

### Response `200 OK`

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440002",
    "business_id": "550e8400-e29b-41d4-a716-446655440000",
    "account_type": "checking",
    "currency": "USD",
    "balance": "1000.0000",
    "available_balance": "1000.0000",
    "created_at": "2024-12-17T10:00:00Z"
  }
]
```

---

## Create Account

```
POST /v1/accounts
```

### Request

```json
{
  "business_id": "550e8400-e29b-41d4-a716-446655440000",
  "account_type": "checking",
  "currency": "USD",
  "initial_balance": "1000.00"
}
```

| Field | Type | Required | Default | Description |
|-------|------|----------|---------|-------------|
| `business_id` | UUID | Yes | - | Business that owns this account |
| `account_type` | string | No | `checking` | Account type |
| `currency` | string | No | `USD` | ISO 4217 currency code |
| `initial_balance` | string | No | `0` | Starting balance |

### Response `201 Created`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440002",
  "business_id": "550e8400-e29b-41d4-a716-446655440000",
  "account_type": "checking",
  "currency": "USD",
  "balance": "1000.0000",
  "available_balance": "1000.0000",
  "created_at": "2024-12-17T10:00:00Z"
}
```

---

## Get Account

```
GET /v1/accounts/{id}
```

### Response `200 OK`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440002",
  "business_id": "550e8400-e29b-41d4-a716-446655440000",
  "account_type": "checking",
  "currency": "USD",
  "balance": "1000.0000",
  "available_balance": "1000.0000",
  "created_at": "2024-12-17T10:00:00Z"
}
```

---

## List Account Transactions

```
GET /v1/accounts/{id}/transactions
```

### Query Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `limit` | integer | 50 | Max transactions to return (1-100) |
| `cursor` | UUID | - | Pagination cursor (last transaction ID) |

### Response `200 OK`

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440003",
    "type": "credit",
    "status": "completed",
    "source_account_id": null,
    "destination_account_id": "550e8400-e29b-41d4-a716-446655440002",
    "amount": "100.0000",
    "currency": "USD",
    "description": "Deposit",
    "created_at": "2024-12-17T10:00:00Z",
    "completed_at": "2024-12-17T10:00:00Z"
  }
]
```

### Pagination

Use cursor-based pagination for efficient traversal:

```bash
# First page
curl ".../transactions?limit=50"

# Next page (use last transaction ID as cursor)
curl ".../transactions?limit=50&cursor=<last_id>"
```
