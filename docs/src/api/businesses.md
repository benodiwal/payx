# Businesses

## List Businesses

```
GET /v1/businesses
```

### Query Parameters

| Parameter | Type | Default | Description |
|-----------|------|---------|-------------|
| `limit` | integer | 50 | Max businesses to return (1-100) |
| `offset` | integer | 0 | Number of records to skip |

### Response `200 OK`

```json
[
  {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "Acme Corp",
    "email": "admin@acme.com",
    "webhook_url": "https://example.com/webhooks",
    "created_at": "2024-12-17T10:00:00Z",
    "updated_at": "2024-12-17T10:00:00Z"
  }
]
```

---

## Create Business

Creates a new business and returns an API key.

```
POST /v1/businesses
```

### Request

```json
{
  "name": "Acme Corp",
  "email": "admin@acme.com",
  "webhook_url": "https://example.com/webhooks"
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `name` | string | Yes | Business name |
| `email` | string | Yes | Unique email address |
| `webhook_url` | string | No | URL for webhook delivery |

### Response `201 Created`

```json
{
  "business": {
    "id": "550e8400-e29b-41d4-a716-446655440000",
    "name": "Acme Corp",
    "email": "admin@acme.com",
    "webhook_url": "https://example.com/webhooks",
    "created_at": "2024-12-17T10:00:00Z",
    "updated_at": "2024-12-17T10:00:00Z"
  },
  "api_key": {
    "id": "550e8400-e29b-41d4-a716-446655440001",
    "key": "payx_abc123...",
    "prefix": "payx_abc123"
  },
  "webhook_secret": "whsec_..."
}
```

> **Important**: Store `api_key.key` and `webhook_secret` securely. They cannot be retrieved later.

---

## Get Business

```
GET /v1/businesses/{id}
```

### Response `200 OK`

```json
{
  "id": "550e8400-e29b-41d4-a716-446655440000",
  "name": "Acme Corp",
  "email": "admin@acme.com",
  "webhook_url": "https://example.com/webhooks",
  "created_at": "2024-12-17T10:00:00Z",
  "updated_at": "2024-12-17T10:00:00Z"
}
```

---

## Update Business

```
PUT /v1/businesses/{id}
```

### Request

```json
{
  "name": "Acme Corporation",
  "webhook_url": "https://example.com/webhooks/v2"
}
```

All fields are optional. Only provided fields will be updated.

### Response `200 OK`

Returns the updated business object.
